//! Native tests for the pipeline worker's pure decision logic (PRD testing
//! decisions: classification, publish-set computation, pending handling and
//! status building are plain functions — the wasm shim stays thin).

use pipeline::{
    classify, contents_url, failure_description, merge_pending, pending_description, post_path,
    post_slug, status_payload, statuses_url, success_description, verify_signature, PendingEntry,
    PublishSet, PushClass, PushCommit, PushEvent, StatusState, PENDING_KEY, STATUS_CONTEXT,
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

// --- signature verification (user story 34) ---

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
fn post_path_round_trips_the_slug() {
    assert_eq!(post_path("hello"), "content/blog/hello/index.mdx");
    assert_eq!(post_slug(&post_path("hello")), Some("hello"));
}

// --- classification (user stories 3, 5, 30, 31) ---

#[test]
fn content_only_push_takes_the_fast_path() {
    let class = classify(&[commit(
        &["content/blog/hello/index.mdx"],
        &["content/blog/older/index.mdx"],
        &[],
    )]);
    assert_eq!(
        class,
        PushClass::ContentOnly(PublishSet {
            changed: slugs(&["hello", "older"]),
            removed: vec![],
        })
    );
}

#[test]
fn removed_post_dir_lands_in_the_removed_set() {
    let class = classify(&[commit(&[], &[], &["content/blog/hello/index.mdx"])]);
    assert_eq!(
        class,
        PushClass::ContentOnly(PublishSet {
            changed: vec![],
            removed: slugs(&["hello"]),
        })
    );
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
fn mixed_push_takes_the_code_path_with_the_publish_set() {
    let class = classify(&[commit(
        &["content/blog/hello/index.mdx"],
        &["app/src/post.rs"],
        &[],
    )]);
    assert_eq!(
        class,
        PushClass::Code(PublishSet {
            changed: slugs(&["hello"]),
            removed: vec![],
        })
    );
}

#[test]
fn colocated_components_count_as_code() {
    // ADR-0004: per-post Rust must ride the deploy path even under content/
    let class = classify(&[commit(&["content/blog/hello/components.rs"], &[], &[])]);
    assert_eq!(class, PushClass::Code(PublishSet::default()));
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
            PushClass::Code(PublishSet::default()),
            "{path} should classify as code"
        );
    }
}

#[test]
fn later_commits_supersede_earlier_ones_per_slug() {
    // added then removed within one push → the final state is removed
    let class = classify(&[
        commit(&["content/blog/hello/index.mdx"], &[], &[]),
        commit(&[], &[], &["content/blog/hello/index.mdx"]),
    ]);
    assert_eq!(
        class,
        PushClass::ContentOnly(PublishSet {
            changed: vec![],
            removed: slugs(&["hello"]),
        })
    );

    // removed then re-added → changed
    let class = classify(&[
        commit(&[], &[], &["content/blog/hello/index.mdx"]),
        commit(&["content/blog/hello/index.mdx"], &[], &[]),
    ]);
    assert_eq!(
        class,
        PushClass::ContentOnly(PublishSet {
            changed: slugs(&["hello"]),
            removed: vec![],
        })
    );
}

#[test]
fn empty_push_is_ignored() {
    assert_eq!(classify(&[]), PushClass::Ignore);
}

// --- pending stash (user story 31; PRD KV schema `pending`) ---

fn entry(slug: &str, sha: &str, removed: bool) -> PendingEntry {
    PendingEntry {
        slug: slug.to_string(),
        sha: sha.to_string(),
        removed,
    }
}

#[test]
fn merge_pending_appends_new_entries() {
    let set = PublishSet {
        changed: slugs(&["hello"]),
        removed: slugs(&["gone"]),
    };
    assert_eq!(
        merge_pending(vec![], &set, "abc123"),
        vec![
            entry("hello", "abc123", false),
            entry("gone", "abc123", true)
        ]
    );
}

#[test]
fn merge_pending_supersedes_older_entries_for_the_same_slug() {
    let prev = vec![
        entry("hello", "old000", false),
        entry("other", "old000", false),
    ];
    let set = PublishSet {
        changed: vec![],
        removed: slugs(&["hello"]),
    };
    assert_eq!(
        merge_pending(prev, &set, "new111"),
        vec![
            entry("other", "old000", false),
            entry("hello", "new111", true)
        ]
    );
}

#[test]
fn pending_entries_round_trip_as_the_kv_payload() {
    let stash = vec![entry("hello", "abc123", false)];
    let json = serde_json::to_string(&stash).expect("serializes");
    let back: Vec<PendingEntry> = serde_json::from_str(&json).expect("deserializes");
    assert_eq!(back, stash);
    // entries written before the `removed` field existed must still read
    let legacy: Vec<PendingEntry> =
        serde_json::from_str(r#"[{"slug":"hello","sha":"abc123"}]"#).expect("legacy reads");
    assert_eq!(legacy, stash);
    assert_eq!(PENDING_KEY, "pending");
}

// --- commit status building (user story 12; ADR-0007 amendment) ---

#[test]
fn success_description_lists_published_and_removed_slugs() {
    let set = PublishSet {
        changed: slugs(&["hello", "world"]),
        removed: slugs(&["gone"]),
    };
    assert_eq!(
        success_description(&set),
        "published hello, world; removed gone"
    );
    assert_eq!(
        success_description(&PublishSet {
            changed: slugs(&["hello"]),
            removed: vec![],
        }),
        "published hello"
    );
}

#[test]
fn failure_description_is_concise_and_counts_diagnostics() {
    let diag = |message: &str| content_parser::Diagnostic {
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
fn pending_description_counts_the_parked_set() {
    let set = PublishSet {
        changed: slugs(&["hello", "world"]),
        removed: slugs(&["gone"]),
    };
    assert_eq!(
        pending_description(&set),
        "code push: 3 content changes parked for CI publish"
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
}

// --- manifest (pins the inventory linkage anchor, like blog-cli) ---

#[test]
fn manifest_exposes_the_real_app_vocabulary() {
    let manifest = pipeline::manifest();
    let names: Vec<_> = manifest.names().collect();
    assert!(
        names.contains(&"Callout") && names.contains(&"Counter"),
        "expected the app vocabulary, got {names:?}"
    );
}
