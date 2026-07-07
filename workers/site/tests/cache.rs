//! Native tests for the pure cache policy.

use site::cache::{
    cache_key, etag, is_entity_header, not_modified, revalidates, should_cache, CACHE_CONTROL,
};

#[test]
fn cache_key_is_the_absolute_url() {
    assert_eq!(
        cache_key(Some("https"), Some("blog.example.com"), "/posts/hello"),
        Some("https://blog.example.com/posts/hello".to_string())
    );
    assert_eq!(
        cache_key(Some("https"), Some("blog.example.com"), "/"),
        Some("https://blog.example.com/".to_string())
    );
}

/// A relative request URI means no key rather than a malformed one.
#[test]
fn cache_key_requires_an_absolute_url() {
    assert_eq!(cache_key(None, None, "/posts/hello"), None);
    assert_eq!(cache_key(None, Some("blog.example.com"), "/x"), None);
    assert_eq!(cache_key(Some("https"), None, "/x"), None);
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

/// Only a matching 200 thins to a 304 — a 404 or 500 with a stray ETag
/// must never lose its body.
#[test]
fn revalidates_only_a_matching_200() {
    let tag = etag("abc123");
    assert!(revalidates(200, Some(&tag), Some(&tag)));
    assert!(!revalidates(404, Some(&tag), Some(&tag)));
    assert!(!revalidates(500, Some(&tag), Some(&tag)));
    assert!(!revalidates(200, Some("\"other\""), Some(&tag)));
    assert!(!revalidates(200, None, Some(&tag)));
    assert!(!revalidates(200, Some(&tag), None));
}

/// The 304 keeps validators and cache directives, drops body metadata.
#[test]
fn entity_headers_are_the_content_family() {
    assert!(is_entity_header("content-type"));
    assert!(is_entity_header("content-length"));
    assert!(!is_entity_header("etag"));
    assert!(!is_entity_header("cache-control"));
    assert!(!is_entity_header("x-blog-cache"));
}
