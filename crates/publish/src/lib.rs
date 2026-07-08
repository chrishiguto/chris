//! Pure publish planning: validate post sources, lay a snapshot out as KV
//! writes. Wasm-clean — no fs, HTTP, or clock; callers own transport.
//! Snapshots are immutable; the caller flips `current` last, so readers never see a blend.

use content::{
    snapshot_index_key, snapshot_post_key, AstError, Diagnostic, Document, IndexEntry, Manifest,
};

#[derive(Debug, Clone)]
pub struct PostSource {
    /// Directory name under `content/blog/`; the KV and URL identity.
    pub slug: String,
    /// Path stamped into diagnostics.
    pub file: String,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct ParsedPost {
    pub slug: String,
    pub document: Document,
}

/// A post whose HEAD source failed validation; the previous entry and
/// payload ride into the new snapshot unchanged.
#[derive(Debug, Clone)]
pub struct CarriedPost {
    pub entry: IndexEntry,
    /// Serialized `Document` exactly as the previous snapshot stored it.
    pub payload: String,
}

/// One KV put; serializes to the `{"key","value"}` shape `wrangler kv bulk put` consumes.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct KvWrite {
    pub key: String,
    pub value: String,
}

/// All KV writes for one snapshot publish; the caller flips `current` after
/// all writes. Cache invalidation is the caller's transport concern.
#[derive(Debug, Clone)]
pub struct SnapshotPlan {
    /// Land these first, in any order.
    pub post_writes: Vec<KvWrite>,
    /// Write only after every post write — a torn publish must never leave
    /// an index naming missing posts.
    pub index_write: KvWrite,
    /// The new index, newest-first.
    pub index: Vec<IndexEntry>,
}

/// Validates every source, collecting diagnostics across all files —
/// nothing invalid can reach KV.
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

/// Per-post [`check`]: each source passes or fails on its own, in input
/// order — one broken post must not wedge the rest.
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

/// The slug is a directory name the parser never sees; gate it here before
/// URLs, KV keys, and module names consume it.
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

/// Lays out one immutable snapshot: the index is exactly checked + carried
/// posts — absent from both means retired.
pub fn snapshot(
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
    })
}
