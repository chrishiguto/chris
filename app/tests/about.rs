//! The `/about` static page: prompt motif, prose, currently list, contact
//! block. The links are well-formed mocks.
#![cfg(feature = "ssr")]

use app::about::AboutPage;
use common::ssr;

mod common;

fn about_html() -> String {
    ssr(|| (), || leptos::view! { <AboutPage /> })
}

#[test]
fn about_opens_with_the_prompt_motif() {
    let html = about_html();
    for needle in ["~/chris", ">$<", "cat about.md"] {
        assert!(html.contains(needle), "prompt missing `{needle}`: {html}");
    }
    let prompt = html.find("cat about.md").unwrap();
    let heading = html.find("about</h1>").expect("about heading missing");
    assert!(prompt < heading, "prompt must precede the heading: {html}");
}

#[test]
fn about_renders_all_four_sections_in_order() {
    let html = about_html();
    let positions: Vec<usize> = ["cat about.md", "about</h1>", "currently", "contact"]
        .iter()
        .map(|needle| {
            html.find(needle)
                .unwrap_or_else(|| panic!("section `{needle}` missing: {html}"))
        })
        .collect();
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "sections out of order: {html}"
    );
}

#[test]
fn about_prose_carries_the_design_copy() {
    let html = about_html();
    for needle in [
        "christiano higuto",
        "giant notebook",
        "away from the keyboard",
    ] {
        assert!(html.contains(needle), "prose missing `{needle}`: {html}");
    }
}

#[test]
fn currently_lists_reading_learning_listening() {
    let html = about_html();
    for needle in ["reading ·", "learning ·", "listening ·"] {
        assert!(
            html.contains(needle),
            "currently missing `{needle}`: {html}"
        );
    }
}

// Real handles don't exist yet; the hrefs must still be
// well-formed so the styling and layout are honest.
#[test]
fn contact_links_are_well_formed_mocks() {
    let html = about_html();
    for needle in [
        "href=\"mailto:hi@chris.dev\"",
        "href=\"https://github.com/chris\"",
        "href=\"https://www.linkedin.com/in/chris\"",
    ] {
        assert!(html.contains(needle), "contact missing `{needle}`: {html}");
    }
}
