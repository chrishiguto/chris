//! Site chrome: the bar with its terminal breadcrumb (`~/chris` root on
//! every page, segments on post pages), nav links, and the footer with its
//! konami package — all fully server-rendered.
#![cfg(feature = "ssr")]

use app::components::{Footer, Header};
use leptos::prelude::provide_context;
use leptos::view;
use leptos_router::components::Router;
use leptos_router::location::RequestUrl;

mod common;

use common::tag_containing;

/// The header as the worker renders it: inside a router primed with the
/// request URL, exactly like leptos_axum does per request.
fn header_at(path: &'static str) -> String {
    common::ssr(
        move || provide_context(RequestUrl::new(path)),
        || {
            view! {
                <Router>
                    <Header />
                </Router>
            }
        },
    )
}

/// The mono nav links render on every page, whatever the left slot shows.
fn assert_global_nav_links(html: &str) {
    let writing = tag_containing(html, ">writing<");
    assert!(
        writing.contains("href=\"/posts\"") && writing.contains("nav-link"),
        "`writing` must be a nav-link to /posts: {html}"
    );
    let about = tag_containing(html, ">about<");
    assert!(
        about.contains("href=\"/about\"") && about.contains("nav-link"),
        "`about` must be a nav-link to /about: {html}"
    );
}

#[test]
fn bar_carries_the_breadcrumb_root_nav_and_toggle() {
    let html = header_at("/");
    assert!(
        html.contains("<span class=\"nav-tilde\">~/</span>chris"),
        "the breadcrumb root must render `~/` faint before `chris`: {html}"
    );
    let mark = tag_containing(&html, "class=\"nav-mark\"");
    assert!(mark.contains("href=\"/\""), "the root links home: {html}");
    assert!(
        html.contains("nav-path"),
        "the root is the breadcrumb itself, on every page: {html}"
    );
    assert_global_nav_links(&html);
    assert!(
        html.contains("theme-toggle"),
        "the bar keeps the theme toggle: {html}"
    );
    assert!(
        !html.contains("aria-current"),
        "no nav link is active on the home page: {html}"
    );
    assert!(
        !html.contains("nav-seg") && !html.contains("nav-sep"),
        "no segments beyond the root on the home page: {html}"
    );
}

#[test]
fn active_nav_link_follows_the_route() {
    let posts = header_at("/posts");
    assert!(
        tag_containing(&posts, ">writing<").contains("aria-current=\"page\""),
        "`writing` must be active on /posts: {posts}"
    );
    assert!(
        !tag_containing(&posts, ">about<").contains("aria-current"),
        "`about` must not be active on /posts: {posts}"
    );

    let about = header_at("/about");
    assert!(
        tag_containing(&about, ">about<").contains("aria-current=\"page\""),
        "`about` must be active on /about: {about}"
    );
    assert!(
        !tag_containing(&about, ">writing<").contains("aria-current"),
        "`writing` must not be active on /about: {about}"
    );
}

#[test]
fn post_pages_extend_the_breadcrumb_with_their_segments() {
    let html = header_at("/posts/missing-await");
    let mark = tag_containing(&html, "class=\"nav-mark\"");
    assert!(
        mark.contains("href=\"/\""),
        "the `~/chris` root stays and links home: {html}"
    );
    let posts = tag_containing(&html, ">posts<");
    assert!(
        posts.contains("href=\"/posts\""),
        "the `posts` segment links to the listing: {html}"
    );
    assert!(
        html.contains("<span class=\"nav-seg\">missing-await</span>"),
        "the slug is the active (unlinked) segment: {html}"
    );
    assert_global_nav_links(&html);
    assert!(
        !html.contains("aria-current"),
        "no nav link claims the post page — the underlined slug marks the spot: {html}"
    );
    assert!(
        html.contains("theme-toggle"),
        "post pages keep the theme toggle: {html}"
    );
}

// Paths under /posts/ that no route matches (deep or empty) are 404s and
// keep the bare root — segments only describe real post routes.
#[test]
fn unmatched_post_paths_keep_the_bare_root() {
    for path in ["/posts/", "/posts/a/b"] {
        let html = header_at(path);
        assert!(
            html.contains("nav-mark") && !html.contains("nav-seg"),
            "`{path}` must keep the bare breadcrumb root: {html}"
        );
    }
}

// The toggle island SSRs both glyphs: CSS picks the visible one,
// so the button can't flash a stale icon before hydration.
#[test]
fn theme_toggle_ssrs_both_glyphs_as_an_island() {
    let html = header_at("/");
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

#[test]
fn footer_ships_copyright_and_the_konami_package() {
    let html = common::ssr(|| {}, || view! { <Footer /> });
    assert!(
        html.contains("© 2026 christiano higuto — built slowly, on purpose"),
        "the footer signs the site: {html}"
    );
    assert!(
        html.contains("<leptos-island"),
        "the konami egg must hydrate as an island: {html}"
    );
    assert!(
        html.contains("↑↑↓↓←→←→ba"),
        "the hint ships with the egg: {html}"
    );
    assert!(
        !html.contains("konami-toast"),
        "the toast only exists after the code is entered: {html}"
    );
}

// The chrome wraps every routed page and the 404 fallback alike: header and
// footer sit outside `Routes`, so nothing can render without them.
#[test]
fn chrome_wraps_every_page_including_404() {
    for path in ["/", "/nowhere"] {
        let html = common::app_at(path);
        assert!(
            html.contains("site-nav"),
            "`{path}` must render the nav: {html}"
        );
        assert!(
            html.contains("site-footer"),
            "`{path}` must render the footer: {html}"
        );
        if path == "/nowhere" {
            assert!(
                html.contains("404"),
                "the fallback still renders inside the chrome: {html}"
            );
        }
    }
}
