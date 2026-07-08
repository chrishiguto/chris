use content::IndexEntry;
use content::{ComponentSpec, Manifest, PropSpec, PropType};
use publish::{check, check_each, snapshot, CarriedPost, PostSource};

fn manifest() -> Manifest {
    Manifest {
        components: vec![ComponentSpec {
            name: "Callout".into(),
            props: vec![PropSpec {
                name: "kind".into(),
                ty: PropType::String,
                required: true,
            }],
            accepts_children: true,
        }],
    }
}

fn post(slug: &str, title: &str, date: &str, body: &str) -> PostSource {
    PostSource {
        slug: slug.into(),
        file: format!("content/blog/{slug}/index.mdx"),
        source: format!("---\ntitle: {title}\ndate: {date}\n---\n\n{body}\n"),
    }
}

fn entry(slug: &str, title: &str, date: &str, tags: &[&str], draft: bool) -> IndexEntry {
    IndexEntry {
        slug: slug.into(),
        title: title.into(),
        date: date.into(),
        description: None,
        tags: tags.iter().map(|t| t.to_string()).collect(),
        draft,
    }
}

#[test]
fn check_parses_a_valid_tree() {
    let posts = [
        post("a", "A", "2026-01-01", "Hello."),
        post(
            "b",
            "B",
            "2026-02-01",
            "<Callout kind=\"note\">Hi.</Callout>",
        ),
    ];
    let parsed = check(&posts, &manifest()).unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].slug, "a");
    assert_eq!(parsed[0].document.frontmatter.title, "A");
}

#[test]
fn check_collects_diagnostics_across_files() {
    let posts = [
        post("a", "A", "2026-01-01", "<Nope />"),
        post("b", "B", "2026-02-01", "<Callout>missing kind</Callout>"),
    ];
    let diags = check(&posts, &manifest()).unwrap_err();
    assert_eq!(diags.len(), 2);
    assert_eq!(diags[0].file.as_deref(), Some("content/blog/a/index.mdx"));
    assert_eq!(diags[1].file.as_deref(), Some("content/blog/b/index.mdx"));
}

#[test]
fn check_rejects_a_malformed_date() {
    let posts = [post("a", "A", "someday", "Hello.")];
    let diags = check(&posts, &manifest()).unwrap_err();
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("YYYY-MM-DD"), "{}", diags[0]);
    assert_eq!(diags[0].file.as_deref(), Some("content/blog/a/index.mdx"));
}

#[test]
fn check_rejects_a_non_slug_tag() {
    let source = PostSource {
        slug: "a".into(),
        file: "content/blog/a/index.mdx".into(),
        source: "---\ntitle: A\ndate: 2026-01-01\ntags: [ok-tag, \"Not A Slug\"]\n---\n\nx\n"
            .into(),
    };
    let diags = check(&[source], &manifest()).unwrap_err();
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("Not A Slug"), "{}", diags[0]);
    assert!(diags[0].message.contains("lowercase"), "{}", diags[0]);
}

/// The per-post gate partitions instead of failing the batch.
#[test]
fn check_each_passes_and_fails_posts_independently() {
    let posts = [
        post("good", "Good", "2026-01-01", "Fine."),
        post("bad", "Bad", "2026-02-01", "<Nope />"),
    ];
    let results = check_each(&posts, &manifest());
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].as_ref().unwrap().slug, "good");
    let diags = results[1].as_ref().unwrap_err();
    assert_eq!(diags[0].file.as_deref(), Some("content/blog/bad/index.mdx"));
}

#[test]
fn snapshot_writes_posts_then_index_under_snapshot_keys() {
    let parsed = check(
        &[
            post("older", "Older", "2026-01-01", "One."),
            post("newer", "Newer", "2026-03-01", "Two."),
        ],
        &manifest(),
    )
    .unwrap();
    let plan = snapshot(&parsed, Vec::new(), "abc123").unwrap();

    let keys: Vec<_> = plan.post_writes.iter().map(|w| w.key.as_str()).collect();
    assert_eq!(
        keys,
        ["snapshot:abc123:post:older", "snapshot:abc123:post:newer"]
    );
    assert_eq!(plan.index_write.key, "snapshot:abc123:index");

    let slugs: Vec<_> = plan.index.iter().map(|e| e.slug.as_str()).collect();
    assert_eq!(slugs, ["newer", "older"]);

    // Payloads must round-trip through the schema-versioned Document.
    let doc = content::Document::from_json(&plan.post_writes[0].value).unwrap();
    assert_eq!(doc.frontmatter.title, "Older");

    let index: Vec<IndexEntry> = serde_json::from_str(&plan.index_write.value).unwrap();
    assert_eq!(index, plan.index);
}

