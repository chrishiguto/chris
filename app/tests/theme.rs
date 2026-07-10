//! Theme guards: every rendered element gets a `.post` selector, tokens are
//! oklch declared once via `light-dark()`, and fonts load from Google Fonts.
#![cfg(feature = "ssr")]

use std::fs;
use std::path::Path;

use app::app::{shell, GOOGLE_FONTS_URL, THEME_SCRIPT};
use app::components::Header;
use app::render::render_document;
use leptos::prelude::{LeptosOptions, RenderHtml};
use leptos::view;

mod common;

/// main.css plus every local sheet it `@import`s — derived from the import lines
/// so the guards track what Tailwind bundles instead of silently diverging.
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

/// Every `.rs` source Tailwind scans for utility classes (`@source "../src"`
/// plus the co-located post components discovered by app's build.rs).
fn utility_sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, files: &mut Vec<(String, String)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, files);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push((
                    path.display().to_string(),
                    fs::read_to_string(&path).unwrap(),
                ));
            }
        }
    }
    let mut files = Vec::new();
    for root in [
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src")),
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../content/blog")),
    ] {
        walk(root, &mut files);
    }
    files
}

fn shell_html() -> String {
    use leptos::prelude::{provide_context, Owner};

    // The router needs the RequestUrl leptos_axum would provide per-request.
    let owner = Owner::new();
    owner.set();
    provide_context(leptos_router::location::RequestUrl::new("/"));
    let options = LeptosOptions::builder().output_name("chris").build();
    shell(options).to_html()
}

// A new AST node type without theming must fail here, not on a live page.
// The boundary characters stop `.post-body p` matching `.post-body pre`.
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

// The token contract from the design-system PRD: every color is oklch,
// declared exactly once via `light-dark()`, with `data-theme` flipping
// `color-scheme` so an explicit choice overrides the system preference.
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
    // "Tonal subtle": no pure white or black surfaces/text anywhere.
    for forbidden in ["#fff", "#000", "oklch(100%", "oklch(0% 0 0)"] {
        assert!(
            !css.to_lowercase().contains(forbidden),
            "pure `{forbidden}` has no place in the warm palette"
        );
    }
}

#[test]
fn theme_scales_match_the_design() {
    let css = stylesheet();
    for (token, value) in [
        ("--text-xs", "0.75rem"),
        ("--text-sm", "0.8125rem"),
        ("--text-base", "1rem"),
        ("--text-lg", "1.125rem"),
        ("--text-xl", "1.375rem"),
        ("--text-2xl", "1.75rem"),
        ("--text-3xl", "2.375rem"),
        ("--text-4xl", "3.25rem"),
        ("--leading-tight", "1.15"),
        ("--leading-snug", "1.4"),
        ("--leading-normal", "1.55"),
        ("--leading-relaxed", "1.75"),
        ("--tracking-tight", "-0.02em"),
        ("--tracking-wide", "0.06em"),
        ("--ease-out", "cubic-bezier(0.25, 0.46, 0.45, 0.94)"),
        ("--ease-in-out", "cubic-bezier(0.65, 0, 0.35, 1)"),
        ("--ease-out-expo", "cubic-bezier(0.16, 1, 0.3, 1)"),
    ] {
        assert!(
            css.contains(&format!("{token}: {value}")),
            "`{token}` must be re-valued to `{value}`"
        );
    }
    assert!(
        css.contains("--animate-fade-up:") && css.contains("@keyframes fade-up"),
        "the design's `fade-up` reveal must exist as an animation token"
    );
    for shadow in ["--shadow-sm:", "--shadow-md:"] {
        let line = css
            .lines()
            .find(|line| line.trim_start().starts_with(shadow))
            .unwrap_or_else(|| panic!("`{shadow}` must be re-valued warm"));
        assert!(
            line.contains("light-dark("),
            "`{shadow}` must carry both modes via `light-dark()`: {line}"
        );
    }
}

