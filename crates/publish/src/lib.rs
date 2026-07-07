//! The publish operation's pure core: validate post sources and
//! turn them into a KV write plan. Shared by `xtask` and the
//! pipeline worker — so it stays wasm-clean: no filesystem, no
//! HTTP, no clock. Callers own transport (Cloudflare API vs KV binding).

use content::{
    post_key, post_path, tag_path, AstError, Diagnostic, Document, IndexEntry, Manifest,
    FEED_PATHS, INDEX_KEY, LISTING_PAGES,
};

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
    /// URL paths whose cache entries this publish invalidates:
    /// listings, feeds, the touched posts, and every tag page whose listing
    /// changed — tags on the new frontmatter *and* tags the previous index
    /// had on the touched posts (a dropped tag's page loses an entry too).
    /// Sorted, deduplicated; callers prefix their origin when purging.
    pub purge: Vec<String>,
}

/// Parses and validates every source against the manifest, collecting
/// diagnostics across all files — `xtask check` and publish share this gate,
/// so nothing invalid can reach KV (user stories 13, 14).
pub fn check(
    posts: &[PostSource],
    manifest: &Manifest,
) -> Result<Vec<ParsedPost>, Vec<Diagnostic>> {
    let mut parsed = Vec::new();
    let mut diags = Vec::new();
    for post in posts {
        match content::parse_validated_named(&post.source, &post.file, manifest) {
            Ok(document) => {
                diags.extend(check_date(&document, &post.file));
                diags.extend(check_tags(&document, &post.file));
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

/// Each tag names a `/tags/{tag}` URL verbatim (pages, feeds, purge set), so
/// tags must be lowercase slugs — no escaping layer exists or is wanted.
fn check_tags<'a>(document: &'a Document, file: &'a str) -> impl Iterator<Item = Diagnostic> + 'a {
    document
        .frontmatter
        .tags
        .iter()
        .filter(|tag| {
            tag.is_empty()
                || !tag
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
        .map(move |tag| Diagnostic {
            message: format!(
                "tag \"{tag}\" must be a lowercase slug (a-z, 0-9, -) — it becomes the /tags/{{tag}} URL"
            ),
            file: Some(file.to_string()),
            line: None,
            column: None,
        })
}

/// Merges changed and removed posts into the previous index and lays out the
/// KV writes. Last-write-wins on the whole index (single-writer).
pub fn plan(
    prev_index: Vec<IndexEntry>,
    changed: &[ParsedPost],
    removed: &[String],
) -> Result<PublishPlan, AstError> {
    let replaced =
        |slug: &str| removed.iter().any(|r| r == slug) || changed.iter().any(|p| p.slug == slug);
    let purge = purge_paths(&prev_index, changed, removed, &replaced);

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
        purge,
    })
}

/// The enumerated invalidation set: listings and feeds always
/// change (dates, counts, order), touched posts change or vanish, and a tag
/// page changes iff a touched post carries the tag now or carried it before.
fn purge_paths(
    prev_index: &[IndexEntry],
    changed: &[ParsedPost],
    removed: &[String],
    replaced: &impl Fn(&str) -> bool,
) -> Vec<String> {
    let touched_tags = prev_index
        .iter()
        .filter(|entry| replaced(&entry.slug))
        .flat_map(|entry| entry.tags.iter())
        .chain(
            changed
                .iter()
                .flat_map(|post| post.document.frontmatter.tags.iter()),
        );
    LISTING_PAGES
        .into_iter()
        .chain(FEED_PATHS)
        .map(String::from)
        .chain(touched_tags.map(|tag| tag_path(tag)))
        .chain(changed.iter().map(|post| post_path(&post.slug)))
        .chain(removed.iter().map(|slug| post_path(slug)))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}
