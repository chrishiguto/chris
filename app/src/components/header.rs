use content::{ABOUT_PATH, POSTS_PATH};
use leptos::prelude::*;
use leptos_router::hooks::use_location;

use super::ThemeToggle;

/// The sticky site bar: the logo linking home on the left, the mono nav
/// with the theme toggle on the right. Fully server-rendered from the
/// request URL.
#[component]
pub fn Header() -> impl IntoView {
    // Read once, non-reactively: outside islands nothing runs client-side,
    // so the header is a pure per-request render (every navigation is a
    // full page load).
    let path = use_location().pathname.get_untracked();
    view! {
        <header class="site-nav">
            <div class="mx-auto flex w-full max-w-2xl items-center justify-between gap-6 px-6">
                // Both theme variants ship in the HTML; CSS shows the one
                // matching the effective scheme, like the toggle glyphs, so
                // the served page stays one response per URL.
                <a href="/" class="nav-logo">
                    <img
                        src="/images/logo.svg"
                        alt="chris"
                        width="28"
                        height="28"
                        class="logo-dark"
                    />
                    <img
                        src="/images/logo-black.svg"
                        alt="chris"
                        width="28"
                        height="28"
                        class="logo-light"
                    />
                </a>
                <nav class="flex shrink-0 items-center gap-1">
                    {bar_links(&path)} <ThemeToggle />
                </nav>
            </div>
        </header>
    }
}

fn bar_links(path: &str) -> impl IntoView {
    view! {
        {nav_link("writing", POSTS_PATH, path == POSTS_PATH)}
        {nav_link("about", ABOUT_PATH, path == ABOUT_PATH)}
    }
}

fn nav_link(label: &'static str, href: &'static str, current: bool) -> impl IntoView {
    view! {
        <a href=href class="nav-link" aria-current=current.then_some("page")>
            {label}
        </a>
    }
}
