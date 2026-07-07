//! The publish operation's pure core: validate post sources and lay a
//! snapshot out as KV writes. Wasm-clean — no filesystem, HTTP, or clock;
//! callers own transport. Snapshots are immutable (`snapshot:{sha}:*`); the
//! caller flips `current` afterwards, so readers never see a blend.

use content::{
    post_path, snapshot_index_key, snapshot_post_key, tag_path, AstError, Diagnostic, Document,
    IndexEntry, Manifest, FEED_PATHS, LISTING_PAGES,
};

/// One post's raw authoring input, however the caller obtained it.
#[derive(Debug, Clone)]
pub struct PostSource {
    /// Directory name under `content/blog/`; the KV and URL identity.
    pub slug: String,
    /// Path stamped into diagnostics.
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

/// A post carried into the new snapshot unchanged: its source at HEAD
/// failed validation, so the previous entry and payload ride along instead.
#[derive(Debug, Clone)]
pub struct CarriedPost {
    pub entry: IndexEntry,
    /// The serialized `Document` exactly as the previous snapshot stored it.
    pub payload: String,
}

/// One KV put the caller must perform. Serializes to exactly the
/// `{"key","value"}` shape `wrangler kv bulk put` consumes.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct KvWrite {
    pub key: String,
    pub value: String,
}

/// The purge-by-URL API caps each request at 30 files (non-Enterprise).
pub const PURGE_FILES_LIMIT: usize = 30;

/// Everything one snapshot publish must do to KV. The caller flips
/// `current` after all writes, then purges.
#[derive(Debug, Clone)]
pub struct SnapshotPlan {
    /// `snapshot:{sha}:post:*` payloads; land these first, in any order.
    pub post_writes: Vec<KvWrite>,
    /// `snapshot:{sha}:index`, written only after every post write — a torn
    /// write must never leave an index naming missing posts.
    pub index_write: KvWrite,
    /// The new index, newest-first.
    pub index: Vec<IndexEntry>,
    /// URL paths this publish invalidates: the whole enumerated set of the
    /// previous and new indexes — a full rebuild records no body deltas.
    /// Sorted, deduplicated; callers prefix their origin.
    pub purge: Vec<String>,
}

impl SnapshotPlan {
    /// The purge set as absolute URLs, chunked to the API's per-request
    /// cap; transports wrap each chunk in their own wire format.
    pub fn purge_chunks(&self, origin: &str) -> Vec<Vec<String>> {
        let origin = origin.trim_end_matches('/');
        self.purge
            .chunks(PURGE_FILES_LIMIT)
            .map(|chunk| chunk.iter().map(|path| format!("{origin}{path}")).collect())
            .collect()
    }
}

/// Validates every source against the manifest, collecting diagnostics
/// across all files — nothing invalid can reach KV.
pub fn check(
    posts: &[PostSource],
    manifest: &Manifest,
) -> Result<Vec<ParsedPost>, Vec<Diagnostic>> {
    let mut parsed = Vec::new();
    let mut diags = Vec::new();
    for result in check_each(posts, manifest) {
        match result {
            Ok(post) => parsed.push(post),
            Err(errs) => diags.extend(errs),
        }
    }
    if diags.is_empty() {
        Ok(parsed)
    } else {
        Err(diags)
    }
}

/// The same gate as [`check`], per post: each source passes or fails on its
/// own, in input order — one broken post must not wedge the rest. Frontmatter
/// and component rules live in `content::parse_validated`; only the slug
/// (a directory name, invisible to the parser) is checked here.
pub fn check_each(
    posts: &[PostSource],
    manifest: &Manifest,
) -> Vec<Result<ParsedPost, Vec<Diagnostic>>> {
    posts
        .iter()
        .map(|post| {
            let mut diags: Vec<Diagnostic> = check_slug(post).into_iter().collect();
            let document = match content::parse_validated(&post.source, &post.file, manifest) {
                Ok(document) => Some(document),
                Err(errs) => {
                    diags.extend(errs);
                    None
                }
            };
            match (document, diags.is_empty()) {
                (Some(document), true) => Ok(ParsedPost {
                    slug: post.slug.clone(),
                    document,
                }),
                _ => Err(diags),
            }
        })
        .collect()
}

/// Slugs name the post URL, its KV keys, and its co-located component
/// module — enforce [`content::valid_slug`] before anything consumes them.
fn check_slug(post: &PostSource) -> Option<Diagnostic> {
    (!content::valid_slug(&post.slug)).then(|| Diagnostic {
        message: format!(
            "slug \"{}\" must be a lowercase slug (a-z, 0-9, -) starting with a letter — it \
             names the /posts/{{slug}} URL and the post's component module",
            post.slug
        ),
        file: Some(post.file.clone()),
        line: None,
        column: None,
    })
}

/// Lays out one immutable snapshot: every checked post, every carried post,
/// and the index built from exactly those — absent from both means retired.
/// `prev_index` feeds only the purge set.
pub fn snapshot(
    prev_index: &[IndexEntry],
    posts: &[ParsedPost],
    carried: Vec<CarriedPost>,
    sha: &str,
) -> Result<SnapshotPlan, AstError> {
    let mut index: Vec<IndexEntry> = posts
        .iter()
        .map(|post| IndexEntry::new(&post.slug, &post.document.frontmatter))
        .chain(carried.iter().map(|post| post.entry.clone()))
        .collect();
    index.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| a.slug.cmp(&b.slug)));
    let purge = purge_paths(prev_index, &index);

    let index_write = KvWrite {
        key: snapshot_index_key(sha),
        value: serde_json::to_string(&index).map_err(AstError::Json)?,
    };
    let post_writes = posts
        .iter()
        .map(|post| {
            Ok(KvWrite {
                key: snapshot_post_key(sha, &post.slug),
                value: post.document.to_json()?,
            })
        })
        .chain(carried.into_iter().map(|post| {
            Ok(KvWrite {
                key: snapshot_post_key(sha, &post.entry.slug),
                value: post.payload,
            })
        }))
        .collect::<Result<Vec<_>, AstError>>()?;

    Ok(SnapshotPlan {
        post_writes,
        index_write,
        index,
        purge,
    })
}

/// Listings and feeds always change; every post and tag URL either side of
/// the publish knows about may have changed or vanished.
fn purge_paths(prev_index: &[IndexEntry], index: &[IndexEntry]) -> Vec<String> {
    let entries = prev_index.iter().chain(index);
    LISTING_PAGES
        .into_iter()
        .chain(FEED_PATHS)
        .map(String::from)
        .chain(entries.flat_map(|entry| {
            entry
                .tags
                .iter()
                .map(|tag| tag_path(tag))
                .chain(std::iter::once(post_path(&entry.slug)))
        }))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}
