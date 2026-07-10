//! Pure publish planning: validate post sources, lay a snapshot out as KV
//! writes. Wasm-clean — no fs, HTTP, or clock; callers own transport.
//! Snapshots are immutable; the caller flips `current` last, so readers never see a blend.

use content::{
    post_tag, snapshot_index_key, snapshot_post_key, AstError, Diagnostic, Document, IndexEntry,
    Manifest, VIEWS_TAG,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

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

/// Content-addresses one serialized post payload; index entries carry it so
/// the next publish can tell changed posts from untouched ones.
pub fn content_hash(payload: &str) -> String {
    format!("{:x}", Sha256::digest(payload.as_bytes()))
}

/// Cache tags a flip from `prev` to `next` made stale: every added, removed,
/// or changed post, plus the shared views tag once anything changed at all.
/// Empty means readers see no difference — nothing to purge. Entries without
/// a hash always count as changed.
pub fn stale_tags(prev: &[IndexEntry], next: &[IndexEntry]) -> Vec<String> {
    let prev_hashes: BTreeMap<&str, &str> = prev
        .iter()
        .map(|entry| (entry.slug.as_str(), entry.content_hash.as_str()))
        .collect();
    let next_slugs: BTreeSet<&str> = next.iter().map(|entry| entry.slug.as_str()).collect();

    let mut changed: BTreeSet<&str> = BTreeSet::new();
    for entry in next {
        if entry.content_hash.is_empty()
            || prev_hashes.get(entry.slug.as_str()) != Some(&entry.content_hash.as_str())
        {
            changed.insert(&entry.slug);
        }
    }
    changed.extend(
        prev_hashes
            .keys()
            .filter(|slug| !next_slugs.contains(*slug)),
    );

    let mut tags: Vec<String> = changed.iter().map(|slug| post_tag(slug)).collect();
    if !tags.is_empty() {
        tags.push(VIEWS_TAG.to_string());
    }
    tags
}

/// Lays out one immutable snapshot: the index is exactly checked + carried
/// posts — absent from both means retired. Every entry gets its payload's
/// hash, so pre-hash entries riding in as carried posts heal in place.
pub fn snapshot(
    posts: &[ParsedPost],
    carried: Vec<CarriedPost>,
    sha: &str,
) -> Result<SnapshotPlan, AstError> {
    let mut index: Vec<IndexEntry> = Vec::with_capacity(posts.len() + carried.len());
    let mut post_writes: Vec<KvWrite> = Vec::with_capacity(posts.len() + carried.len());
    for post in posts {
        let payload = post.document.to_json()?;
        let mut entry = IndexEntry::new(&post.slug, &post.document.frontmatter);
        entry.content_hash = content_hash(&payload);
        entry.reading_minutes = Some(content::reading_minutes(&post.document.ast));
        index.push(entry);
        post_writes.push(KvWrite {
            key: snapshot_post_key(sha, &post.slug),
            value: payload,
        });
    }
    for CarriedPost { mut entry, payload } in carried {
        entry.content_hash = content_hash(&payload);
        post_writes.push(KvWrite {
            key: snapshot_post_key(sha, &entry.slug),
            value: payload,
        });
        index.push(entry);
    }
    index.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| a.slug.cmp(&b.slug)));

    let index_write = KvWrite {
        key: snapshot_index_key(sha),
        value: serde_json::to_string(&index).map_err(AstError::Json)?,
    };

    Ok(SnapshotPlan {
        post_writes,
        index_write,
        index,
    })
}
