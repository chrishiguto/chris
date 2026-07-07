//! `parse_index` boundary tests: the first-publish sentinel is explicit —
//! empty output or wrangler's exact `Value not found` — and everything else
//! fails closed. A swallowed index would make `plan` rewrite the listing to
//! only the posts it was given, silently unlisting the rest of the site.

use xtask::parse_index;

#[test]
fn empty_output_means_first_publish() {
    assert_eq!(parse_index("").unwrap(), vec![]);
    assert_eq!(parse_index("  \n").unwrap(), vec![]);
}

#[test]
fn wranglers_value_not_found_means_first_publish() {
    // Verified against wrangler: `kv key get` on a missing key prints
    // exactly this to stdout and exits 0.
    assert_eq!(parse_index("Value not found\n").unwrap(), vec![]);
}

#[test]
fn a_real_index_round_trips() {
    let entries = parse_index(
        r#"[
            {"slug": "newer", "title": "Newer", "date": "2026-06-15"},
            {"slug": "older", "title": "Older", "date": "2026-06-01", "tags": ["rust"], "draft": true}
        ]"#,
    )
    .unwrap();
    let slugs: Vec<_> = entries.iter().map(|e| e.slug.as_str()).collect();
    assert_eq!(slugs, ["newer", "older"]);
    assert!(entries[1].draft);
}

#[test]
fn unexpected_content_fails_closed() {
    // An HTML error page, a changed wrangler message, truncated JSON: all
    // must error rather than plan a from-scratch index.
    for garbage in [
        "<html><body>502 Bad Gateway</body></html>",
        "value not found",
        "Value not found for key index",
        r#"[{"slug": "trunc"#,
    ] {
        assert!(
            parse_index(garbage).is_err(),
            "must reject as non-index: {garbage:?}"
        );
    }
}
