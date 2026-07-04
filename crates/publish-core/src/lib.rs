//! The publish operation's pure core (ADR-0007): validate post sources and
//! turn them into a KV write plan. Shared by the `blog` CLI today and the
//! pipeline worker (Slice 6) — so it stays wasm-clean: no filesystem, no
//! HTTP, no clock. Callers own transport (Cloudflare API vs KV binding).

use content_ast::{AstError, Document, IndexEntry};
use content_parser::Diagnostic;
use registry::Manifest;

/// KV key of the ordered post listing (PRD "KV schema").
pub const INDEX_KEY: &str = "index";

/// One post's raw authoring input, however the caller obtained it
/// (filesystem for the CLI, GitHub contents API for the pipeline).
#[derive(Debug, Clone)]
pub struct PostSource {
    /// Directory name under `content/blog/`; the KV and URL identity.
    pub slug: String,
    /// Path stamped into diagnostics, e.g. `content/blog/{slug}/index.mdx`.
    pub file: String,
    /// Raw `.mdx` text.
    pub source: String,
}

/// A source that passed parsing and manifest validation.
#[derive(Debug, Clone)]
pub struct ParsedPost {
    pub slug: String,
    pub document: Document,
}

/// One KV put the caller must perform.
#[derive(Debug, Clone, PartialEq)]
pub struct KvWrite {
    pub key: String,
    pub value: String,
}

/// Everything a publish must do to KV, in order: puts, then deletes.
#[derive(Debug, Clone)]
pub struct PublishPlan {
    /// `post:{slug}` payloads for every changed post, then the `index` key.
    pub writes: Vec<KvWrite>,
    /// `post:{slug}` keys of removed posts.
    pub deletes: Vec<String>,
    /// The rewritten index, newest-first; what `writes` serializes last.
    pub index: Vec<IndexEntry>,
}

fn post_key(slug: &str) -> String {
    format!("post:{slug}")
}

/// Parses and validates every source against the manifest, collecting
/// diagnostics across all files — `blog check` and publish share this gate,
/// so nothing invalid can reach KV (user stories 13, 14).
pub fn check(
    posts: &[PostSource],
    manifest: &Manifest,
) -> Result<Vec<ParsedPost>, Vec<Diagnostic>> {
    let mut parsed = Vec::new();
    let mut diags = Vec::new();
    for post in posts {
        match content_parser::parse_validated_named(&post.source, &post.file, manifest) {
            Ok(document) => {
                diags.extend(check_date(&document, &post.file));
                parsed.push(ParsedPost {
                    slug: post.slug.clone(),
                    document,
                });
            }
            Err(errs) => diags.extend(errs),
        }
    }
    if diags.is_empty() {
        Ok(parsed)
    } else {
        Err(diags)
    }
}

/// The index orders lexicographically on `date`, which is only correct for
/// ISO `YYYY-MM-DD` — anything else is a publish-blocking diagnostic.
fn check_date(document: &Document, file: &str) -> Option<Diagnostic> {
    let date = document.frontmatter.date.as_bytes();
    let shape_ok = date.len() == 10
        && date.iter().enumerate().all(|(i, b)| match i {
            4 | 7 => *b == b'-',
            _ => b.is_ascii_digit(),
        });
    (!shape_ok).then(|| Diagnostic {
        message: format!(
            "frontmatter `date` must be YYYY-MM-DD, got \"{}\"",
            document.frontmatter.date
        ),
        file: Some(file.to_string()),
        line: None,
        column: None,
    })
}

/// Merges changed and removed posts into the previous index and lays out the
/// KV writes. Last-write-wins on the whole index (single-writer, per PRD).
pub fn plan(
    prev_index: Vec<IndexEntry>,
    changed: &[ParsedPost],
    removed: &[String],
) -> Result<PublishPlan, AstError> {
    let replaced =
        |slug: &str| removed.iter().any(|r| r == slug) || changed.iter().any(|p| p.slug == slug);

    let mut index: Vec<IndexEntry> = prev_index
        .into_iter()
        .filter(|entry| !replaced(&entry.slug))
        .chain(
            changed
                .iter()
                .map(|post| IndexEntry::new(&post.slug, &post.document.frontmatter)),
        )
        .collect();
    index.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| a.slug.cmp(&b.slug)));

    let index_json = serde_json::to_string(&index).map_err(AstError::Json)?;
    let writes = changed
        .iter()
        .map(|post| {
            Ok(KvWrite {
                key: post_key(&post.slug),
                value: post.document.to_json()?,
            })
        })
        .chain(std::iter::once(Ok(KvWrite {
            key: INDEX_KEY.to_string(),
            value: index_json,
        })))
        .collect::<Result<Vec<_>, AstError>>()?;

    Ok(PublishPlan {
        writes,
        deletes: removed.iter().map(|slug| post_key(slug)).collect(),
        index,
    })
}
