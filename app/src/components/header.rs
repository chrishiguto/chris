use content::{post_path_slug, ABOUT_PATH, POSTS_PATH};
use leptos::prelude::*;
use leptos_router::hooks::use_location;

use super::ThemeToggle;

/// The sticky site bar: the terminal breadcrumb on the left — its `~/chris`
/// root on every page, extended with `/ posts / {slug}` segments on post
/// pages — and the mono nav with the theme toggle on the right. Fully
/// server-rendered from the request URL.
#[component]
pub fn Header() -> impl IntoView {
    // Read once, non-reactively: outside islands nothing runs client-side,
    // so the header is a pure per-request render (every navigation is a
    // full page load).
    let path = use_location().pathname.get_untracked();
    view! {
        <header class="site-nav">
            <div class="mx-auto flex w-full max-w-2xl items-center justify-between gap-6 px-6">
                {breadcrumb(post_path_slug(&path).map(str::to_string))}
                <nav class="flex shrink-0 items-center gap-1">
                    {bar_links(&path)} <ThemeToggle />
                </nav>
            </div>
        </header>
    }
}

/// One breadcrumb for every page: the `~/chris` root always links home; a
/// post page appends its segments, every one linked except the slug the
/// reader is on.
fn breadcrumb(slug: Option<String>) -> impl IntoView {
    let segments = slug.map(|slug| {
        view! {
            <span class="nav-sep">"/"</span>
            <a href=POSTS_PATH>"posts"</a>
            <span class="nav-sep">"/"</span>
            <span class="nav-seg">{slug}</span>
        }
    });
    view! {
        <div class="nav-path">
            <a href="/" class="nav-mark">
                <span class="nav-tilde">"~/"</span>
                "chris"
            </a>
            {segments}
        </div>
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
