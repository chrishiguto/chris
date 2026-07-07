//! KV keys and public URL paths, defined once so routers, sitemaps, and
//! publish plans never hand-copy literals.

use serde::{Deserialize, Serialize};

/// The only mutable content key: names the published snapshot. Flipped last,
/// after the full `snapshot:{sha}:*` set, so readers never see a blend.
pub const CURRENT_KEY: &str = "current";

/// Value stored under [`CURRENT_KEY`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentPointer {
    /// Commit SHA the snapshot was built from, or a `manual-*` label.
    pub sha: String,
}

impl CurrentPointer {
    /// Fail-closed: a corrupt pointer is an error, never a fallback.
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

fn snapshot_prefix(sha: &str) -> String {
    format!("{SNAPSHOT_KEY_SPACE}{sha}:")
}

/// Inverse of the snapshot key builders; `None` for non-snapshot keys.
pub fn snapshot_key_sha(key: &str) -> Option<&str> {
    let rest = key.strip_prefix(SNAPSHOT_KEY_SPACE)?;
    let (sha, _) = rest.split_once(':')?;
    (!sha.is_empty()).then_some(sha)
}

pub const SNAPSHOT_KEY_SPACE: &str = "snapshot:";

/// Pre-snapshot flat listing key; readers fall back to it until the first
/// pointer flip.
pub const INDEX_KEY: &str = "index";

/// Pre-snapshot flat post key; see [`INDEX_KEY`].
pub fn post_key(slug: &str) -> String {
    format!("post:{slug}")
}

/// Index key for the resolved pointer, falling back to the flat pre-snapshot
/// key; read and write paths share this rule.
pub fn index_key_at(current: Option<&str>) -> String {
    match current {
        Some(sha) => snapshot_index_key(sha),
        None => INDEX_KEY.to_string(),
    }
}

/// As [`index_key_at`], for one post's payload.
pub fn post_key_at(current: Option<&str>, slug: &str) -> String {
    match current {
        Some(sha) => snapshot_post_key(sha, slug),
        None => post_key(slug),
    }
}

/// Slug grammar: lowercase letters, digits, and `-`, starting with a letter.
/// No underscores — `-` maps to `_` for the component module name.
pub fn valid_slug(slug: &str) -> bool {
    slug.starts_with(|c: char| c.is_ascii_lowercase())
        && slug
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// A post's public path (and cache key / purge path).
pub fn post_path(slug: &str) -> String {
    format!("/posts/{slug}")
}

/// A tag page's public path (and cache key / purge path).
pub fn tag_path(tag: &str) -> String {
    format!("/tags/{tag}")
}

/// Index-backed HTML listing pages: routed, sitemapped, purged on publish.
pub const LISTING_PAGES: [&str; 3] = ["/", "/posts", "/tags"];

/// The Atom feed's public path (and cache key / purge path).
pub const RSS_PATH: &str = "/rss.xml";

/// The sitemap's public path (and cache key / purge path).
pub const SITEMAP_PATH: &str = "/sitemap.xml";

/// The index-backed XML feeds; purged on every publish (not sitemap-listed).
pub const FEED_PATHS: [&str; 2] = [RSS_PATH, SITEMAP_PATH];

/// Authoring tree root: one `{CONTENT_ROOT}/{slug}/{POST_FILE}` per post.
pub const CONTENT_ROOT: &str = "content/blog";

pub const POST_FILE: &str = "index.mdx";

/// Repo path of a post source; inverse of [`post_slug`], not the public URL.
pub fn source_path(slug: &str) -> String {
    format!("{CONTENT_ROOT}/{slug}/{POST_FILE}")
}

/// `content/blog/{slug}/index.mdx` → the slug; `None` otherwise.
pub fn post_slug(path: &str) -> Option<&str> {
    let rest = path.strip_prefix(CONTENT_ROOT)?.strip_prefix('/')?;
    let (slug, file) = rest.split_once('/')?;
    (file == POST_FILE && !slug.is_empty()).then_some(slug)
}

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
    fn post_slug_matches_only_post_sources() {
        assert_eq!(post_slug("content/blog/hello/index.mdx"), Some("hello"));
        assert_eq!(post_slug("content/blog/hello/notes.txt"), None);
        assert_eq!(post_slug("content/blog/a/b/index.mdx"), None);
        assert_eq!(post_slug("content/blog/index.mdx"), None);
        assert_eq!(post_slug("docs/index.mdx"), None);
    }

    #[test]
    fn source_path_inverts_post_slug() {
        assert_eq!(source_path("hello"), "content/blog/hello/index.mdx");
        assert_eq!(post_slug(&source_path("hello")), Some("hello"));
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
