use content::IndexEntry;
use content::{ComponentSpec, Manifest, PropSpec, PropType};
use publish::{check, plan, PostSource};

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
fn plan_writes_posts_and_rewrites_index_newest_first() {
    let parsed = check(
        &[
            post("older", "Older", "2026-01-01", "One."),
            post("newer", "Newer", "2026-03-01", "Two."),
        ],
        &manifest(),
    )
    .unwrap();
    let plan = plan(Vec::new(), &parsed, &[]).unwrap();

    let keys: Vec<_> = plan.writes.iter().map(|w| w.key.as_str()).collect();
    assert_eq!(keys, ["post:older", "post:newer", "index"]);
    assert!(plan.deletes.is_empty());

    let slugs: Vec<_> = plan.index.iter().map(|e| e.slug.as_str()).collect();
    assert_eq!(slugs, ["newer", "older"]);

    // Post payloads must round-trip through the schema-versioned Document.
    let doc = content::Document::from_json(&plan.writes[0].value).unwrap();
    assert_eq!(doc.frontmatter.title, "Older");

    // The index write is the serialized rewritten index.
    let index: Vec<IndexEntry> = serde_json::from_str(&plan.writes[2].value).unwrap();
    assert_eq!(index, plan.index);
}

#[test]
fn plan_merges_into_previous_index_and_removes() {
    let prev = vec![
        IndexEntry {
            slug: "gone".into(),
            title: "Gone".into(),
            date: "2026-04-01".into(),
            description: None,
            tags: vec![],
            draft: false,
        },
        IndexEntry {
            slug: "kept".into(),
            title: "Kept".into(),
            date: "2026-02-01".into(),
            description: None,
            tags: vec![],
            draft: false,
        },
        IndexEntry {
            slug: "updated".into(),
            title: "Old title".into(),
            date: "2026-01-01".into(),
            description: None,
            tags: vec![],
            draft: false,
        },
    ];
    let parsed = check(
        &[post("updated", "New title", "2026-05-01", "Fresh.")],
        &manifest(),
    )
    .unwrap();
    let plan = plan(prev, &parsed, &["gone".into()]).unwrap();

    assert_eq!(plan.deletes, ["post:gone"]);
    let titles: Vec<_> = plan
        .index
        .iter()
        .map(|e| (e.slug.as_str(), e.title.as_str()))
        .collect();
    assert_eq!(titles, [("updated", "New title"), ("kept", "Kept")]);
}

#[test]
fn plan_keeps_drafts_in_the_index() {
    let source = PostSource {
        slug: "wip".into(),
        file: "content/blog/wip/index.mdx".into(),
        source: "---\ntitle: WIP\ndate: 2026-06-01\ndraft: true\n---\n\nSoon.\n".into(),
    };
    let parsed = check(&[source], &manifest()).unwrap();
    let plan = plan(Vec::new(), &parsed, &[]).unwrap();
    assert!(plan.index[0].draft, "drafts are stored, filtered at render");
}

// The publish half of the draft flip: pushing `draft: true → false`
// must rewrite the index entry as published AND purge the listing surfaces
// the post now appears on (plus its own URL and tag pages), so stale cached
// listings cannot keep hiding it for the TTL.
#[test]
fn plan_publishes_a_draft_flip_and_purges_listings() {
    let prev = vec![IndexEntry {
        slug: "wip".into(),
        title: "WIP".into(),
        date: "2026-06-01".into(),
        description: None,
        tags: vec!["rust".into()],
        draft: true,
    }];
    let source = PostSource {
        slug: "wip".into(),
        file: "content/blog/wip/index.mdx".into(),
        source: "---\ntitle: WIP\ndate: 2026-06-01\ntags: [rust]\n---\n\nDone.\n".into(),
    };
    let parsed = check(&[source], &manifest()).unwrap();
    let plan = plan(prev, &parsed, &[]).unwrap();

    assert!(
        !plan.index[0].draft,
        "the flipped post must be listed: {:?}",
        plan.index
    );
    assert!(
        plan.writes.iter().any(|w| w.key == "post:wip"),
        "the flipped post must be rewritten"
    );
    let expected = [
        "/",
        "/posts",
        "/posts/wip",
        "/rss.xml",
        "/sitemap.xml",
        "/tags",
        "/tags/rust",
    ];
    assert_eq!(plan.purge, expected);
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

#[test]
fn plan_purges_listings_feeds_and_touched_tag_pages() {
    // "updated" drops the old-tag it had in the previous index and gains
    // new-tag; "gone" is removed and had gone-tag. All three tag pages (and
    // both post URLs) must purge, alongside the fixed listing/feed set.
    let prev = vec![
        IndexEntry {
            slug: "updated".into(),
            title: "Old".into(),
            date: "2026-01-01".into(),
            description: None,
            tags: vec!["old-tag".into()],
            draft: false,
        },
        IndexEntry {
            slug: "gone".into(),
            title: "Gone".into(),
            date: "2026-02-01".into(),
            description: None,
            tags: vec!["gone-tag".into()],
            draft: false,
        },
        IndexEntry {
            slug: "untouched".into(),
            title: "Untouched".into(),
            date: "2026-03-01".into(),
            description: None,
            tags: vec!["quiet-tag".into()],
            draft: false,
        },
    ];
    let source = PostSource {
        slug: "updated".into(),
        file: "content/blog/updated/index.mdx".into(),
        source: "---\ntitle: New\ndate: 2026-05-01\ntags: [new-tag]\n---\n\nx\n".into(),
    };
    let parsed = check(&[source], &manifest()).unwrap();
    let plan = plan(prev, &parsed, &["gone".into()]).unwrap();

    let expected = [
        "/",
        "/posts",
        "/posts/gone",
        "/posts/updated",
        "/rss.xml",
        "/sitemap.xml",
        "/tags",
        "/tags/gone-tag",
        "/tags/new-tag",
        "/tags/old-tag",
    ];
    assert_eq!(plan.purge, expected);
    assert!(
        !plan.purge.iter().any(|p| p.contains("quiet-tag")),
        "untouched posts' tag pages must not purge: {:?}",
        plan.purge
    );
}

#[test]
fn plan_orders_same_date_posts_by_slug() {
    let parsed = check(
        &[
            post("zeta", "Z", "2026-01-01", "One."),
            post("alpha", "A", "2026-01-01", "Two."),
        ],
        &manifest(),
    )
    .unwrap();
    let plan = plan(Vec::new(), &parsed, &[]).unwrap();
    let slugs: Vec<_> = plan.index.iter().map(|e| e.slug.as_str()).collect();
    assert_eq!(slugs, ["alpha", "zeta"]);
}
