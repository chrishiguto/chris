use content::ABOUT_PATH;
use leptos::prelude::*;
use leptos_router::hooks::use_location;

use super::ThemeToggle;

/// The sticky site bar: the logo linking home on the left, the "about" nav
/// link with the theme toggle on the right. The logo carries you to the
/// writing home, so the bar needs no "writing" link. Fully server-rendered
/// from the request URL.
#[component]
pub fn Header() -> impl IntoView {
    // Read once, non-reactively: outside islands nothing runs client-side,
    // so the header is a pure per-request render (every navigation is a
    // full page load).
    let path = use_location().pathname.get_untracked();
    view! {
        <header class="site-nav">
            <div class="mx-auto flex w-full max-w-2xl items-center justify-between gap-6 px-6">
                // Both theme variants ship in the HTML; the `when-*` pair
                // shows the one matching the effective scheme, so the served
                // page stays one response per URL.
                <a href="/" class="nav-logo">
                    <img
                        src="/images/logo-dark.svg"
                        alt="chris"
                        width="28"
                        height="28"
                        class="when-dark"
                    />
                    <img
                        src="/images/logo-light.svg"
                        alt="chris"
                        width="28"
                        height="28"
                        class="when-light"
                    />
                </a>
                <nav class="flex shrink-0 items-center gap-1">
                    <NavLink label="about" href=ABOUT_PATH path=path />
                    <ThemeToggle />
                </nav>
            </div>
        </header>
    }
}

/// `page` on the link's exact route only. The about page has no subpaths, so
/// anything under `/about/` is a 404 that must not claim the link — the same
/// bar 404s render — and lookalike paths (`/about-x`) never could.
fn aria_current(path: &str, href: &str) -> Option<&'static str> {
    (path == href).then_some("page")
}

#[component]
fn NavLink(label: &'static str, href: &'static str, path: String) -> impl IntoView {
    view! {
        <a href=href class="nav-link" aria-current=aria_current(&path, href)>
            {label}
        </a>
    }
}