// Grep-clean: the v1 token vocabulary is gone from the stylesheet and from
// every source file Tailwind scans for utilities.
#[test]
fn stale_v1_tokens_are_gone() {
    let stale = [
        "surface-raised",
        "ink-muted",
        "ink-faint",
        "font-serif",
        "font-heading",
        "--hue-",
        "Lora",
        "Libre Baskerville",
        "IBM Plex Mono",
    ];
    let css = stylesheet();
    for token in stale {
        assert!(!css.contains(token), "stale `{token}` in the stylesheet");
    }
    for (path, source) in utility_sources() {
        for token in stale {
            assert!(!source.contains(token), "stale `{token}` in {path}");
        }
    }
}

// The design's reading experience: 17px / 65ch / 1.75, plus the Slice 2
// article chrome (mono meta row, back link, tag-pill hash glyph).
#[test]
fn post_prose_reads_at_the_design_measure() {
    let css = stylesheet();
    for (property, why) in [
        ("font-size: 1.0625rem", "article body must read at 17px"),
        ("line-height: 1.75", "article body must lead at 1.75"),
        ("max-width: 65ch", "prose must cap at the 65ch measure"),
    ] {
        assert!(css.contains(property), "{why} (`{property}`)");
    }
    for class in [".post-meta", ".back-link", ".tag-hash"] {
        assert!(
            css.contains(class),
            "no `{class}` styling in the stylesheet"
        );
    }
}

// Callouts are code-comment asides (design Callout): a mono `// kind` label
// in the family hue, two hue families with severity read through fill
// intensity — note/tip accent-subtle, warning transparent, danger tinted —
// and no left-border stripes, ever.
#[test]
fn callout_and_error_surfaces_are_styled() {
    let css = stylesheet();
    for class in [".callout-label", ".callout-title"] {
        assert!(
            css.contains(class),
            "no `{class}` styling in the stylesheet"
        );
    }
    let base = rule_body(&css, ".callout {");
    assert!(
        base.contains("background-color: var(--color-accent-subtle)"),
        "note/tip must fill with accent-subtle: {base}"
    );
    assert!(
        !base.contains("border-inline-start"),
        "no left-border stripes on callouts: {base}"
    );
    let warning = rule_body(&css, ".callout-warning {");
    assert!(
        warning.contains("--callout-hue: var(--color-danger)"),
        "warning label must read danger: {warning}"
    );
    assert!(
        warning.contains("background-color: transparent"),
        "warning must hold the fill back: {warning}"
    );
    let danger = rule_body(&css, ".callout-danger {");
    assert!(
        danger.contains("background-color: color-mix(in oklab, var(--color-danger)"),
        "danger must fill danger-tinted: {danger}"
    );
    let error = rule_body(&css, ".component-error {");
    assert!(
        error.contains("var(--color-danger)"),
        "component errors must stay danger-jarring: {error}"
    );
}

/// The declarations of the first rule opened by `opener` (up to its `}`).
fn rule_body<'a>(css: &'a str, opener: &str) -> &'a str {
    let start = css
        .find(opener)
        .unwrap_or_else(|| panic!("no `{opener}` rule in the stylesheet"));
    let len = css[start..]
        .find('}')
        .unwrap_or_else(|| panic!("unterminated `{opener}` rule"));
    &css[start..start + len]
}

#[test]
fn listing_and_tag_surfaces_are_styled() {
    let css = stylesheet();
    for class in [
        ".post-list",
        ".post-row",
        ".post-row-lead",
        ".post-row-meta",
        ".post-row-desc",
        ".post-tags",
        ".tag",
        ".plink",
    ] {
        assert!(
            css.contains(class),
            "no `{class}` styling in the stylesheet"
        );
    }
}

// The PostRow hover contract (design PostRow.jsx): title turns accent, the
// arrow slides in; the slide transform is off under reduced motion, and the
// description truncates to a single line.
#[test]
fn post_rows_hover_and_truncate_per_the_design() {
    let css = stylesheet();
    for rule in [
        ".post-row:hover .post-row-title",
        ".post-row:hover .post-row-lead",
        "text-overflow: ellipsis",
    ] {
        assert!(css.contains(rule), "no `{rule}` in the stylesheet");
    }
    let transform_disabled = css
        .split("@media (prefers-reduced-motion: reduce)")
        .skip(1)
        .any(|block| {
            let scope = block.split("@media").next().unwrap_or(block);
            scope.contains(".post-row-lead") && scope.contains("transform: none")
        });
    assert!(
        transform_disabled,
        "the arrow slide must be disabled under prefers-reduced-motion"
    );
}

