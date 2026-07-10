//! Site chrome: the route-aware nav (bar vs terminal breadcrumb) and the
//! footer with its konami package — all fully server-rendered.
#![cfg(feature = "ssr")]

use app::app::App;
use app::components::{Footer, Header};
use leptos::prelude::provide_context;
use leptos::view;
use leptos_router::components::Router;
use leptos_router::location::RequestUrl;

mod common;

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

/// The opening tag around the first occurrence of `needle` — attribute
/// order in leptos output is an implementation detail, so assertions look
/// inside one tag instead of pinning full-tag strings.
fn tag_containing<'a>(html: &'a str, needle: &str) -> &'a str {
    let at = html
        .find(needle)
        .unwrap_or_else(|| panic!("no `{needle}` in: {html}"));
    let start = html[..at].rfind('<').expect("needle outside any tag");
    let end = at + html[at..].find('>').expect("unclosed tag");
    &html[start..=end]
}

#[test]
fn bar_variant_carries_the_wordmark_nav_and_toggle() {
    let html = header_at("/");
    assert!(
        html.contains("<span class=\"nav-tilde\">~/</span>chris"),
        "the wordmark must render `~/` faint before `chris`: {html}"
    );
    let mark = tag_containing(&html, "class=\"nav-mark\"");
    assert!(
        mark.contains("href=\"/\""),
        "the wordmark links home: {html}"
    );
    let writing = tag_containing(&html, ">writing<");
    assert!(
        writing.contains("href=\"/posts\"") && writing.contains("nav-link"),
        "`writing` must be a nav-link to /posts: {html}"
    );
    let about = tag_containing(&html, ">about<");
    assert!(
        about.contains("href=\"/about\"") && about.contains("nav-link"),
        "`about` must be a nav-link to /about: {html}"
    );
    assert!(
        html.contains("theme-toggle"),
        "the bar variant keeps the theme toggle: {html}"
    );
    assert!(
        !html.contains("aria-current"),
        "no nav link is active on the home page: {html}"
    );
    assert!(
        !html.contains("nav-path"),
        "no breadcrumb on the home page: {html}"
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
fn post_pages_switch_to_the_terminal_breadcrumb() {
    let html = header_at("/posts/missing-await");
    assert!(
        html.contains("nav-path"),
        "post pages get the breadcrumb: {html}"
    );
    let root = tag_containing(&html, ">~/chris<");
    assert!(root.contains("href=\"/\""), "`~/chris` links home: {html}");
    let posts = tag_containing(&html, ">posts<");
    assert!(
        posts.contains("href=\"/posts\""),
        "the `posts` segment links to the listing: {html}"
    );
    assert!(
        html.contains("<span class=\"nav-seg\">missing-await</span>"),
        "the slug is the active (unlinked) segment: {html}"
    );
    assert!(
        !html.contains("nav-mark") && !html.contains("nav-link"),
        "the bar variant is gone on post pages: {html}"
    );
    assert!(
        html.contains("theme-toggle"),
        "the breadcrumb variant keeps the theme toggle: {html}"
    );
}

// Paths under /posts/ that no route matches (deep or empty) are 404s and
// keep the bar variant — the breadcrumb only describes real post routes.
#[test]
fn unmatched_post_paths_keep_the_bar() {
    for path in ["/posts/", "/posts/a/b"] {
        let html = header_at(path);
        assert!(
            html.contains("nav-mark") && !html.contains("nav-path"),
            "`{path}` must keep the bar variant: {html}"
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

// The chrome wraps every routed page and the 404 fallback alike: header and
// footer sit outside `Routes`, so nothing can render without them.
#[test]
fn chrome_wraps_every_page_including_404() {
    for path in ["/", "/nowhere"] {
        let html = common::ssr(
            move || provide_context(RequestUrl::new(path)),
            || view! { <App /> },
        );
        assert!(
            html.contains("site-nav"),
            "`{path}` must render the nav: {html}"
        );
        assert!(
            html.contains("site-footer"),
            "`{path}` must render the footer: {html}"
        );
    }
    let html = common::ssr(
        || provide_context(RequestUrl::new("/nowhere")),
        || view! { <App /> },
    );
    assert!(
        html.contains("404"),
        "the fallback still renders inside the chrome: {html}"
    );
}
