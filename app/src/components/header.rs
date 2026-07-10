use content::{post_path_slug, ABOUT_PATH, POSTS_PATH};
use leptos::prelude::*;
use leptos_router::hooks::use_location;

use super::ThemeToggle;

/// The sticky site bar (design NavBar): `~/chris` wordmark everywhere except
/// post pages, where the `~/chris/posts/{slug}` breadcrumb takes over — both
/// fully server-rendered from the request URL, with the mono nav and theme
/// toggle at the right end on every page.
#[component]
pub fn Header() -> impl IntoView {
    // Read once, non-reactively: outside islands nothing runs client-side,
    // so the header is a pure per-request render (every navigation is a
    // full page load).
    let path = use_location().pathname.get_untracked();
    let left = match post_path_slug(&path).map(str::to_string) {
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

/// Every segment is a link except the slug the reader is on.
fn breadcrumb(slug: String) -> impl IntoView {
    view! {
        <div class="nav-path">
            <a href="/">"~/chris"</a>
            <span class="nav-sep">"/"</span>
            <a href=POSTS_PATH>"posts"</a>
            <span class="nav-sep">"/"</span>
            <span class="nav-seg">{slug}</span>
        </div>
    }
}
