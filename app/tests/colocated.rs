//! Pins the co-location contract: build.rs discovers
//! `content/blog/{slug}/components.rs` files, their components join the
//! compiled registry, and the post shipping them validates and renders.
//! Run with `cargo test -p app --features ssr`.
#![cfg(feature = "ssr")]

use app::post::render_document;
use leptos::prelude::RenderHtml;

#[test]
fn colocated_components_join_the_manifest() {
    let manifest = registry::manifest();
    let stages = manifest
        .get("DeployStages")
        .expect("DeployStages discovered from content/blog/ci-code-path/components.rs");
    assert!(stages.prop("total").is_some_and(|p| p.required));
    assert!(!stages.accepts_children);
}

// The mixed-commit demo's local half: the post referencing its co-located
// island validates against the live manifest and renders through dispatch.
#[test]
fn the_ci_code_path_post_renders_its_colocated_island() {
    let source = include_str!("../../content/blog/ci-code-path/index.mdx");
    let doc = content::parse_validated(source, &registry::manifest())
        .expect("post must validate against the live manifest");
    let html = render_document(&doc).to_html().replace("<!>", "");
    assert!(
        html.contains("<leptos-island"),
        "DeployStages island missing: {html}"
    );
    assert!(
        !html.contains("component-error"),
        "no component may fail dispatch: {html}"
    );
}
