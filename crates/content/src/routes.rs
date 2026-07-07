//! The KV-key and public-URL vocabulary, defined once (ADR-0008 amendment).
//!
//! The write path's purge set "must mirror how the site keys its cache
//! entries" — that mirror is enforced here by construction: the site's
//! router/sitemap, the app's hrefs, and the publish plan's keys and purge
//! paths all derive from these definitions instead of hand-copied literals.

/// KV key of the ordered post listing (PRD "KV schema").
pub const INDEX_KEY: &str = "index";

/// KV key of one post's AST document.
pub fn post_key(slug: &str) -> String {
    format!("post:{slug}")
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
