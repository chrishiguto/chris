//! The home page `/`: the masthead identity band over the career timeline,
//! all static HTML — no index read, no island.
#![cfg(feature = "ssr")]

use app::home::HomePage;
use common::{ssr, tag_containing};

mod common;

fn home_html() -> String {
    ssr(|| (), || leptos::view! { <HomePage /> })
}

#[test]
fn home_greets_with_masthead_and_external_contacts() {
    let html = home_html();
    assert!(
        html.contains("hey, i’m chris"),
        "the masthead greets: {html}"
    );
    // External-only contacts sharing the about page's contact-link component;
    // the nav owns "writing" and "about", so the masthead carries no in-app
    // links.
    assert!(
        html.contains("mailto:hi@chris.dev"),
        "the email contact ships: {html}"
    );
    assert!(
        html.contains("github.com/chris"),
        "the github contact ships: {html}"
    );
    assert!(
        !html.contains(">the writing<") && !html.contains(">about me<"),
        "the masthead carries no in-app writing/about links: {html}"
    );
}

// The career section under the masthead: the markup shape the CSS styles —
// div.tl > div.tl-spine + ol.tl-list > li.tl-item — with sides alternating
// so the cards zigzag around the center spine.
#[test]
fn home_mounts_the_career_timeline() {
    let html = home_html();
    assert!(html.contains(">career</p>"), "the section label: {html}");
    assert!(html.contains("class=\"tl mt-8\""), "{html}");
    let spine = tag_containing(&html, "class=\"tl-spine\"");
    assert!(
        spine.contains("aria-hidden=\"true\""),
        "the spine is decoration: {html}"
    );
    assert!(html.contains("<ol class=\"tl-list\""), "{html}");
    let first_a = html.find("tl-item tl-side-a").expect("no side-a entry");
    let first_b = html.find("tl-item tl-side-b").expect("no side-b entry");
    assert!(first_a < first_b, "entries alternate starting left: {html}");
    // Each entry pairs the display-face year numeral with its full range.
    assert!(html.contains("tl-yearnum"), "{html}");
    // Exactly one stint is current, and its dot breathes.
    assert_eq!(html.matches("tl-dot tl-now").count(), 1, "{html}");
}

// The home lists no posts and hydrates no island — the timeline is static
// HTML, so the page needs no per-request index at all.
#[test]
fn home_carries_no_post_list_or_island() {
    let html = home_html();
    assert!(!html.contains("post-list"), "{html}");
    assert!(!html.contains("<leptos-island"), "{html}");
    assert!(!html.contains("type=\"search\""), "{html}");
}
