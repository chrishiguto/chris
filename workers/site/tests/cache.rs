//! Native tests for the pure cache policy.

use site::cache::{cache_key, etag, not_modified, should_cache, CACHE_CONTROL};

#[test]
fn cache_control_splits_edge_ttl_from_browser_revalidation() {
    assert_eq!(CACHE_CONTROL, "max-age=0, s-maxage=604800");
}

#[test]
fn cache_key_is_the_absolute_url() {
    assert_eq!(
        cache_key("https://blog.example.com/posts/hello"),
        Some("https://blog.example.com/posts/hello".to_string())
    );
    assert_eq!(
        cache_key("https://blog.example.com/"),
        Some("https://blog.example.com/".to_string())
    );
}

/// Variants must share one entry — a `?utm=…` copy that survived a purge
/// would serve stale content for the full TTL.
#[test]
fn cache_key_strips_query_and_fragment() {
    assert_eq!(
        cache_key("https://blog.example.com/posts/hello?utm_source=feed"),
        Some("https://blog.example.com/posts/hello".to_string())
    );
    assert_eq!(
        cache_key("https://blog.example.com/tags#top"),
        Some("https://blog.example.com/tags".to_string())
    );
}

/// A relative URI means no key rather than a malformed one.
#[test]
fn cache_key_requires_an_absolute_url() {
    assert_eq!(cache_key("/posts/hello"), None);
    assert_eq!(cache_key("://missing-scheme/x"), None);
}

#[test]
fn should_cache_requires_ok_status_and_the_marker_header() {
    assert!(should_cache(200, Some(CACHE_CONTROL)));
    assert!(!should_cache(200, None));
    assert!(!should_cache(200, Some("no-store")));
    assert!(!should_cache(404, Some(CACHE_CONTROL)));
    assert!(!should_cache(500, Some(CACHE_CONTROL)));
}

#[test]
fn etag_is_the_quoted_snapshot_sha() {
    assert_eq!(etag("abc123def456"), "\"abc123def456\"");
}

#[test]
fn not_modified_matches_the_exact_validator() {
    let tag = etag("abc123");
    assert!(not_modified(&tag, &tag));
    assert!(!not_modified("\"other\"", &tag));
    assert!(!not_modified("abc123", &tag)); // unquoted is not a match
}

#[test]
fn not_modified_handles_lists_weak_prefixes_and_star() {
    let tag = etag("abc123");
    assert!(not_modified(&format!("\"stale\", {tag}"), &tag));
    assert!(not_modified(&format!("W/{tag}"), &tag));
    assert!(not_modified("*", &tag));
    assert!(!not_modified("\"a\", \"b\"", &tag));
}
