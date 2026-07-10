//! Design-system guards, stylesheet side: tokens are oklch `light-dark()`
//! declared once, every rendered element has a `.post-body` selector (the
//! kitchen-sink fixture proves the converse — every selector has markup),
//! and each surface keeps its design treatment. HTML shapes live in the
//! per-surface suites (chrome, listing, render, about).
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
    use leptos::prelude::provide_context;

    // The router needs the RequestUrl leptos_axum would provide per-request.
    let options = LeptosOptions::builder().output_name("chris").build();
    common::ssr(
        || provide_context(leptos_router::location::RequestUrl::new("/")),
        move || shell(options),
    )
}

// A new AST node type without theming must fail here, not on a live page.
// The boundary characters stop `.post-body p` matching `.post-body pre`.
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
        let styled = [" ", ",", ":", "\n"]
            .iter()
            .any(|boundary| css.contains(&format!("{selector}{boundary}")));
        assert!(styled, "no `{selector}` selector in the stylesheet");
    }
}

// The token contract: every color is oklch,
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
        "the `fade-up` reveal must exist as an animation token"
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

// The reading experience: 17px / 65ch / 1.75, plus the article chrome
// (mono meta row, back link, tag-pill hash glyph).
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
    for class in [".post-meta", ".meta-sep", ".back-link", ".tag-hash"] {
        assert!(
            css.contains(class),
            "no `{class}` styling in the stylesheet"
        );
    }
}

// Callouts are code-comment asides: a mono `// kind` label
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
    let base = rule_body(&css, &[".callout"]);
    assert!(
        base.contains("background-color: var(--color-accent-subtle)"),
        "note/tip must fill with accent-subtle: {base}"
    );
    assert!(
        !base.contains("border-inline-start"),
        "no left-border stripes on callouts: {base}"
    );
    let label = rule_body(&css, &[".callout-label"]);
    assert!(
        label.contains("color: var(--color-accent)"),
        "note/tip labels must read accent: {label}"
    );
    let danger_labels = rule_body(
        &css,
        &[
            ".callout-warning .callout-label",
            ".callout-danger .callout-label",
        ],
    );
    assert!(
        danger_labels.contains("color: var(--color-danger)"),
        "warning/danger labels must read danger: {danger_labels}"
    );
    let warning = rule_body(&css, &[".callout-warning"]);
    assert!(
        warning.contains("background-color: transparent"),
        "warning must hold the fill back: {warning}"
    );
    let danger = rule_body(&css, &[".callout-danger"]);
    assert!(
        danger.contains("background-color: color-mix(in oklab, var(--color-danger)"),
        "danger must fill danger-tinted: {danger}"
    );
    let error = rule_body(&css, &[".component-error"]);
    assert!(
        error.contains("var(--color-danger)"),
        "component errors must stay danger-jarring: {error}"
    );
}

// Code blocks are chromed panels: the wrapper owns the
// fill/border/radius and clips its corners, the bar names the language in
// faint mono over a hairline, the pre scrolls sideways at 13px/1.65, and
// the copy button warms to accent on hover.
#[test]
fn code_block_chrome_is_styled() {
    let css = stylesheet();
    let block = rule_body(&css, &[".code-block"]);
    for needle in [
        "background-color: var(--color-surface-2)",
        "border: 1px solid var(--color-line)",
        "overflow: hidden",
        "font-family: var(--font-mono)",
    ] {
        assert!(block.contains(needle), "missing `{needle}`: {block}");
    }
    let bar = rule_body(&css, &[".code-bar"]);
    for needle in [
        "justify-content: space-between",
        "border-bottom: 1px solid var(--color-line)",
        "color: var(--color-ink-3)",
    ] {
        assert!(bar.contains(needle), "missing `{needle}`: {bar}");
    }
    let pre = rule_body(&css, &[".code-block pre"]);
    for needle in [
        "margin: 0",
        "overflow-x: auto",
        "font-size: 0.8125rem",
        "line-height: 1.65",
    ] {
        assert!(pre.contains(needle), "missing `{needle}`: {pre}");
    }
    let copy_hover = rule_body(&css, &[".code-copy:hover"]);
    assert!(
        copy_hover.contains("color: var(--color-accent)"),
        "the copy button must warm to accent on hover: {copy_hover}"
    );
}