// Geist + Geist Mono come from Google Fonts with `display=swap`; the v1
// self-hosted faces, `@font-face` blocks, and preloads are gone.
#[test]
fn fonts_load_from_google_with_swap() {
    let css = stylesheet();
    assert!(
        !css.contains("@font-face") && !css.contains("/fonts/"),
        "self-hosted faces must be gone from the stylesheet"
    );
    assert!(
        css.contains("\"Geist\"") && css.contains("\"Geist Mono\""),
        "font stacks must lead with Geist / Geist Mono"
    );
    let fonts_dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/fonts"));
    assert!(
        !fonts_dir.exists(),
        "app/assets/fonts must be deleted along with the @font-face blocks"
    );
    for part in [
        "fonts.googleapis.com/css2",
        "family=Geist",
        "family=Geist+Mono",
        "display=swap",
    ] {
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
    assert!(
        !html.contains("as=\"font\"") && !html.contains("/fonts/"),
        "font preloads went out with the self-hosted faces: {html}"
    );
}

// ADR-0011: a stored explicit theme is re-applied by a blocking inline
// script ahead of every stylesheet, so the first paint can't flash the wrong
// theme — and the script is a constant, so the served HTML never varies.
#[test]
fn stored_theme_is_applied_pre_paint() {
    for part in [
        "localStorage.getItem(\"chris-theme\")",
        "\"light\"",
        "\"dark\"",
        "dataset.theme",
    ] {
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

// The toggle island SSRs both glyphs (ADR-0011): CSS picks the visible one,
// so the button can't flash a stale icon before hydration.
#[test]
fn theme_toggle_ssrs_both_glyphs_as_an_island() {
    let html = common::ssr(|| {}, || view! { <Header /> });
    assert!(
        html.contains("<leptos-island"),
        "the toggle must hydrate as an island: {html}"
    );
    for needle in [
        "class=\"theme-toggle\"",
        "aria-label=\"toggle theme\"",
        "glyph-moon",
        "glyph-sun",
        "☾",
        "☀",
    ] {
        assert!(html.contains(needle), "toggle missing `{needle}`: {html}");
    }
}

// The glyphs follow the effective scheme in CSS alone: an explicit
// `data-theme` wins, the system preference decides otherwise; hover rotates
// the glyph per the design's motion rules unless reduced motion is set.
#[test]
fn theme_toggle_glyphs_follow_the_effective_scheme() {
    let css = stylesheet();
    for selector in [
        ".theme-toggle",
        ".theme-toggle .glyph-sun",
        ":root[data-theme=\"dark\"] .theme-toggle .glyph-moon",
        ":root:not([data-theme]) .theme-toggle .glyph-moon",
    ] {
        assert!(css.contains(selector), "no `{selector}` in the stylesheet");
    }
    assert!(
        css.contains("(prefers-color-scheme: dark)"),
        "the unset state must follow the system preference"
    );
    assert!(
        css.contains("rotate(-20deg)"),
        "hover must rotate the glyph per the design's motion rules"
    );
}

// The house link move: current-color text, underline grows from the left.
#[test]
fn base_links_grow_an_underline_from_the_left() {
    let css = stylesheet();
    assert!(
        css.contains("background-size: 0% 1px"),
        "base links must start with a zero-width underline"
    );
    assert!(
        css.contains("background-size: 100% 1px"),
        "hover must grow the underline to full width"
    );
    assert!(
        css.contains("font-family: var(--font-sans)"),
        "the body reads in the sans face"
    );
}

// One real post exercising every node type and Callout kind — if it renders,
// the whole vocabulary has markup for the selectors above to style.
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
        "// note",
        "// tip",
        "// warning",
        "// danger",
        "<leptos-island",
    ] {
        assert!(html.contains(needle), "kitchen sink missing {needle}");
    }
    assert!(
        !html.contains("component-error"),
        "no component may fail dispatch: {html}"
    );
}
