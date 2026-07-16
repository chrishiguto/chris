//! Document-shell contracts: the head loads in no-flash order and the
//! fonts arrive from the URL the stacks expect. How anything looks is
//! deliberately untested — that's the kitchen-sink read in both themes.
#![cfg(feature = "ssr")]

use app::app::{shell, GOOGLE_FONTS_URL, THEME_SCRIPT, THEME_STORAGE_KEY};
use leptos::prelude::LeptosOptions;

mod common;

fn shell_html() -> String {
    use leptos::prelude::provide_context;

    // The router needs the RequestUrl leptos_axum would provide per-request.
    let options = LeptosOptions::builder().output_name("chris").build();
    common::ssr(
        || provide_context(leptos_router::location::RequestUrl::new("/")),
        move || shell(options),
    )
}

// Geist + Geist Mono come from Google Fonts with `display=swap`, preconnected
// and linked in the shell head.
#[test]
fn fonts_load_from_google_with_swap() {
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
