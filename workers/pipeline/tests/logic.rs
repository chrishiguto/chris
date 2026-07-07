//! Native tests for the pipeline worker's pure decision logic:
//! classification, reconcile vocabulary, and status building are plain
//! functions — the wasm shim stays thin.

use pipeline::{
    classify, code_push_description, contents_url, dispatch_payload, dispatch_url,
    failure_description, head_ref_url, parse_head_ref, parse_tree_listing, post_slug,
    purge_payloads, purge_url, reconcile_description, source_path, status_payload, statuses_url,
    tree_post_slugs, tree_url, verify_publish_auth, verify_signature, PublishRequest, PushClass,
    PushCommit, PushEvent, ReconcileConfig, StatusState, PURGE_FILES_LIMIT, STATUS_CONTEXT,
    WORKFLOW_FILE,
};

fn commit(added: &[&str], modified: &[&str], removed: &[&str]) -> PushCommit {
    let paths = |list: &[&str]| list.iter().map(|p| p.to_string()).collect();
    PushCommit {
        added: paths(added),
        modified: paths(modified),
        removed: paths(removed),
    }
}

fn slugs(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

// --- webhook payload ---

#[test]
fn push_event_deserializes_the_fields_the_decision_needs() {
    let event: PushEvent =
        serde_json::from_str(include_str!("fixtures/push.json")).expect("payload should parse");
    assert_eq!(event.git_ref, "refs/heads/main");
    assert_eq!(event.after, "6113728f27ae82c7b1a177c8d03f9e96e0adf246");
    assert!(!event.deleted);
    assert_eq!(event.repository.full_name, "chrishiguto/chris");
    assert_eq!(event.repository.default_branch, "main");
    assert!(event.is_default_branch());
    assert_eq!(event.commits.len(), 1);
    assert_eq!(event.commits[0].added, ["content/blog/hello/index.mdx"]);
    assert_eq!(event.commits[0].modified, ["docs/DOCS_INDEX.md"]);
}

#[test]
fn branch_pushes_are_not_default_branch() {
    let mut event: PushEvent =
        serde_json::from_str(include_str!("fixtures/push.json")).expect("payload should parse");
    event.git_ref = "refs/heads/feature/prd-1".to_string();
    assert!(!event.is_default_branch());
}

// --- signature verification ---

// GitHub's documented webhook validation example, so the implementation is
// checked against the spec rather than against itself.
const DOC_SECRET: &str = "It's a Secret to Everybody";
const DOC_BODY: &[u8] = b"Hello, World!";
const DOC_SIGNATURE: &str =
    "sha256=757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e17";

#[test]
fn valid_signature_verifies() {
    assert!(verify_signature(DOC_SECRET, DOC_BODY, Some(DOC_SIGNATURE)));
}

#[test]
fn tampered_body_and_wrong_secret_fail() {
    assert!(!verify_signature(
        DOC_SECRET,
        b"Hello, World?",
        Some(DOC_SIGNATURE)
    ));
    assert!(!verify_signature(
        "wrong secret",
        DOC_BODY,
        Some(DOC_SIGNATURE)
    ));
}

#[test]
fn missing_or_malformed_signature_headers_fail() {
    assert!(!verify_signature(DOC_SECRET, DOC_BODY, None));
    assert!(!verify_signature(DOC_SECRET, DOC_BODY, Some("")));
    assert!(!verify_signature(DOC_SECRET, DOC_BODY, Some("sha256=")));
    assert!(!verify_signature(DOC_SECRET, DOC_BODY, Some("sha256=zz")));
    assert!(!verify_signature(
        DOC_SECRET,
        DOC_BODY,
        Some("sha256=757107")
    ));
    // sha1= is the legacy header; only sha256 is accepted
    assert!(!verify_signature(
        DOC_SECRET,
        DOC_BODY,
        Some("sha1=deadbeef")
    ));
    // multibyte input must not panic the hex decoder
    assert!(!verify_signature(DOC_SECRET, DOC_BODY, Some("sha256=éé")));
}

// --- path predicates ---

#[test]
fn post_slug_matches_only_post_sources() {
    assert_eq!(post_slug("content/blog/hello/index.mdx"), Some("hello"));
    assert_eq!(post_slug("content/blog/hello/notes.txt"), None);
    assert_eq!(post_slug("content/blog/a/b/index.mdx"), None);
    assert_eq!(post_slug("content/blog/index.mdx"), None);
    assert_eq!(post_slug("docs/index.mdx"), None);
}

#[test]
fn source_path_round_trips_the_slug() {
    assert_eq!(source_path("hello"), "content/blog/hello/index.mdx");
    assert_eq!(post_slug(&source_path("hello")), Some("hello"));
}

// --- classification ---

#[test]
fn content_only_push_takes_the_fast_path() {
    let class = classify(&[commit(
        &["content/blog/hello/index.mdx"],
        &["content/blog/older/index.mdx"],
        &[],
    )]);
    assert_eq!(class, PushClass::ContentOnly);
}

#[test]
fn removed_post_source_still_takes_the_fast_path() {
    // A removal is a content change like any other: the reconcile rebuilds
    // to HEAD, where the post no longer exists.
    let class = classify(&[commit(&[], &[], &["content/blog/hello/index.mdx"])]);
    assert_eq!(class, PushClass::ContentOnly);
}

#[test]
fn docs_only_push_is_ignored() {
    let class = classify(&[commit(&["README.md"], &["docs/DOCS_INDEX.md"], &[])]);
    assert_eq!(class, PushClass::Ignore);
}

#[test]
fn non_post_content_files_alone_are_ignored() {
    let class = classify(&[commit(&[], &["content/blog/hello/notes.txt"], &[])]);
    assert_eq!(class, PushClass::Ignore);
}

#[test]
fn mixed_push_takes_the_code_path_and_counts_touched_posts() {
    let class = classify(&[commit(
        &["content/blog/hello/index.mdx"],
        &["app/src/post.rs"],
        &[],
    )]);
    assert_eq!(class, PushClass::Code { touched_posts: 1 });
}

#[test]
fn colocated_components_count_as_code() {
    // Per-post Rust must ride the deploy path even under content/
    let class = classify(&[commit(&["content/blog/hello/components.rs"], &[], &[])]);
    assert_eq!(class, PushClass::Code { touched_posts: 0 });
}

#[test]
fn build_defining_files_count_as_code() {
    for path in [
        "Cargo.toml",
        "Cargo.lock",
        "justfile",
        "wrangler.toml",
        "app/style/main.css",
        "crates/registry/Cargo.toml",
        "workers/pipeline/wrangler.toml",
        ".github/workflows/deploy.yml",
    ] {
        assert_eq!(
            classify(&[commit(&[], &[path], &[])]),
            PushClass::Code { touched_posts: 0 },
            "{path} should classify as code"
        );
    }
}

#[test]
fn a_slug_touched_by_many_commits_counts_once() {
    // added, modified, and removed across one push is one touched post —
    // the count only feeds the status message; the reconcile rebuilds to
    // HEAD regardless of what the delta says.
    let class = classify(&[
        commit(&["content/blog/hello/index.mdx"], &[], &[]),
        commit(&[], &["app/src/lib.rs"], &["content/blog/hello/index.mdx"]),
    ]);
    assert_eq!(class, PushClass::Code { touched_posts: 1 });
}

#[test]
fn empty_push_is_ignored() {
    assert_eq!(classify(&[]), PushClass::Ignore);
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
fn reconcile_config_round_trips_as_the_trigger_payload() {
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

// --- commit status building ---

#[test]
fn reconcile_description_reports_success_and_carried_failures() {
    let (state, description) = reconcile_description(3, 0, &[]);
    assert_eq!(state, StatusState::Success);
    assert_eq!(description, "reconciled: 3 posts published");

    let (state, description) = reconcile_description(1, 0, &[]);
    assert_eq!(state, StatusState::Success);
    assert_eq!(description, "reconciled: 1 post published");

    let diag = content::Diagnostic {
        message: "unknown component <OrbitSimulatr>".to_string(),
        file: Some("content/blog/broken/index.mdx".to_string()),
        line: Some(3),
        column: None,
    };
    let (state, description) = reconcile_description(2, 1, &[diag]);
    assert_eq!(state, StatusState::Failure);
    assert_eq!(
        description,
        "1 post failed validation (previous versions kept); \
         content/blog/broken/index.mdx: unknown component <OrbitSimulatr>"
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

#[test]
fn code_push_description_counts_the_content_changes() {
    assert_eq!(
        code_push_description(3),
        "code push: 3 content changes publish after the CI deploy"
    );
    assert_eq!(
        code_push_description(0),
        "code push: publish reconciles after the CI deploy"
    );
}

#[test]
fn status_payload_carries_the_context_and_clamps_the_description() {
    let payload = status_payload(StatusState::Success, "published hello");
    let json: serde_json::Value = serde_json::from_str(&payload).expect("valid json");
    assert_eq!(json["state"], "success");
    assert_eq!(json["context"], STATUS_CONTEXT);
    assert_eq!(json["description"], "published hello");
    assert_eq!(STATUS_CONTEXT, "blog/publish");

    // the Commit Status API caps descriptions at 140 characters
    let long = "x".repeat(200);
    let payload = status_payload(StatusState::Failure, &long);
    let json: serde_json::Value = serde_json::from_str(&payload).expect("valid json");
    let description = json["description"].as_str().expect("string");
    assert_eq!(description.chars().count(), 140);
    assert!(description.ends_with('…'));
}

#[test]
fn status_states_serialize_lowercase() {
    for (state, expected) in [
        (StatusState::Pending, "pending"),
        (StatusState::Success, "success"),
        (StatusState::Failure, "failure"),
        (StatusState::Error, "error"),
    ] {
        let json: serde_json::Value =
            serde_json::from_str(&status_payload(state, "x")).expect("valid json");
        assert_eq!(json["state"], expected);
    }
}

// --- GitHub API request shapes ---

#[test]
fn github_urls_pin_repo_path_and_sha() {
    assert_eq!(
        contents_url("chrishiguto/chris", "hello", "abc123"),
        "https://api.github.com/repos/chrishiguto/chris/contents/content/blog/hello/index.mdx?ref=abc123"
    );
    assert_eq!(
        statuses_url("chrishiguto/chris", "abc123"),
        "https://api.github.com/repos/chrishiguto/chris/statuses/abc123"
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

// --- workflow dispatch (the code path) ---

#[test]
fn dispatch_url_targets_the_publish_workflow() {
    assert_eq!(
        dispatch_url("chrishiguto/chris"),
        "https://api.github.com/repos/chrishiguto/chris/actions/workflows/publish.yml/dispatches"
    );
    assert_eq!(WORKFLOW_FILE, "publish.yml");
}

#[test]
fn dispatch_payload_carries_branch_ref_and_commit_sha() {
    let json: serde_json::Value =
        serde_json::from_str(&dispatch_payload("main", "abc123")).expect("valid json");
    assert_eq!(json["ref"], "main");
    assert_eq!(json["inputs"]["sha"], "abc123");
}

// --- /publish auth ---

#[test]
fn matching_bearer_token_authenticates() {
    assert!(verify_publish_auth("s3cret", Some("Bearer s3cret")));
}

#[test]
fn missing_or_wrong_tokens_are_rejected() {
    assert!(!verify_publish_auth("s3cret", None));
    assert!(!verify_publish_auth("s3cret", Some("")));
    assert!(!verify_publish_auth("s3cret", Some("s3cret")));
    assert!(!verify_publish_auth("s3cret", Some("Bearer wrong")));
    assert!(!verify_publish_auth("s3cret", Some("Bearer s3cret extra")));
    // only CI calls this endpoint; the exact scheme it sends is the contract
    assert!(!verify_publish_auth("s3cret", Some("bearer s3cret")));
}

#[test]
fn publish_request_carries_repository_and_branch() {
    // CI also sends `sha`; serde ignores it — a reconcile targets HEAD.
    let request: PublishRequest = serde_json::from_str(
        r#"{"sha":"abc123","repository":"chrishiguto/chris","branch":"main"}"#,
    )
    .expect("valid request");
    assert_eq!(request.repository, "chrishiguto/chris");
    assert_eq!(request.branch, "main");

    // a caller predating the branch field parses (and is rejected by the
    // handler with a clear message rather than a serde error)
    let legacy: PublishRequest =
        serde_json::from_str(r#"{"sha":"abc123","repository":"chrishiguto/chris"}"#)
            .expect("legacy request still parses");
    assert!(legacy.branch.is_empty());
}

// --- manifest (pins the inventory linkage anchor for this consumer:
// app::manifest() only yields the vocabulary if the linker kept app's
// registrations in the binary that calls it) ---

#[test]
fn manifest_exposes_the_real_app_vocabulary() {
    let manifest = app::manifest();
    let names: Vec<_> = manifest.names().collect();
    assert!(
        names.contains(&"Callout") && names.contains(&"Counter"),
        "expected the app vocabulary, got {names:?}"
    );
}

// --- cache purge requests ---

#[test]
fn purge_url_targets_the_zone_purge_endpoint() {
    assert_eq!(
        purge_url("zone123"),
        "https://api.cloudflare.com/client/v4/zones/zone123/purge_cache"
    );
}

#[test]
fn purge_payloads_prefix_the_site_origin_onto_each_path() {
    let paths = vec!["/".to_string(), "/posts/hello".to_string()];
    assert_eq!(
        purge_payloads("https://blog.example.com", &paths),
        vec![r#"{"files":["https://blog.example.com/","https://blog.example.com/posts/hello"]}"#]
    );
}

/// A configured `SITE_ORIGIN` with a trailing slash must not produce
/// `https://host//posts/…` — purge-by-URL matches URLs exactly.
#[test]
fn purge_payloads_normalize_a_trailing_slash_origin() {
    let paths = vec!["/posts/hello".to_string()];
    assert_eq!(
        purge_payloads("https://blog.example.com/", &paths),
        vec![r#"{"files":["https://blog.example.com/posts/hello"]}"#]
    );
}

/// The purge-by-URL API caps each request at 30 files; a bigger publish
/// (many tags) must split into several requests, not silently truncate.
#[test]
fn purge_payloads_chunk_to_the_api_file_limit() {
    let paths: Vec<String> = (0..PURGE_FILES_LIMIT + 1)
        .map(|n| format!("/posts/p{n}"))
        .collect();
    let payloads = purge_payloads("https://blog.example.com", &paths);
    assert_eq!(payloads.len(), 2);
    let files = |payload: &str| {
        serde_json::from_str::<serde_json::Value>(payload).unwrap()["files"]
            .as_array()
            .unwrap()
            .len()
    };
    assert_eq!(files(&payloads[0]), PURGE_FILES_LIMIT);
    assert_eq!(files(&payloads[1]), 1);
}

#[test]
fn no_paths_means_no_purge_requests() {
    assert!(purge_payloads("https://blog.example.com", &[]).is_empty());
}
