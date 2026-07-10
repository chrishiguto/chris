use content::ABOUT_PATH;
use leptos::prelude::*;
use leptos_router::hooks::use_location;

use super::ThemeToggle;

/// `/posts/{slug}` → the slug: the one route family where the wordmark gives
/// way to the terminal breadcrumb. Anything deeper (or empty) matches no
/// route, so it 404s under the wordmark.
fn breadcrumb_slug(path: &str) -> Option<&str> {
    let slug = path.strip_prefix("/posts/")?;
    (!slug.is_empty() && !slug.contains('/')).then_some(slug)
}

/// The sticky site bar (design NavBar): `~/chris` wordmark on listing pages,
/// the `~/chris/posts/{slug}` breadcrumb on post pages — both fully
/// server-rendered from the request URL, with the mono nav and theme toggle
/// at the right end on every page.
#[component]
pub fn Header() -> impl IntoView {
    // Read once, non-reactively: outside islands nothing runs client-side,
    // so the header is a pure per-request render (every navigation is a
    // full page load).
    let path = use_location().pathname.get_untracked();
    let left = match breadcrumb_slug(&path).map(str::to_string) {
        Some(slug) => breadcrumb(slug).into_any(),
        None => wordmark().into_any(),
    };

    view! {
        <header class="site-nav">
            <div class="mx-auto flex w-full max-w-2xl items-center justify-between gap-6 px-6">
                {left}
                <nav class="flex shrink-0 items-center gap-1">
                    {bar_links(&path)} <ThemeToggle />
                </nav>
            </div>
        </header>
    }
}

fn wordmark() -> impl IntoView {
    view! {
        <a href="/" class="nav-mark">
            <span class="nav-tilde">"~/"</span>
            "chris"
        </a>
    }
}

fn bar_links(path: &str) -> impl IntoView {
    view! {
        {nav_link("writing", "/posts", path == "/posts")}
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

/// Every segment is a link except the slug the reader is on.
fn breadcrumb(slug: String) -> impl IntoView {
    view! {
        <div class="nav-path">
            <a href="/">"~/chris"</a>
            <span class="nav-sep">"/"</span>
            <a href="/posts">"posts"</a>
            <span class="nav-sep">"/"</span>
            <span class="nav-seg">{slug}</span>
        </div>
    }
}
