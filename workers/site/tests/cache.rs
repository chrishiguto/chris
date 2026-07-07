//! Native tests for the pure cache policy (ADR-0008): what the shim's
//! Cache API front keys on and which responses it may store.

use site::cache::{cache_key, should_cache, CACHE_CONTROL};

#[test]
fn cache_control_is_the_seven_day_ttl() {
    assert_eq!(CACHE_CONTROL, "max-age=604800");
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

/// Query and fragment variants must share one entry: the purge set names
/// pages as bare `origin + path` URLs (the `publish` crate), and a `?utm=…` copy
/// that survived a purge would serve stale content for the full TTL.
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

/// The Cache API keys on absolute URLs; a relative URI (dev-server edge
/// case) means no key rather than a malformed one.
#[test]
fn cache_key_requires_an_absolute_url() {
    assert_eq!(cache_key("/posts/hello"), None);
    assert_eq!(cache_key("://missing-scheme/x"), None);
}

/// Only responses a handler explicitly marked cacheable are stored: 200 +
/// our exact Cache-Control value. Drafts and 404s never carry the header.
#[test]
fn should_cache_requires_ok_status_and_the_marker_header() {
    assert!(should_cache(200, Some(CACHE_CONTROL)));
    assert!(!should_cache(200, None));
    assert!(!should_cache(200, Some("no-store")));
    assert!(!should_cache(404, Some(CACHE_CONTROL)));
    assert!(!should_cache(500, Some(CACHE_CONTROL)));
}
