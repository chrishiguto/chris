//! Site chrome: the bar with its logo, nav links, and the signature
//! footer — all fully server-rendered.
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

/// The nav collapses to a single "about" link (the logo carries you home),
/// rendered on every page whatever the left slot shows.
fn assert_global_nav_links(html: &str) {
    let about = tag_containing(html, ">about<");
    assert!(
        about.contains("href=\"/about\"") && about.contains("nav-link"),
        "`about` must be a nav-link to /about: {html}"
    );
    assert!(
        !html.contains(">writing<"),
        "the nav no longer carries a `writing` link — the logo goes home: {html}"
    );
}

// Both theme variants of the logo ship in the HTML; the `when-*` pair shows
// the one matching the effective scheme, so the mark can't flash on theme
// change. Pinning src against class keeps the variants from swapping.
#[test]
fn bar_carries_the_logo_nav_and_toggle() {
    let html = header_at("/");
    let mark = tag_containing(&html, "class=\"nav-logo\"");
    assert!(mark.contains("href=\"/\""), "the logo links home: {html}");
    for theme in ["dark", "light"] {
        let img = tag_containing(&html, &format!("src=\"/images/logo-{theme}.svg\""));
        assert!(
            img.contains(&format!("class=\"when-{theme}\"")),
            "the {theme} logo must show under the {theme} scheme: {html}"
        );
        assert!(
            img.contains("alt=\"chris\"") && img.contains("width=") && img.contains("height="),
            "logo variants ship as sized, labeled images: {html}"
        );
    }
    assert_global_nav_links(&html);
    assert!(
        html.contains("theme-toggle"),
        "the bar keeps the theme toggle: {html}"
    );
    assert!(
        !html.contains("aria-current"),
        "no nav link is active on the home page: {html}"
    );
}

#[test]
fn active_nav_link_follows_the_route() {
    let about = header_at("/about");
    assert!(
        tag_containing(&about, ">about<").contains("aria-current=\"page\""),
        "`about` must be active on /about: {about}"
    );

    // Off /about — on the writing home and on a post page — no nav link is
    // current; the logo, not a link, marks the way home.
    for path in ["/", "/posts/missing-await"] {
        let html = header_at(path);
        assert!(
            !html.contains("aria-current"),
            "no nav link is current on `{path}`: {html}"
        );
    }
}

// The bar is the only navigation chrome, and it never leaks the route it sits
// on: a post page shows no slug, and a lookalike path can't wrongly mark the
// about link current.
#[test]
fn the_bar_never_leaks_the_route_or_claims_a_lookalike() {
    let post = header_at("/posts/missing-await");
    assert!(
        !post.contains("missing-await"),
        "the bar must not carry the slug: {post}"
    );

    let lookalike = header_at("/about-x");
    assert!(
        !lookalike.contains("aria-current"),
        "a lookalike path must not claim the about link: {lookalike}"
    );
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
    for needle in ["class=\"theme-toggle\"", "aria-label=\"toggle theme\""] {
        assert!(html.contains(needle), "toggle missing `{needle}`: {html}");
    }
    // Pinning glyph against class keeps the pair from swapping: the moon
    // shows under light (inviting the switch to dark), the sun under dark.
    for (glyph, theme) in [("☾", "light"), ("☀", "dark")] {
        assert!(
            tag_containing(&html, glyph).contains(&format!("class=\"glyph when-{theme}\"")),
            "`{glyph}` must show under the {theme} scheme: {html}"
        );
    }
}

#[test]
fn footer_signs_the_site() {
    let html = common::ssr(|| {}, || view! { <Footer /> });
    assert!(
        html.contains("© 2026 christiano higuto — built slowly, on purpose"),
        "the footer signs the site: {html}"
    );
}

// The fade-up stagger is a page-load cadence: every routed page and both
// 404s mount their main content under `page-enter`. The CSS side of the
// contract pins in stylesheet_contract.rs.
#[test]
fn every_page_mounts_its_content_under_page_enter() {
    for path in ["/", "/posts/anything", "/about", "/nowhere"] {
        let html = common::app_at(path);
        let mount = tag_containing(&html, "page-enter");
        assert!(
            mount.starts_with("<section") || mount.starts_with("<article"),
            "`{path}` must mount main content under page-enter: {html}"
        );
    }
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
