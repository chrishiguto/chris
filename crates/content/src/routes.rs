//! KV keys and public URL paths, defined once so routers, sitemaps, and
//! publish plans never hand-copy literals.

use std::collections::BTreeSet;

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

/// The writing page's public path; post pages and the tag filter hang off it.
pub const POSTS_PATH: &str = "/posts";

/// A post's public path (and cache key / purge path).
pub fn post_path(slug: &str) -> String {
    format!("{POSTS_PATH}/{slug}")
}

/// The writing page's filter query parameter: the selected tags ride in it
/// comma-separated, so a filtered view is a plain shareable URL.
pub const TAG_FILTER_PARAM: &str = "q";

/// Tag grammar: lowercase letters, digits, and `-`, never empty. This
/// charset is what lets tags ride verbatim in [`TAG_FILTER_PARAM`]'s value
/// and keeps its comma separator unambiguous.
pub fn valid_tag(tag: &str) -> bool {
    !tag.is_empty()
        && tag
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// A tag's filter target on the writing page: a single-tag deep link into
/// the listing's filter island — [`tag_filter_path_selected`]'s singleton
/// case.
pub fn tag_filter_path(tag: &str) -> String {
    tag_filter_path_selected(&BTreeSet::from([tag.to_string()]))
}

/// The listing path carrying a whole selection, bare when it is empty.
/// `BTreeSet` iteration keeps the URL canonical — sorted and deduped — so
/// equal selections share one URL however they were clicked together.
pub fn tag_filter_path_selected(tags: &BTreeSet<String>) -> String {
    if tags.is_empty() {
        return POSTS_PATH.to_string();
    }
    let tags: Vec<&str> = tags.iter().map(String::as_str).collect();
    format!("{POSTS_PATH}?{TAG_FILTER_PARAM}={}", tags.join(","))
}

/// The path builders' inverse, over the filter parameter's already-decoded
/// values (URL decoding is the browser's job — [`valid_tag`]'s grammar means
/// the builders never encode anything to begin with): every tag the values
/// name, across repeats and comma lists; empty segments drop.
pub fn tag_filter_selection(values: impl IntoIterator<Item = String>) -> BTreeSet<String> {
    let mut tags = BTreeSet::new();
    for value in values {
        tags.extend(
            value
                .split(',')
                .filter(|tag| !tag.is_empty())
                .map(str::to_string),
        );
    }
    tags
}

/// Index-backed HTML listing pages: routed, sitemapped, purged on publish.
pub const LISTING_PAGES: [&str; 2] = ["/", POSTS_PATH];

/// The about page's public path (and cache key / purge path).
pub const ABOUT_PATH: &str = "/about";

/// Hardcoded pages with no KV read: routed and sitemapped, but cached under
/// the site tag alone — they change on deploy, never on publish.
pub const STATIC_PAGES: [&str; 1] = [ABOUT_PATH];

/// The Atom feed's public path (and cache key / purge path).
pub const RSS_PATH: &str = "/rss.xml";

/// The sitemap's public path (and cache key / purge path).
pub const SITEMAP_PATH: &str = "/sitemap.xml";

/// The index-backed XML feeds; purged on every publish (not sitemap-listed).
pub const FEED_PATHS: [&str; 2] = [RSS_PATH, SITEMAP_PATH];

/// Cache tag carried by every cacheable response; purging it evicts the site.
pub const SITE_TAG: &str = "site";

/// Cache tag shared by the index-backed views (listings, feeds): they
/// project every post, so any content change purges them together.
pub const VIEWS_TAG: &str = "views";

/// Cache tag of one post's page; scoped purges evict exactly the posts that
/// changed.
pub fn post_tag(slug: &str) -> String {
    format!("post:{slug}")
}

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
    fn tag_filter_path_rides_the_listing_query() {
        assert_eq!(tag_filter_path("rust"), "/posts?q=rust");
    }

    #[test]
    fn valid_tag_admits_only_the_comma_safe_slug_charset() {
        assert!(valid_tag("rust"));
        assert!(valid_tag("web-2"));
        assert!(!valid_tag(""));
        assert!(!valid_tag("Rust"));
        assert!(!valid_tag("a,b"));
        assert!(!valid_tag("a b"));
    }

    #[test]
    fn tag_filter_path_selected_sorts_tags_and_bares_the_empty_selection() {
        let selected = BTreeSet::from(["wasm".to_string(), "rust".to_string()]);
        assert_eq!(tag_filter_path_selected(&selected), "/posts?q=rust,wasm");
        assert_eq!(tag_filter_path_selected(&BTreeSet::new()), "/posts");
    }

    #[test]
    fn tag_filter_selection_inverts_the_paths_over_decoded_values() {
        let selected = BTreeSet::from(["rust".to_string(), "wasm".to_string()]);
        // Slug-grammar tags carry no escapes, so a built query value is
        // already the decoded form the browser hands back.
        let query = tag_filter_path_selected(&selected);
        let (_, value) = query.split_once('=').unwrap();
        assert_eq!(tag_filter_selection([value.to_string()]), selected);
        let single = tag_filter_path("rust");
        let (_, value) = single.split_once('=').unwrap();
        assert_eq!(
            tag_filter_selection([value.to_string()]),
            BTreeSet::from(["rust".to_string()])
        );
        // Repeated params union; empty segments drop.
        assert_eq!(
            tag_filter_selection(["wasm".to_string(), "rust,".to_string()]),
            selected
        );
        assert_eq!(tag_filter_selection([String::new()]), BTreeSet::new());
        assert_eq!(tag_filter_selection(None::<String>), BTreeSet::new());
    }

    // Tag browsing is an in-page filter, never a routed page.
    #[test]
    fn listing_pages_are_home_and_posts_only() {
        assert_eq!(LISTING_PAGES, ["/", "/posts"]);
    }

    #[test]
    fn static_pages_carry_the_about_path() {
        assert_eq!(ABOUT_PATH, "/about");
        assert!(STATIC_PAGES.contains(&ABOUT_PATH));
    }

    #[test]
    fn cache_tags_name_one_post_or_a_shared_scope() {
        assert_eq!(post_tag("hello"), "post:hello");
        assert_ne!(SITE_TAG, VIEWS_TAG);
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
