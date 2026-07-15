//! Listing pages: `/` and `/posts`, rendered from the index provided via
//! context. Drafts are in the index but filtered here.

use content::{IndexEntry, ABOUT_PATH, POSTS_PATH};
use leptos::prelude::*;

use crate::components::{page, page_title, post_list, section_label, ListedPost, TagFilter};

/// Per-request index from the site worker, newest-first.
#[derive(Clone)]
pub struct IndexData(pub Vec<IndexEntry>);

pub const RECENT_POSTS: usize = 3;

fn listed_posts() -> Vec<ListedPost> {
    use_context::<IndexData>()
        .map(|data| data.0)
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| entry.is_listed())
        .map(Into::into)
        .collect()
}

/// One empty state for both listings, so `/` and `/posts` can't drift.
fn nothing_published() -> impl IntoView {
    view! { <p class="mt-6 text-ink-2">"nothing published yet — check back soon."</p> }
}

#[component]
pub fn PostsPage() -> impl IntoView {
    let posts = listed_posts();
    let has_posts = !posts.is_empty();
    let listing = view! {
        <Show when=move || has_posts fallback=nothing_published>
            <TagFilter posts=posts.clone() />
        </Show>
    };
    page(Some(page_title("posts")), "posts", listing)
}

#[component]
pub fn HomePage() -> impl IntoView {
    let posts = listed_posts();
    let total = posts.len();
    let recent: Vec<_> = posts.into_iter().take(RECENT_POSTS).collect();
    let has_posts = !recent.is_empty();
    // Type-erased: the fully typed Show nested into the page view overflows
    // rustc's query depth.
    let latest = view! {
        <Show when=move || has_posts fallback=nothing_published>
            {post_list(recent.iter().cloned().map(|post| (post, None)).collect(), "mt-4")}
            <p class="mt-4 text-ink-2">
                "that's the latest three. " <a href=POSTS_PATH class="plink">
                    {format!("read all {total} posts →")}
                </a>
            </p>
        </Show>
    }
    .into_any();
    page(
        None,
        "hey, i'm chris",
        view! {
            <p class="mt-5 max-w-[48ch] leading-relaxed text-ink-2">
                "software engineer. i write about code, systems, and figuring things out — in english e às vezes em português. this site is my notebook, left open on purpose."
            </p>
            <p class="mt-6 max-w-[52ch] leading-relaxed text-ink-2">
                "new here? start with " <a href=POSTS_PATH class="plink">
                    "the writing"
                </a> ", or read a little more " <a href=ABOUT_PATH class="plink">
                    "about me"
                </a> "."
            </p>
            <div class="mt-14">{section_label("latest writing")} {latest}</div>
        },
    )
}