// The site chrome contract: a sticky translucent
// blurred bar, nav links whose accent underline slides in on hover and stays
// on the active route, and the mono footer hosting the konami toast.
#[test]
fn site_chrome_is_styled() {
    let css = stylesheet();
    let nav = rule_body(&css, &[".site-nav"]);
    assert!(
        nav.contains("position: sticky") && nav.contains("top: 0"),
        "the bar must stick to the top: {nav}"
    );
    assert!(
        nav.contains("backdrop-filter: blur(12px)"),
        "the bar blurs what scrolls under it: {nav}"
    );
    assert!(
        nav.contains("color-mix(in srgb, var(--color-surface) 78%, transparent)"),
        "the bar fill is the translucent surface: {nav}"
    );
    let link_slide = rule_body(&css, &[".nav-link::after"]);
    assert!(
        link_slide.contains("right: 100%"),
        "the accent underline starts collapsed and slides in: {link_slide}"
    );
    assert!(
        css.contains(".nav-link[aria-current=\"page\"]"),
        "the active route keeps a persistent underline"
    );
    let shared_box = rule_body(&css, &[".nav-link", ".nav-seg"]);
    assert!(
        shared_box.contains("position: relative"),
        "the slug shares the nav-link box so its underline sits inside the truncation clip: {shared_box}"
    );
    let shared_line = rule_body(&css, &[".nav-link::after", ".nav-seg::after"]);
    assert!(
        shared_line.contains("var(--color-accent)"),
        "one grouped rule paints the accent you-are-here line for links and slug alike: {shared_line}"
    );
    let link = rule_body(&css, &[".nav-link"]);
    assert!(
        !link.contains("text-transform"),
        "nav labels are authored lowercase, never CSS-transformed: {link}"
    );
    let footer = rule_body(&css, &[".site-footer"]);
    assert!(
        footer.contains("var(--font-mono)") && footer.contains("var(--color-ink-3)"),
        "the footer signs in faint mono: {footer}"
    );
    let toast = rule_body(&css, &[".konami-toast"]);
    assert!(
        toast.contains("position: fixed") && toast.contains("var(--color-accent)"),
        "the toast floats over the page with an accent border: {toast}"
    );
    assert!(
        css.contains("@keyframes blink"),
        "the toast cursor needs its blink keyframes"
    );
}

/// The declarations of the one rule whose selector list is exactly
/// `selectors` — any order, any formatting, so a stylesheet reflow can't
/// break a guard. Matching the whole set keeps a grouped rule distinct from
/// its selectors' standalone rules; more than one match panics instead of
/// silently guarding the first.
fn rule_body<'a>(css: &'a str, selectors: &[&str]) -> &'a str {
    let mut want: Vec<String> = selectors.iter().map(|s| normalize(s)).collect();
    want.sort();
    let mut matches = Vec::new();
    let mut stack = Vec::new();
    let mut prelude_start = 0;
    let bytes = css.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            // Comments may hold braces; skip them whole.
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i += css[i..].find("*/").map_or(css.len() - i, |end| end + 2);
                continue;
            }
            b'{' => {
                stack.push((strip_comments(&css[prelude_start..i]), i + 1));
                prelude_start = i + 1;
            }
            b'}' => {
                if let Some((prelude, body_start)) = stack.pop() {
                    let mut have: Vec<String> = prelude.split(',').map(normalize).collect();
                    have.sort();
                    if have == want {
                        matches.push(&css[body_start..i]);
                    }
                }
                prelude_start = i + 1;
            }
            b';' => prelude_start = i + 1,
            _ => {}
        }
        i += 1;
    }
    match matches[..] {
        [body] => body,
        [] => panic!("no `{}` rule in the stylesheet", selectors.join(", ")),
        _ => panic!(
            "`{}` opens {} rules — a guard must name exactly one",
            selectors.join(", "),
            matches.len()
        ),
    }
}

/// Selector text with whitespace runs collapsed, so formatting can't matter.
fn normalize(selector: &str) -> String {
    selector.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A rule's prelude spans from the previous rule, so the comment above a
/// rule lands in it; selectors are what's left once comments go.
fn strip_comments(prelude: &str) -> String {
    let mut out = String::new();
    let mut rest = prelude;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        rest = rest[start..]
            .find("*/")
            .map_or("", |end| &rest[start + end + 2..]);
    }
    out.push_str(rest);
    out
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

// The filter island's two states: the active pill takes the accent
// treatment; the `$ ls` empty state is mono and muted.
#[test]
fn tag_filter_states_are_styled() {
    let css = stylesheet();
    let active = rule_body(&css, &["a.tag.tag-active"]);
    assert!(
        active.contains("background-color: var(--color-accent-subtle)"),
        "the active pill fills accent-subtle: {active}"
    );
    assert!(
        active.contains("border-color: var(--color-accent)"),
        "{active}"
    );
    let empty = rule_body(&css, &[".filter-empty"]);
    assert!(empty.contains("var(--font-mono)"), "{empty}");
    assert!(empty.contains("var(--color-ink-3)"), "{empty}");
}

// The PostRow hover contract: title turns accent, the
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

// The glyphs follow the effective scheme in CSS alone: an explicit
// `data-theme` wins, the system preference decides otherwise; hover rotates
// the glyph unless reduced motion is set.
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
        "hover must rotate the glyph"
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
