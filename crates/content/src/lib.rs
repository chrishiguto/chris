//! The shared content crate: the versioned AST IR plus the component
//! vocabulary types (always available, wasm-lean), and the MDX-subset parser
//! behind the `parse` feature (syn-style opt-in, so read-path consumers never
//! link markdown-rs).
//!
//! KV stores a serde-typed semantic AST — never pre-rendered
//! HTML and never raw markdown. These types are that contract: the pipeline
//! worker writes them at publish time, the site worker renders them at
//! request time, and `xtask check` validates against them locally.
//!
//! The schema is versioned via [`SCHEMA_VERSION`]. [`Document::from_json`]
//! rejects entries written under a different version so stale KV data is
//! detectable (and migratable) instead of silently misrendered.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

mod manifest;
pub use manifest::{integral, ComponentSpec, Manifest, PropSpec, PropType};

mod routes;
pub use routes::{
    index_key_at, post_key, post_key_at, post_path, post_slug, snapshot_index_key,
    snapshot_key_sha, snapshot_post_key, source_path, tag_path, CurrentPointer, CONTENT_ROOT,
    CURRENT_KEY, FEED_PATHS, INDEX_KEY, LISTING_PAGES, POST_FILE, RSS_PATH, SITEMAP_PATH,
    SNAPSHOT_KEY_SPACE,
};

#[cfg(feature = "parse")]
mod parse;
#[cfg(feature = "parse")]
pub use parse::*;

/// Version stamped into every serialized [`Document`].
///
/// Bump on any change to the shape of [`Document`], [`Frontmatter`],
/// [`Node`], or [`PropValue`] that alters the serialized form.
pub const SCHEMA_VERSION: u32 = 1;

/// Errors from (de)serializing a [`Document`].
#[derive(Debug)]
pub enum AstError {
    /// The payload was written under a different [`SCHEMA_VERSION`].
    SchemaVersionMismatch { found: u32, expected: u32 },
    /// The payload is not valid JSON for this schema.
    Json(serde_json::Error),
}

impl std::fmt::Display for AstError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AstError::SchemaVersionMismatch { found, expected } => {
                write!(
                    f,
                    "schema version mismatch: found {found}, expected {expected}"
                )
            }
            AstError::Json(err) => write!(f, "invalid document JSON: {err}"),
        }
    }
}

impl std::error::Error for AstError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AstError::Json(err) => Some(err),
            AstError::SchemaVersionMismatch { .. } => None,
        }
    }
}

/// A fully parsed post: the unit stored under `post:{slug}` in KV.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    /// Schema version this document was serialized under; see [`SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Post metadata extracted from the YAML frontmatter block.
    pub frontmatter: Frontmatter,
    /// The post body as a sequence of block-level nodes.
    pub ast: Vec<Node>,
}

impl Document {
    /// Serializes the document to JSON.
    pub fn to_json(&self) -> Result<String, AstError> {
        serde_json::to_string(self).map_err(AstError::Json)
    }

    /// Deserializes a document, rejecting payloads whose `schema_version`
    /// differs from [`SCHEMA_VERSION`].
    pub fn from_json(json: &str) -> Result<Self, AstError> {
        let doc: Document = serde_json::from_str(json).map_err(AstError::Json)?;
        if doc.schema_version != SCHEMA_VERSION {
            return Err(AstError::SchemaVersionMismatch {
                found: doc.schema_version,
                expected: SCHEMA_VERSION,
            });
        }
        Ok(doc)
    }
}

/// Post metadata; drives listings, tag pages, and feeds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frontmatter {
    /// Post title, shown in listings and the page `<title>`.
    pub title: String,
    /// Publication date as an ISO `YYYY-MM-DD` string; listings sort on it.
    pub date: String,
    /// One-line summary for feeds. Optional and skipped when absent, so
    /// pre-description payloads round-trip unchanged (no schema bump).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Tags for tag pages and feeds; empty when omitted.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Drafts stay reachable by slug but are filtered from listings/feeds.
    #[serde(default)]
    pub draft: bool,
}

/// One entry of the KV `index` key: the ordered post listing the site
/// renders from. Drafts are stored here but filtered out
/// of every listing/feed at render time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexEntry {
    pub slug: String,
    pub title: String,
    /// ISO `YYYY-MM-DD`; the index is ordered newest-first on this.
    pub date: String,
    /// One-line summary for feeds; skipped when absent (see [`Frontmatter`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub draft: bool,
}

impl IndexEntry {
    /// The one draft-visibility rule: a draft renders by slug but appears in
    /// no listing, feed, sitemap, or tag page. Every index consumer must
    /// filter through this, never `!draft` by hand.
    pub fn is_listed(&self) -> bool {
        !self.draft
    }

    pub fn new(slug: &str, frontmatter: &Frontmatter) -> Self {
        Self {
            slug: slug.to_string(),
            title: frontmatter.title.clone(),
            date: frontmatter.date.clone(),
            description: frontmatter.description.clone(),
            tags: frontmatter.tags.clone(),
            draft: frontmatter.draft,
        }
    }
}

/// One semantic node of the post body.
///
/// Prose maps to HTML-shaped variants; [`Node::Component`] references a
/// registered Leptos component *by name*, resolved through the registry at
/// render time — the stored content never knows how a component renders.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Node {
    /// `# Heading` through `###### Heading`; `level` is 1–6.
    Heading { level: u8, children: Vec<Node> },
    /// A paragraph of inline content.
    Paragraph { children: Vec<Node> },
    /// Literal text.
    Text { value: String },
    /// `*emphasis*`.
    Emphasis { children: Vec<Node> },
    /// `**strong**`.
    Strong { children: Vec<Node> },
    /// `` `inline code` ``.
    InlineCode { value: String },
    /// `[children](url "title")`.
    Link {
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        children: Vec<Node>,
    },
    /// `![alt](url "title")`.
    Image {
        url: String,
        alt: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },
    /// Ordered or bullet list; `start` is the first ordinal of ordered lists.
    List {
        ordered: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start: Option<u32>,
        items: Vec<ListItem>,
    },
    /// Fenced code block, stored as raw text + language only — presentation
    /// (highlighting included) is a renderer concern.
    CodeBlock {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        lang: Option<String>,
        text: String,
    },
    /// `> quoted` block content.
    Blockquote { children: Vec<Node> },
    /// `---` horizontal rule.
    ThematicBreak,
    /// Hard line break (trailing-backslash or double-space).
    Break,
    /// Lowercase-tag HTML passthrough, e.g. `<abbr title="...">`; children
    /// are markdown, parsed recursively.
    Html {
        tag: String,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        attrs: BTreeMap<String, String>,
        children: Vec<Node>,
    },
    /// PascalCase component invocation, resolved by name through the
    /// registry at render time; children are markdown, parsed recursively.
    Component {
        name: String,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        props: BTreeMap<String, PropValue>,
        children: Vec<Node>,
    },
}

/// One item of a [`Node::List`]; its children are block-level nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListItem {
    pub children: Vec<Node>,
}

/// A component prop value. Scalar literals only in v1: structured
/// data arrives as children or not at all.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PropValue {
    String(String),
    Number(f64),
    Bool(bool),
}
