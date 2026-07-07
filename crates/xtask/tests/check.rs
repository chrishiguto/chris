//! Integration tests over fixture content trees (PRD testing decisions):
//! `check` passes on the valid tree and reports every problem — parse,
//! validation, date shape, missing index.mdx — on the invalid one.

use std::path::PathBuf;

fn fixture_tree(name: &str) -> PathBuf {
    [
        env!("CARGO_MANIFEST_DIR"),
        "tests/fixtures",
        name,
        "content/blog",
    ]
    .iter()
    .collect()
}

#[test]
fn manifest_exposes_the_real_app_vocabulary() {
    let manifest = app::manifest();
    for name in ["Callout", "Counter"] {
        assert!(
            manifest.get(name).is_some(),
            "app's inventory registrations must reach the CLI; missing {name}"
        );
    }
}

#[test]
fn valid_tree_checks_clean() {
    let posts = xtask::check_tree(&fixture_tree("valid"), &app::manifest()).unwrap();
    let slugs: Vec<_> = posts.iter().map(|p| p.slug.as_str()).collect();
    assert_eq!(slugs, ["first-post", "second-post"]);
    assert_eq!(posts[0].document.frontmatter.title, "First post");
}

#[test]
fn invalid_tree_reports_every_problem() {
    let diags = xtask::check_tree(&fixture_tree("invalid"), &app::manifest()).unwrap_err();

    let for_file = |needle: &str| {
        diags
            .iter()
            .find(|d| d.file.as_deref().is_some_and(|f| f.contains(needle)))
            .unwrap_or_else(|| panic!("no diagnostic for {needle}: {diags:?}"))
    };

    let bad_component = for_file("bad-component/index.mdx");
    assert!(
        bad_component.message.contains("did you mean `<Callout>`"),
        "message: {}",
        bad_component.message
    );
    assert!(
        bad_component.line.is_some(),
        "validation diagnostics carry lines"
    );

    let bad_date = for_file("bad-date/index.mdx");
    assert!(
        bad_date.message.contains("YYYY-MM-DD"),
        "message: {}",
        bad_date.message
    );

    let no_index = for_file("no-index");
    assert!(
        no_index.message.contains("index.mdx"),
        "message: {}",
        no_index.message
    );
}

#[test]
fn the_repos_real_content_tree_checks_clean() {
    let root: PathBuf = [env!("CARGO_MANIFEST_DIR"), "../../content/blog"]
        .iter()
        .collect();
    let posts = xtask::check_tree(&root, &app::manifest()).unwrap();
    assert!(!posts.is_empty(), "the repo ships real posts");
}