/// The index is exactly checked + carried posts: anything the previous
/// snapshot knew that is absent from the rebuild is retired — there is no
/// explicit removed list.
#[test]
fn snapshot_retires_posts_absent_from_the_rebuild() {
    let parsed = check(
        &[post("updated", "New title", "2026-05-01", "Fresh.")],
        &manifest(),
    )
    .unwrap();
    let plan = snapshot(&parsed, Vec::new(), "abc123").unwrap();

    let titles: Vec<_> = plan
        .index
        .iter()
        .map(|e| (e.slug.as_str(), e.title.as_str()))
        .collect();
    assert_eq!(titles, [("updated", "New title")]);
}

/// A failed post rides in unchanged: previous entry, previous payload,
/// keyed under the new snapshot.
#[test]
fn snapshot_carries_failed_posts_previous_versions() {
    let parsed = check(&[post("good", "Good", "2026-05-01", "Fine.")], &manifest()).unwrap();
    let carried = vec![CarriedPost {
        entry: entry("broken", "Broken", "2026-01-01", &[], false),
        payload: r#"{"stored":"payload"}"#.into(),
    }];
    let plan = snapshot(&parsed, carried, "abc123").unwrap();

    let slugs: Vec<_> = plan.index.iter().map(|e| e.slug.as_str()).collect();
    assert_eq!(slugs, ["good", "broken"]);
    let write = plan
        .post_writes
        .iter()
        .find(|w| w.key == "snapshot:abc123:post:broken")
        .expect("carried payload must be written under the new snapshot");
    assert_eq!(write.value, r#"{"stored":"payload"}"#);
}

#[test]
fn snapshot_keeps_drafts_in_the_index() {
    let source = PostSource {
        slug: "wip".into(),
        file: "content/blog/wip/index.mdx".into(),
        source: "---\ntitle: WIP\ndate: 2026-06-01\ndraft: true\n---\n\nSoon.\n".into(),
    };
    let parsed = check(&[source], &manifest()).unwrap();
    let plan = snapshot(&parsed, Vec::new(), "abc123").unwrap();
    assert!(plan.index[0].draft, "drafts are stored, filtered at render");
}

#[test]
fn kv_writes_serialize_to_the_wrangler_bulk_shape() {
    let parsed = check(&[post("a", "A", "2026-01-01", "Hi.")], &manifest()).unwrap();
    let plan = snapshot(&parsed, Vec::new(), "abc123").unwrap();
    let json = serde_json::to_value(&plan.index_write).unwrap();
    assert_eq!(json["key"], "snapshot:abc123:index");
    assert!(json["value"].is_string());
    assert_eq!(json.as_object().unwrap().len(), 2);
}

#[test]
fn snapshot_orders_same_date_posts_by_slug() {
    let parsed = check(
        &[
            post("zeta", "Z", "2026-01-01", "One."),
            post("alpha", "A", "2026-01-01", "Two."),
        ],
        &manifest(),
    )
    .unwrap();
    let plan = snapshot(&parsed, Vec::new(), "abc123").unwrap();
    let slugs: Vec<_> = plan.index.iter().map(|e| e.slug.as_str()).collect();
    assert_eq!(slugs, ["alpha", "zeta"]);
}

/// The slug is a directory name the parser never sees; check gates it.
#[test]
fn check_rejects_an_invalid_slug() {
    let bad = |slug: &str| PostSource {
        slug: slug.into(),
        file: format!("content/blog/{slug}/index.mdx"),
        source: "---\ntitle: T\ndate: 2026-01-01\n---\n\nx\n".into(),
    };
    for slug in ["Hello", "a_b", "v1.0", "1st", "-x", ""] {
        let diags = check(&[bad(slug)], &manifest()).unwrap_err();
        assert_eq!(diags.len(), 1, "slug {slug:?}: {diags:#?}");
        assert!(
            diags[0].message.contains("lowercase slug"),
            "slug {slug:?}: {}",
            diags[0]
        );
    }
    assert!(check(&[bad("ok-slug-2")], &manifest()).is_ok());
}
