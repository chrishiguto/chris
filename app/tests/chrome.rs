//! Site chrome is paper, not a bar: an inert veil, one footer, and plain
//! gutter wayfinding on inner pages. All of it is server-rendered.
#![cfg(feature = "ssr")]

use app::components::{Footer, GutterNav, Veil};
use leptos::view;

mod common;

#[test]
fn veil_is_inert_and_replaces_the_bar() {
    let html = common::ssr(|| {}, || view! { <Veil /> });
    assert!(
        html.contains("class=\"site-veil\"") && html.contains("aria-hidden=\"true\""),
        "the decorative veil must stay out of interaction and accessibility: {html}"
    );

    for path in ["/", "/about", "/nowhere"] {
        let html = common::app_at(path);
        assert!(
            html.contains("site-veil"),
            "`{path}` needs the veil: {html}"
        );
        for retired in ["site-nav", "nav-logo", "nav-link", "theme-toggle"] {
            assert!(
                !html.contains(retired),
                "`{path}` must not render retired chrome `{retired}`: {html}"
            );
        }
    }
}

#[test]
fn footer_carries_only_the_signature_and_destinations() {
    let html = common::ssr(|| {}, || view! { <Footer /> });
    assert_eq!(
        html.matches("<footer").count(),
        1,
        "one site footer: {html}"
    );
    for item in ["christiano higuto", "são paulo", ">rss<", ">source<"] {
        assert!(html.contains(item), "footer missing `{item}`: {html}");
    }
    assert!(
        html.contains("href=\"/rss.xml\""),
        "rss destination: {html}"
    );
    assert!(
        html.contains("href=\"https://github.com/chrishiguto/chris\""),
        "source destination: {html}"
    );
    assert!(
        !html.contains("built slowly") && !html.contains("purpose"),
        "the retired tagline must stay gone: {html}"
    );
}

#[test]
fn post_wayfinding_is_a_plain_writing_link_not_an_island() {
    let html = common::ssr(
        || {},
        || view! { <GutterNav href=content::WRITING_PATH label="writing" /> },
    );
    assert!(
        html.starts_with("<nav"),
        "wayfinding is semantic nav: {html}"
    );
    assert!(
        html.contains("href=\"/writing\"") && html.contains("← writing"),
        "post wayfinding returns to writing: {html}"
    );
    assert!(
        !html.contains("leptos-island"),
        "history-based back behavior is retired: {html}"
    );
}

#[test]
fn chrome_wraps_every_page_including_404() {
    for path in ["/", "/nowhere"] {
        let html = common::app_at(path);
        assert_eq!(
            html.matches("<footer").count(),
            1,
            "`{path}` footer: {html}"
        );
        assert!(html.contains("site-veil"), "`{path}` veil: {html}");
        if path == "/nowhere" {
            assert!(
                html.contains("404"),
                "fallback remains inside chrome: {html}"
            );
        }
    }
}

#[test]
fn every_page_mounts_content_in_the_middle_column() {
    for path in ["/", "/writing", "/nowhere"] {
        let html = common::app_at(path);
        assert!(
            html.contains("page-grid"),
            "`{path}` needs the page grid: {html}"
        );
        assert!(
            html.contains("page-column page-enter"),
            "`{path}` content belongs in the middle column: {html}"
        );
    }
}
