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
                    {nav_link("about", ABOUT_PATH, &path)} <ThemeToggle />
                </nav>
            </div>
        </header>
    }
}

/// `page` on the link's exact route, `true` on a subpath — the about link
/// marks itself current on `/about` and its subpaths. The segment boundary
/// keeps lookalike 404 paths (`/about-x`) from claiming the section.
fn aria_current(path: &str, href: &str) -> Option<&'static str> {
    match path.strip_prefix(href) {
        Some("") => Some("page"),
        Some(rest) if rest.starts_with('/') => Some("true"),
        _ => None,
    }
}

fn nav_link(label: &'static str, href: &'static str, path: &str) -> impl IntoView {
    view! {
        <a href=href class="nav-link" aria-current=aria_current(path, href)>
            {label}
        </a>
    }
}
