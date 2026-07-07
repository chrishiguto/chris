//! `parse_index`/`parse_pointer` boundaries: the first-publish sentinel is
//! explicit, everything else fails closed.

use xtask::{parse_index, parse_pointer};

#[test]
fn empty_output_means_first_publish() {
    assert_eq!(parse_index("").unwrap(), vec![]);
    assert_eq!(parse_index("  \n").unwrap(), vec![]);
}

#[test]
fn wranglers_value_not_found_means_first_publish() {
    // wrangler prints exactly this to stdout and exits 0 on a missing key
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
    // all must error rather than plan a from-scratch index
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

#[test]
fn missing_pointer_means_no_snapshot_yet() {
    assert_eq!(parse_pointer("").unwrap(), None);
    assert_eq!(parse_pointer("Value not found\n").unwrap(), None);
}

#[test]
fn a_real_pointer_yields_its_sha() {
    assert_eq!(
        parse_pointer(r#"{"sha":"abc123"}"#).unwrap(),
        Some("abc123".to_string())
    );
}

#[test]
fn a_garbled_pointer_fails_closed() {
    // falling back to "no snapshot" on garbage would compute a wrong purge set
    for garbage in ["<html>502</html>", r#"{"sha""#, "value not found"] {
        assert!(
            parse_pointer(garbage).is_err(),
            "must reject as non-pointer: {garbage:?}"
        );
    }
}
