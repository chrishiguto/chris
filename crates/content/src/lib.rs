//! Versioned AST IR and component vocabulary types; MDX-subset parser behind
//! the `parse` feature so read-path consumers never link markdown-rs.
//! KV stores this typed AST — never pre-rendered HTML or raw markdown.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

mod manifest;
pub use manifest::{integral, ComponentSpec, Manifest, PropSpec, PropType};

mod routes;
pub use routes::{
    index_key_at, post_key, post_key_at, post_path, post_slug, snapshot_index_key,
    snapshot_key_sha, snapshot_post_key, source_path, tag_path, valid_slug, CurrentPointer,
    CONTENT_ROOT, CURRENT_KEY, FEED_PATHS, INDEX_KEY, LISTING_PAGES, POST_FILE, RSS_PATH,
    SITEMAP_PATH, SNAPSHOT_KEY_SPACE,
};

#[cfg(feature = "parse")]
mod parse;
#[cfg(feature = "parse")]
pub use parse::*;

/// Stamped into every serialized [`Document`]; bump on any change to the
/// serialized shape.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug)]
pub enum AstError {
    SchemaVersionMismatch { found: u32, expected: u32 },
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
    pub schema_version: u32,
    pub frontmatter: Frontmatter,
    pub ast: Vec<Node>,
}

impl Document {
    pub fn to_json(&self) -> Result<String, AstError> {
        serde_json::to_string(self).map_err(AstError::Json)
    }

    /// Probes `schema_version` before the full shape so an old payload
    /// surfaces as a version mismatch, not a missing-field error.
    pub fn from_json(json: &str) -> Result<Self, AstError> {
        #[derive(Deserialize)]
        struct Probe {
            schema_version: u32,
        }
        let found = serde_json::from_str::<Probe>(json)
            .map_err(AstError::Json)?
            .schema_version;
        if found != SCHEMA_VERSION {
            return Err(AstError::SchemaVersionMismatch {
                found,
                expected: SCHEMA_VERSION,
            });
        }
        serde_json::from_str(json).map_err(AstError::Json)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frontmatter {
    pub title: String,
    /// ISO `YYYY-MM-DD`; listings sort lexicographically on it.
    pub date: String,
    /// Skipped when absent so pre-description payloads round-trip unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Drafts stay reachable by slug but are filtered from listings/feeds.
    #[serde(default)]
    pub draft: bool,
}

/// One entry of the KV `index` key: the ordered post listing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexEntry {
    pub slug: String,
    pub title: String,
    /// ISO `YYYY-MM-DD`; the index is ordered newest-first on this.
    pub date: String,
    /// Skipped when absent (see [`Frontmatter`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub draft: bool,
}

impl IndexEntry {
    /// Drafts render by slug but appear in no listing, feed, sitemap, or tag
    /// page; consumers must filter through this, never `!draft` by hand.
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

/// One semantic node of the post body. [`Node::Component`] is resolved by
/// name through the registry at render time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Node {
    /// `level` is 1–6.
    Heading {
        level: u8,
        children: Vec<Node>,
    },
    Paragraph {
        children: Vec<Node>,
    },
    Text {
        value: String,
    },
    Emphasis {
        children: Vec<Node>,
    },
    Strong {
        children: Vec<Node>,
    },
    InlineCode {
        value: String,
    },
    Link {
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        children: Vec<Node>,
    },
    Image {
        url: String,
        alt: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },
    List {
        ordered: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start: Option<u32>,
        items: Vec<ListItem>,
    },
    /// Raw text + language only; highlighting is a renderer concern.
    CodeBlock {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        lang: Option<String>,
        text: String,
    },
    Blockquote {
        children: Vec<Node>,
    },
    ThematicBreak,
    Break,
    /// Lowercase-tag HTML passthrough; children are markdown, not raw HTML.
    Html {
        tag: String,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        attrs: BTreeMap<String, String>,
        children: Vec<Node>,
    },
    /// PascalCase invocation; children are markdown, parsed recursively.
    Component {
        name: String,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        props: BTreeMap<String, PropValue>,
        children: Vec<Node>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListItem {
    pub children: Vec<Node>,
}

/// Scalar literals only; structured data goes in children.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PropValue {
    String(String),
    Number(f64),
    Bool(bool),
}
