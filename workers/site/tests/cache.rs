//! Native tests for the pure cache policy.

use site::cache::{
    etag, is_entity_header, not_modified, post_cache_tags, purge_tags, revalidates, view_cache_tags,
};

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

/// A 404 or 500 with a stray ETag must never lose its body.
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
}

#[test]
fn cache_tags_pair_the_site_scope_with_the_specific_one() {
    assert_eq!(post_cache_tags("hello"), "site,post:hello");
    assert_eq!(view_cache_tags(), "site,views");
}

/// The break-glass contract: a bodyless purge means "everything".
#[test]
fn purge_tags_default_an_empty_body_to_the_site_tag() {
    assert_eq!(purge_tags(b"").unwrap(), vec!["site"]);
    assert_eq!(purge_tags(b" \n\t").unwrap(), vec!["site"]);
}

#[test]
fn purge_tags_parse_an_explicit_list() {
    assert_eq!(
        purge_tags(br#"{"tags":["post:hello","views"]}"#).unwrap(),
        vec!["post:hello", "views"]
    );
}

#[test]
fn purge_tags_reject_malformed_bodies() {
    assert!(purge_tags(b"not json").is_err());
    assert!(purge_tags(br#"{"tags":[]}"#).is_err());
    assert!(purge_tags(br#"{"tags":[""]}"#).is_err());
    assert!(purge_tags(br#"{"tags":["  "]}"#).is_err());
    assert!(purge_tags(br#"{"urls":["/"]}"#).is_err());
}
