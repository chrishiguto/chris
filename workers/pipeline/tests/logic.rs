//! Native tests for the pipeline worker's pure decision logic.

use pipeline::{
    contents_url, failure_description, head_ref_url, parse_head_ref, parse_tree_listing,
    purge_scope, tree_post_slugs, tree_url, PublishOutcome, ReconcileConfig,
};

fn slugs(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

// --- reconcile vocabulary ---

#[test]
fn tree_post_slugs_filters_sorts_and_dedups() {
    let paths = [
        "content/blog/zeta/index.mdx",
        "content/blog/alpha/index.mdx",
        "content/blog/alpha/components.rs",
        "content/blog/alpha/index.mdx",
        "app/src/lib.rs",
        "README.md",
    ];
    assert_eq!(
        tree_post_slugs(paths.iter().copied()),
        slugs(&["alpha", "zeta"])
    );
}

#[test]
fn reconcile_config_round_trips_as_the_publish_target() {
    let config = ReconcileConfig {
        repository: "chrishiguto/chris".into(),
        branch: "main".into(),
    };
    let json = serde_json::to_string(&config).expect("serializes");
    let back: ReconcileConfig = serde_json::from_str(&json).expect("deserializes");
    assert_eq!(back, config);
}

#[test]
fn parse_head_ref_extracts_the_sha() {
    let json = serde_json::json!({
        "ref": "refs/heads/main",
        "object": { "sha": "abc123", "type": "commit" }
    });
    assert_eq!(parse_head_ref(&json).unwrap(), "abc123");
    assert!(parse_head_ref(&serde_json::json!({ "object": {} })).is_err());
    assert!(parse_head_ref(&serde_json::json!({ "message": "Not Found" })).is_err());
}

#[test]
fn parse_tree_listing_yields_post_slugs_from_blobs() {
    let json = serde_json::json!({
        "truncated": false,
        "tree": [
            { "path": "content/blog/hello/index.mdx", "type": "blob" },
            { "path": "content/blog/hello/components.rs", "type": "blob" },
            // a directory named like a post source must not count as one
            { "path": "content/blog/weird/index.mdx", "type": "tree" },
            { "path": "app/src/lib.rs", "type": "blob" },
        ]
    });
    assert_eq!(parse_tree_listing(&json).unwrap(), slugs(&["hello"]));
}

#[test]
fn a_truncated_tree_listing_is_an_error() {
    // a truncated listing would silently retire every post it omitted
    let json = serde_json::json!({ "truncated": true, "tree": [] });
    assert!(parse_tree_listing(&json).is_err());
}

#[test]
fn a_tree_response_without_a_tree_array_is_an_error() {
    let json = serde_json::json!({ "message": "Not Found" });
    assert!(parse_tree_listing(&json).is_err());
}

// --- publish outcome ---

#[test]
fn outcome_summarizes_success_and_carried_failures() {
    let out = PublishOutcome::new(3, 0, 0, true, &[]);
    assert!(out.ok);
    assert_eq!(out.summary, "reconciled: 3 posts published");

    let out = PublishOutcome::new(1, 0, 0, true, &[]);
    assert!(out.ok);
    assert_eq!(out.summary, "reconciled: 1 post published");

    let diag = content::Diagnostic {
        message: "unknown component <OrbitSimulatr>".to_string(),
        file: Some("content/blog/broken/index.mdx".to_string()),
        line: Some(3),
        column: None,
    };
    let out = PublishOutcome::new(2, 1, 1, true, &[diag]);
    assert!(!out.ok);
    assert_eq!(
        out.summary,
        "1 post failed validation (previous versions kept); \
         content/blog/broken/index.mdx: unknown component <OrbitSimulatr>"
    );
}

/// A failed post with no previous version to carry drops from the index;
/// the summary must not claim it was kept.
#[test]
fn outcome_does_not_claim_kept_for_dropped_posts() {
    let diag = content::Diagnostic {
        message: "unknown component <Nope>".to_string(),
        file: Some("content/blog/new-and-broken/index.mdx".to_string()),
        line: None,
        column: None,
    };
    let out = PublishOutcome::new(2, 2, 1, true, &[diag]);
    assert!(!out.ok);
    assert!(
        out.summary
            .contains("previous versions kept where available"),
        "{}",
        out.summary
    );
}

/// KV can flip and the purge still fail — readers see stale pages, so the
/// outcome must not be `ok`, and the summary must say so.
#[test]
fn outcome_is_not_ok_when_the_purge_fails() {
    let out = PublishOutcome::new(2, 0, 0, false, &[]);
    assert!(!out.ok);
    assert_eq!(
        out.summary,
        "reconciled: 2 posts published; cache purge failed — pages may be stale"
    );

    // A validation failure and a purge failure both surface.
    let out = PublishOutcome::new(1, 1, 1, false, &[]);
    assert!(!out.ok);
    assert!(
        out.summary.contains("cache purge failed"),
        "{}",
        out.summary
    );
    assert!(out.summary.contains("failed validation"), "{}", out.summary);
}

/// Debt from a failed purge widens the next scope — without it, a same-HEAD
/// reconcile would diff to nothing while the cache is stale.
#[test]
fn purge_scope_merges_debt_into_the_stale_tags_deduped() {
    let stale = vec!["post:a".to_string(), "views".to_string()];
    let debt = vec!["post:b".to_string(), "views".to_string()];
    assert_eq!(
        purge_scope(stale, Some(debt)),
        ["post:a", "post:b", "views"]
    );
}

#[test]
fn purge_scope_of_an_unchanged_reconcile_is_the_debt_alone() {
    let debt = vec!["post:a".to_string()];
    assert_eq!(purge_scope(vec![], Some(debt)), ["post:a"]);
    assert!(purge_scope(vec![], Some(vec![])).is_empty());
}

/// An unreadable debt ledger could be hiding any tag, so the scope widens to
/// the whole site: over-purge, never staleness.
#[test]
fn purge_scope_escalates_unreadable_debt_to_a_site_purge() {
    assert_eq!(purge_scope(vec![], None), ["site"]);
    assert_eq!(
        purge_scope(vec!["post:a".to_string()], None),
        ["post:a", "site"]
    );
}

#[test]
fn failure_description_is_concise_and_counts_diagnostics() {
    let diag = |message: &str| content::Diagnostic {
        message: message.to_string(),
        file: Some("content/blog/hello/index.mdx".to_string()),
        line: Some(3),
        column: None,
    };
    let one = failure_description(&[diag("unknown component <OrbitSimulatr>")]);
    assert_eq!(
        one,
        "content/blog/hello/index.mdx: unknown component <OrbitSimulatr>"
    );
    let two = failure_description(&[diag("first problem"), diag("second problem")]);
    assert_eq!(
        two,
        "2 errors; first: content/blog/hello/index.mdx: first problem"
    );
}

// --- GitHub API request shapes ---

#[test]
fn github_urls_pin_repo_path_and_sha() {
    assert_eq!(
        contents_url("chrishiguto/chris", "hello", "abc123"),
        "https://api.github.com/repos/chrishiguto/chris/contents/content/blog/hello/index.mdx?ref=abc123"
    );
    assert_eq!(
        head_ref_url("chrishiguto/chris", "main"),
        "https://api.github.com/repos/chrishiguto/chris/git/ref/heads/main"
    );
    assert_eq!(
        tree_url("chrishiguto/chris", "abc123"),
        "https://api.github.com/repos/chrishiguto/chris/git/trees/abc123?recursive=1"
    );
}

#[test]
fn publish_body_deserializes_into_the_reconcile_config() {
    // The /publish body is a ReconcileConfig; an unknown field (a legacy
    // `sha`, say) is ignored so wire drift can't 400 the call.
    let config: ReconcileConfig = serde_json::from_str(
        r#"{"sha":"abc123","repository":"chrishiguto/chris","branch":"main"}"#,
    )
    .expect("a body with an extra field still deserializes");
    assert_eq!(config.repository, "chrishiguto/chris");
    assert_eq!(config.branch, "main");

    // A body missing `branch` fails to parse — the handler turns that into a 400.
    assert!(
        serde_json::from_str::<ReconcileConfig>(r#"{"repository":"chrishiguto/chris"}"#).is_err()
    );
}

// --- manifest ---
// app::manifest() only yields the vocabulary if the linker kept app's
// inventory registrations in this binary.

#[test]
fn manifest_exposes_the_real_app_vocabulary() {
    let manifest = app::manifest();
    let names: Vec<_> = manifest.names().collect();
    assert!(
        names.contains(&"Callout") && names.contains(&"Counter"),
        "expected the app vocabulary, got {names:?}"
    );
}
