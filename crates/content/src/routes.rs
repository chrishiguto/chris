//! The KV-key and public-URL vocabulary, defined once: the site's router
//! and sitemap, the app's hrefs, and the publish plan's keys and purge
//! paths all derive from here instead of hand-copied literals.

use serde::{Deserialize, Serialize};

/// The pointer naming the published snapshot — the only mutable content
/// key. Publishing writes the full `snapshot:{sha}:*` set and flips this
/// last, so readers see whole snapshots, never a blend.
pub const CURRENT_KEY: &str = "current";

/// Value stored under [`CURRENT_KEY`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentPointer {
    /// The snapshot's identity: the commit SHA it was built from (or a
    /// `manual-*` label for break-glass publishes).
    pub sha: String,
}

impl CurrentPointer {
    /// Fail-closed for every reader: a corrupt pointer is an error, never a
    /// fallback to the wrong snapshot.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// KV key of a snapshot's ordered post listing.
pub fn snapshot_index_key(sha: &str) -> String {
    format!("{}index", snapshot_prefix(sha))
}

/// KV key of one post's AST document inside a snapshot.
pub fn snapshot_post_key(sha: &str, slug: &str) -> String {
    format!("{}post:{slug}", snapshot_prefix(sha))
}

/// Common prefix of every key in one snapshot.
fn snapshot_prefix(sha: &str) -> String {
    format!("{SNAPSHOT_KEY_SPACE}{sha}:")
}

/// The sha a `snapshot:{sha}:…` key belongs to; anything else is not a
/// snapshot key. The inverse of the key builders above.
pub fn snapshot_key_sha(key: &str) -> Option<&str> {
    let rest = key.strip_prefix(SNAPSHOT_KEY_SPACE)?;
    let (sha, _) = rest.split_once(':')?;
    (!sha.is_empty()).then_some(sha)
}

/// Prefix shared by every snapshot key of any sha.
pub const SNAPSHOT_KEY_SPACE: &str = "snapshot:";

/// The ordered post listing (pre-snapshot layout; readers fall back to it
/// until the first pointer flip).
pub const INDEX_KEY: &str = "index";

/// One post's AST document (pre-snapshot layout, as [`INDEX_KEY`]).
pub fn post_key(slug: &str) -> String {
    format!("post:{slug}")
}

/// Where the index lives given the resolved pointer: the snapshot's, or the
/// pre-snapshot flat key before the first flip. Read and write paths share
/// this rule so the fallback can be retired in one place.
pub fn index_key_at(current: Option<&str>) -> String {
    match current {
        Some(sha) => snapshot_index_key(sha),
        None => INDEX_KEY.to_string(),
    }
}

/// Where a post's payload lives given the resolved `current` pointer, as
/// [`index_key_at`].
pub fn post_key_at(current: Option<&str>, slug: &str) -> String {
    match current {
        Some(sha) => snapshot_post_key(sha, slug),
        None => post_key(slug),
    }
}

/// A post's public path (and cache key / purge path).
pub fn post_path(slug: &str) -> String {
    format!("/posts/{slug}")
}

/// A tag page's public path (and cache key / purge path).
pub fn tag_path(tag: &str) -> String {
    format!("/tags/{tag}")
}

/// The index-backed HTML listing pages: routed by the site, listed in the
/// sitemap, and purged on every publish.
pub const LISTING_PAGES: [&str; 3] = ["/", "/posts", "/tags"];

/// The index-backed XML feeds; purged on every publish (not sitemap-listed).
pub const FEED_PATHS: [&str; 2] = ["/rss.xml", "/sitemap.xml"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_key_sha_inverts_the_key_builders() {
        assert_eq!(
            snapshot_key_sha(&snapshot_index_key("abc123")),
            Some("abc123")
        );
        assert_eq!(
            snapshot_key_sha(&snapshot_post_key("abc123", "hello")),
            Some("abc123")
        );
    }

    #[test]
    fn snapshot_key_sha_rejects_non_snapshot_keys() {
        assert_eq!(snapshot_key_sha(CURRENT_KEY), None);
        assert_eq!(snapshot_key_sha(INDEX_KEY), None);
        assert_eq!(snapshot_key_sha(&post_key("hello")), None);
        assert_eq!(snapshot_key_sha("snapshot:"), None);
        assert_eq!(snapshot_key_sha("snapshot::index"), None);
    }

    #[test]
    fn keys_at_a_pointer_fall_back_to_the_flat_layout() {
        assert_eq!(index_key_at(Some("abc123")), "snapshot:abc123:index");
        assert_eq!(index_key_at(None), INDEX_KEY);
        assert_eq!(
            post_key_at(Some("abc123"), "hello"),
            "snapshot:abc123:post:hello"
        );
        assert_eq!(post_key_at(None, "hello"), "post:hello");
    }
}
