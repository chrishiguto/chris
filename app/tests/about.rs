//! The `/about` page is hardcoded copy with no logic; asserting its
//! constants only breaks on edits without ever catching a bug. The one
//! contract left is that the router actually serves it.
#![cfg(feature = "ssr")]

mod common;

#[test]
fn about_route_renders_the_page() {
    let html = common::app_at("/about");
    assert!(
        html.contains(">about</h1>"),
        "/about must render the about page: {html}"
    );
    assert!(
        !html.contains("404"),
        "/about must be routed, not the fallback: {html}"
    );
}
