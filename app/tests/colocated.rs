//! Co-location contract: build.rs-discovered `content/blog/{slug}/components.rs`
//! components join the compiled registry and their post renders.
#![cfg(feature = "ssr")]

use app::render::render_document;
use leptos::prelude::RenderHtml;

mod common;

#[test]
fn colocated_components_join_the_manifest() {
    let manifest = registry::manifest();
    let stages = manifest
        .get("DeployStages")
        .expect("DeployStages discovered from content/blog/ci-code-path/components.rs");
    assert!(stages.prop("total").is_some_and(|p| p.required));
    assert!(!stages.accepts_children);
}

#[test]
fn the_ci_code_path_post_renders_its_colocated_island() {
    let source = include_str!("../../content/blog/ci-code-path/index.mdx");
    let doc = content::parse_validated(source, "test.mdx", &registry::manifest())
        .expect("post must validate against the live manifest");
    let html = common::strip_markers(render_document(&doc, "ci-code-path").to_html());
    assert!(
        html.contains("<leptos-island"),
        "DeployStages island missing: {html}"
    );
    assert!(
        !html.contains("component-error"),
        "no component may fail dispatch: {html}"
    );
}
