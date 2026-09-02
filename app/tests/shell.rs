//! Document-shell contracts: the two requested font families load and theme
//! selection remains entirely in system CSS, with no per-visitor script.
#![cfg(feature = "ssr")]

use app::app::{shell, GOOGLE_FONTS_URL};
use leptos::prelude::LeptosOptions;

mod common;

fn shell_html() -> String {
    use leptos::prelude::provide_context;

    let options = LeptosOptions::builder().output_name("chris").build();
    common::ssr(
        || provide_context(leptos_router::location::RequestUrl::new("/")),
        move || shell(options),
    )
}

#[test]
fn newsreader_and_geist_mono_load_with_the_full_axes() {
    for part in [
        "family=Geist+Mono:wght@400..700",
        "family=Newsreader:ital,opsz,wght@0,6..72,300..700;1,6..72,300..700",
        "display=swap",
    ] {
        assert!(
            GOOGLE_FONTS_URL.contains(part),
            "fonts URL missing `{part}`"
        );
    }
    assert_eq!(
        GOOGLE_FONTS_URL.matches("family=").count(),
        2,
        "the URL must load exactly two families"
    );
    assert!(!GOOGLE_FONTS_URL.contains("Fraunces"));
    assert!(!GOOGLE_FONTS_URL.contains("Figtree"));

    let html = shell_html();
    assert!(html.contains("rel=\"preconnect\" href=\"https://fonts.googleapis.com\""));
    assert!(
        html.contains("rel=\"preconnect\" href=\"https://fonts.gstatic.com\"")
            && html.contains("crossorigin")
    );
    assert!(html.contains(&GOOGLE_FONTS_URL.replace('&', "&amp;")));
}

#[test]
fn shell_contains_no_explicit_theme_state_or_prepaint_script() {
    let html = shell_html();
    for retired in ["chris-theme", "localStorage", "dataset.theme", "data-theme"] {
        assert!(
            !html.contains(retired),
            "system-only theming must not ship `{retired}`: {html}"
        );
    }
}
