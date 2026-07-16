//! Site chrome: the bar with its logo, nav links, and the footer with its
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

// Post pages live under the writing section: `writing` is current as the
// section (`true`), never as the page (`page` is the exact route's). The
// boundary check keeps lookalike 404 paths from claiming the section.
#[test]
fn writing_is_the_current_section_on_post_pages() {
    let html = header_at("/posts/missing-await");
    assert!(
        tag_containing(&html, ">writing<").contains("aria-current=\"true\""),
        "`writing` must be section-current on a post page: {html}"
    );
    assert!(
        !tag_containing(&html, ">about<").contains("aria-current"),
        "`about` must not be active on a post page: {html}"
    );
    assert!(
        !html.contains("missing-await"),
        "the bar must not carry the slug: {html}"
    );

    let lookalike = header_at("/posts-lookalike");
    assert!(
        !lookalike.contains("aria-current"),
        "a lookalike 404 path must not claim the section: {lookalike}"
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

// The fade-up stagger is a page-load cadence: every routed page and both
// 404s mount their main content under `page-enter`. The CSS side of the
// contract pins in stylesheet_contract.rs.
#[test]
fn every_page_mounts_its_content_under_page_enter() {
    for path in ["/", "/posts", "/posts/anything", "/about", "/nowhere"] {
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
