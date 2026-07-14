//! Stylesheet contracts: the split bundle holds together, every class the
//! components render keeps a selector (the kitchen-sink fixture proves the
//! converse — every selector has markup), the token mechanism keeps theming
//! client-side, and the shell head loads in no-flash order. How things look
//! is deliberately untested here — that's the kitchen-sink read in both
//! themes.
#![cfg(feature = "ssr")]

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use app::app::{shell, GOOGLE_FONTS_URL, THEME_SCRIPT, THEME_STORAGE_KEY};
use app::render::render_document;
use leptos::prelude::{LeptosOptions, RenderHtml};

mod common;

/// The bundle as Tailwind builds it: main.css with each local `@import`
/// inlined where it sits, so cascade order matches production. A sheet in
/// `style/` that main.css never imports (or imports through a line this
/// parser misses) fails loudly instead of silently dropping out of the
/// guards.
fn stylesheet() -> String {
    let style = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/style"));
    let mut unimported: BTreeSet<String> = fs::read_dir(style)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.ends_with(".css") && name != "main.css")
        .collect();
    let bundled = fs::read_to_string(style.join("main.css"))
        .unwrap()
        .lines()
        .map(|line| {
            match line
                .strip_prefix("@import \"./")
                .and_then(|rest| rest.strip_suffix("\";"))
            {
                Some(sheet) => {
                    assert!(unimported.remove(sheet), "`{sheet}` imported twice");
                    fs::read_to_string(style.join(sheet)).unwrap()
                }
                None => line.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        unimported.is_empty(),
        "sheets main.css never imports: {unimported:?}"
    );
    bundled
}

fn shell_html() -> String {
    use leptos::prelude::provide_context;

    // The router needs the RequestUrl leptos_axum would provide per-request.
    let options = LeptosOptions::builder().output_name("chris").build();
    common::ssr(
        || provide_context(leptos_router::location::RequestUrl::new("/")),
        move || shell(options),
    )
}

/// True when `selector` appears followed by a non-ident character, so
/// `.tag` never rides on `.tag-hash`'s rule.
fn has_selector(css: &str, selector: &str) -> bool {
    css.match_indices(selector).any(|(at, _)| {
        !css[at + selector.len()..]
            .chars()
            .next()
            .is_some_and(|next| next.is_ascii_alphanumeric() || next == '-' || next == '_')
    })
}

// A new AST node type without theming must fail here, not on a live page.
// `pre` only ever renders inside the CodeBlock panel, which owns it.
#[test]
fn stylesheet_styles_every_rendered_element() {
    let css = stylesheet();
    let prose = [
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
        "blockquote",
        "ul",
        "ol",
        "li",
        "img",
        "hr",
    ]
    .map(|element| format!(".post-body {element}"));
    for selector in prose.iter().map(String::as_str).chain([".code-block pre"]) {
        assert!(
            has_selector(&css, selector),
            "no `{selector}` selector in the stylesheet"
        );
    }
}

// The class half of the markup↔stylesheet join: every hand-written class
// the components emit keeps a rule. Tailwind utilities are generated, not
// authored, and stay out; callout-note/tip ride the base `.callout` rule.
#[test]
fn every_component_class_is_styled() {
    let css = stylesheet();
    for class in [
        // listing
        ".post-list",
        ".post-row",
        ".post-row-top",
        ".post-row-title",
        ".post-row-lead",
        ".post-row-meta",
        ".post-row-desc",
        ".plink",
        // post
        ".post",
        ".post-body",
        ".post-meta",
        ".meta-sep",
        ".post-path",
        ".path-root",
        ".path-tilde",
        ".path-sep",
        ".path-here",
        ".post-tags",
        ".tag",
        ".tag-hash",
        // callouts and errors
        ".callout",
        ".callout-label",
        ".callout-title",
        ".callout-body",
        ".callout-warning",
        ".callout-danger",
        ".component-error",
        // code panels
        ".code-block",
        ".code-bar",
        ".code-copy",
        // chrome
        ".site-nav",
        ".nav-mark",
        ".nav-tilde",
        ".nav-link",
        ".theme-toggle",
        ".glyph-sun",
        ".glyph-moon",
        ".site-footer",
        ".konami-toast",
        ".konami-cursor",
        // filter + about
        ".filter-empty",
        ".contact-link",
        ".link-arrow",
    ] {
        assert!(
            has_selector(&css, class),
            "no `{class}` rule in the stylesheet"
        );
    }
}

// Selectors keyed to state the server never renders: the active filter
// pill, the route-aware nav underline, and the glyph swap following the
// effective scheme (explicit `data-theme` wins, system preference decides
// otherwise). No SSR assertion can see these, so their CSS side pins here.
#[test]
fn state_flipped_selectors_are_styled() {
    let css = stylesheet();
    for selector in [
        "a.tag.tag-active",
        ".nav-link[aria-current=\"page\"]",
        ":root[data-theme=\"dark\"] .theme-toggle .glyph-moon",
        ":root:not([data-theme]) .theme-toggle .glyph-moon",
    ] {
        assert!(css.contains(selector), "no `{selector}` in the stylesheet");
    }
    assert!(
        css.contains("(prefers-color-scheme: dark)"),
        "the unset state must follow the system preference"
    );
}

// The token mechanism that keeps theming client-side and the cache one
// response per URL: every color declared exactly once via `light-dark()`
// in oklch, mapped to utilities through `@theme inline`, with `data-theme`
// flipping `color-scheme` over the system default.
#[test]
fn color_tokens_are_oklch_light_dark_declared_once() {
    let css = stylesheet();
    assert!(
        css.contains("@theme inline"),
        "color tokens must map to utilities through `@theme inline`"
    );
    assert!(
        css.contains("color-scheme: light dark"),
        "the root must opt into both schemes for `light-dark()` to resolve"
    );
    for selector in ["[data-theme=\"light\"]", "[data-theme=\"dark\"]"] {
        assert!(
            css.contains(selector),
            "`{selector}` must flip `color-scheme` at the CSS level"
        );
    }
    for token in [
        "--surface:",
        "--surface-2:",
        "--surface-3:",
        "--ink:",
        "--ink-2:",
        "--ink-3:",
        "--line:",
        "--line-2:",
        "--accent:",
        "--accent-2:",
        "--accent-subtle:",
        "--danger:",
    ] {
        let declarations: Vec<&str> = css
            .lines()
            .filter(|line| line.trim_start().starts_with(token))
            .collect();
        assert_eq!(
            declarations.len(),
            1,
            "`{token}` must be declared exactly once, found {declarations:?}"
        );
        assert!(
            declarations[0].contains("light-dark(") && declarations[0].contains("oklch("),
            "`{token}` must hold both modes via `light-dark()` in oklch: {declarations:?}"
        );
    }
}

// Geist + Geist Mono come from Google Fonts with `display=swap`, and the
// families the stylesheet's stacks name are the ones the shell loads.
#[test]
fn fonts_load_from_google_with_swap() {
    let css = stylesheet();
    assert!(
        css.contains("\"Geist\"") && css.contains("\"Geist Mono\""),
        "font stacks must lead with Geist / Geist Mono"
    );
    for part in ["family=Geist", "family=Geist+Mono", "display=swap"] {
        assert!(
            GOOGLE_FONTS_URL.contains(part),
            "Google Fonts URL missing `{part}`"
        );
    }
    let html = shell_html();
    assert!(
        html.contains("rel=\"preconnect\" href=\"https://fonts.googleapis.com\""),
        "missing preconnect to fonts.googleapis.com: {html}"
    );
    assert!(
        html.contains("rel=\"preconnect\" href=\"https://fonts.gstatic.com\"")
            && html.contains("crossorigin"),
        "missing anonymous preconnect to fonts.gstatic.com: {html}"
    );
    assert!(
        // Attribute values render entity-escaped.
        html.contains(&GOOGLE_FONTS_URL.replace('&', "&amp;")),
        "the fonts stylesheet must be linked in the head: {html}"
    );
}

// A stored explicit theme is re-applied by a blocking inline script ahead
// of every stylesheet, so the first paint can't flash the wrong theme — and
// the script is a constant, so the served HTML never varies.
#[test]
fn stored_theme_is_applied_pre_paint() {
    // The script is a hand-written literal; this pins its key to the
    // constant the toggle island persists under.
    let getter = format!("localStorage.getItem(\"{THEME_STORAGE_KEY}\")");
    for part in [getter.as_str(), "\"light\"", "\"dark\"", "dataset.theme"] {
        assert!(
            THEME_SCRIPT.contains(part),
            "theme script missing `{part}`: {THEME_SCRIPT}"
        );
    }
    let html = shell_html();
    let script = html
        .find(THEME_SCRIPT)
        .expect("the inline theme script must ship in the shell head");
    let stylesheet = html
        .find("rel=\"stylesheet\"")
        .expect("the shell links stylesheets");
    assert!(
        script < stylesheet,
        "the theme script must run before any stylesheet loads"
    );
}

// One real post exercising every node type and Callout kind — if it renders,
// the whole vocabulary has markup for the selectors above to style.
#[test]
fn kitchen_sink_fixture_exercises_every_node_type() {
    let source = include_str!("../../content/blog/kitchen-sink/index.mdx");
    let doc = content::parse_validated(source, "test.mdx", &registry::manifest())
        .expect("kitchen-sink post must validate against the live manifest");
    let html = common::strip_markers(render_document(&doc, "kitchen-sink").to_html());
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
        "// note",
        "// tip",
        "// warning",
        "// danger",
        "<span class=\"code-lang\">rust</span>",
        "<span class=\"code-lang\">code</span>",
        "class=\"code-copy\"",
        "<leptos-island",
    ] {
        assert!(html.contains(needle), "kitchen sink missing {needle}");
    }
    assert!(
        !html.contains("component-error"),
        "no component may fail dispatch: {html}"
    );
}
