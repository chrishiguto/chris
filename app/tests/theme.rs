//! Theme guards: the stylesheet is data these tests pin down —
//! every element the AST renderer emits must have a `.post` selector, the
//! tokens must be oklch with light and dark values, and every font the CSS
//! references must exist on disk and load without layout shift.
//! Run with `cargo test -p app --features ssr`.
#![cfg(feature = "ssr")]

use std::fs;
use std::path::Path;

use app::app::{shell, PRELOADED_FONTS};
use app::render::render_document;
use leptos::prelude::{LeptosOptions, RenderHtml};

mod common;

/// `main.css` (foundations) plus every local sheet it `@import`s — the guards
/// below pin the combined sheet, wherever a rule lives. Deriving the list from
/// the `@import` lines means a dropped or added import shifts what the guards
/// see, instead of silently diverging from what Tailwind bundles.
fn stylesheet() -> String {
    let style = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/style"));
    let main = fs::read_to_string(style.join("main.css")).unwrap();
    let imported: String = main
        .lines()
        .filter_map(|line| line.strip_prefix("@import \"./")?.strip_suffix("\";"))
        .map(|sheet| fs::read_to_string(style.join(sheet)).unwrap())
        .collect();
    main + &imported
}

fn assets_dir() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/assets"))
}

// Every element `render_node` can emit needs a deliberate prose style; a new
// AST node type added without theming must fail here, not on a live page.
// Matching requires a selector boundary after the element name so `.post-body
// p` can never be satisfied by `.post-body pre`.
#[test]
fn stylesheet_styles_every_rendered_element() {
    let css = stylesheet();
    for element in [
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "p",
        "a",
        "strong",
        "code",
        "pre",
        "blockquote",
        "ul",
        "ol",
        "li",
        "img",
        "hr",
    ] {
        let styled = [" ", ",", ":", "\n"]
            .iter()
            .any(|boundary| css.contains(&format!(".post-body {element}{boundary}")));
        assert!(
            styled,
            "no `.post-body {element}` selector in the stylesheet"
        );
    }
}

#[test]
fn theme_tokens_are_oklch_with_light_and_dark() {
    let css = stylesheet();
    assert!(
        css.contains("@theme inline"),
        "tokens must map through `@theme inline` so dark mode can flip them"
    );
    assert!(css.contains("oklch("), "palette must be oklch");
    assert!(
        css.contains("prefers-color-scheme: dark"),
        "dark values missing"
    );
    assert!(
        css.contains("color-scheme"),
        "`color-scheme` keeps UA widgets/scrollbars in sync with the theme"
    );
}

#[test]
fn callout_and_error_surfaces_are_styled() {
    let css = stylesheet();
    for class in [
        ".callout ",
        ".callout-note",
        ".callout-tip",
        ".callout-warning",
        ".callout-danger",
        ".callout-title",
        ".component-error",
    ] {
        assert!(
            css.contains(class),
            "no `{class}` styling in the stylesheet"
        );
    }
}

// The theme defines the listing pages' treatment; the pages themselves
// only need markup.
#[test]
fn listing_and_tag_surfaces_are_styled() {
    let css = stylesheet();
    for class in [".post-list", ".post-tags", ".tag"] {
        assert!(
            css.contains(class),
            "no `{class}` styling in the stylesheet"
        );
    }
}

// Zero-layout-shift contract: every face is self-hosted, uses
// `font-display: optional` (fallback never swaps mid-read), and the critical
// faces are preloaded so they reliably make the first paint.
#[test]
fn fonts_are_self_hosted_and_never_shift_layout() {
    let css = stylesheet();
    let faces: Vec<&str> = css
        .split("url(\"/fonts/")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .collect();
    assert!(
        faces.len() >= 3,
        "expected self-hosted faces, got {faces:?}"
    );
    for file in &faces {
        assert!(
            assets_dir().join("fonts").join(file).is_file(),
            "{file} referenced in the stylesheet but missing from app/assets/fonts"
        );
    }
    let face_count = css.matches("@font-face").count();
    assert_eq!(
        css.matches("font-display: optional").count(),
        face_count,
        "every @font-face must use `font-display: optional`"
    );
    for preload in PRELOADED_FONTS {
        let file = preload.trim_start_matches("/fonts/");
        assert!(
            faces.contains(&file),
            "{preload} is preloaded but has no @font-face"
        );
    }
}

// The theme QA page: one long real post exercising every AST node type and
// every Callout kind, validated against the live manifest and rendered
// through real dispatch — if it renders, the whole vocabulary has markup for
// the selectors above to style.
#[test]
fn kitchen_sink_fixture_exercises_every_node_type() {
    let source = include_str!("../../content/blog/kitchen-sink/index.mdx");
    let doc = content::parse_validated(source, "test.mdx", &registry::manifest())
        .expect("kitchen-sink post must validate against the live manifest");
    let html = common::strip_markers(render_document(&doc).to_html());
    for needle in [
        "<h2",
        "<h3",
        "<h4",
        "<h5",
        "<h6",
        "<em>",
        "<strong>",
        "<code",
        "<pre",
        "<a href",
        "<img",
        "<ol start",
        "<ul>",
        "<blockquote>",
        "<hr",
        "<br",
        "<kbd>",
        "class=\"post-tags\"",
        "callout callout-note",
        "callout callout-tip",
        "callout callout-warning",
        "callout callout-danger",
        "<leptos-island",
    ] {
        assert!(html.contains(needle), "kitchen sink missing {needle}");
    }
    assert!(
        !html.contains("component-error"),
        "no component may fail dispatch: {html}"
    );
}

#[test]
fn shell_preloads_critical_fonts() {
    use leptos::prelude::{provide_context, Owner};

    // The shell mounts the full App; the router needs the request URL that
    // leptos_axum would provide per-request.
    let owner = Owner::new();
    owner.set();
    provide_context(leptos_router::location::RequestUrl::new("/"));
    let options = LeptosOptions::builder().output_name("chris").build();
    let html = shell(options).to_html();
    for font in PRELOADED_FONTS {
        let link = format!(
            "<link rel=\"preload\" href=\"{font}\" as=\"font\" type=\"font/woff2\" crossorigin=\"anonymous\""
        );
        assert!(html.contains(&link), "missing preload for {font}: {html}");
    }
}
