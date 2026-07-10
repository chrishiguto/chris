//! Listing pages: `/` and `/posts`, rendered from the index provided via
//! context. Drafts are in the index but filtered here.

use content::{post_path, IndexEntry, ABOUT_PATH, POSTS_PATH};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::components::{meta_row, page, section_label, TagFilter};

/// Per-request index from the site worker, newest-first.
#[derive(Clone)]
pub struct IndexData(pub Vec<IndexEntry>);

pub const RECENT_POSTS: usize = 3;

/// One listed post, in the shape the pages render: the published subset of
/// an index entry. Internal fields (content hash, draft) never reach the
/// client — this is what the filter island serializes as props.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ListedPost {
    pub slug: String,
    pub title: String,
    pub date: String,
    pub reading_minutes: Option<u32>,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

impl From<IndexEntry> for ListedPost {
    fn from(entry: IndexEntry) -> Self {
        Self {
            slug: entry.slug,
            title: entry.title,
            date: entry.date,
            reading_minutes: entry.reading_minutes,
            description: entry.description,
            tags: entry.tags,
        }
    }
}

fn listed_posts() -> Vec<ListedPost> {
    use_context::<IndexData>()
        .map(|data| data.0)
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| entry.is_listed())
        .map(Into::into)
        .collect()
}

/// The whole row is the link; the arrow slides in on hover via CSS. The
/// `<li>` belongs to the caller — the filter island binds visibility to it.
pub(crate) fn post_row(post: ListedPost) -> impl IntoView {
    let meta = meta_row(&post.date, post.reading_minutes);
    view! {
        <a href=post_path(&post.slug) class="post-row">
            <span class="post-row-top">
                <span class="post-row-title">
                    {post.title} <span class="post-row-lead" aria-hidden="true">
                        "→"
                    </span>
                </span>
                <span class="post-row-meta">{meta}</span>
            </span>
            {post
                .description
                .map(|description| view! { <span class="post-row-desc">{description}</span> })}
        </a>
    }
}

/// Markup shape `post.css` styles: `ul.post-list > li > a.post-row`.
fn post_list(posts: Vec<ListedPost>) -> impl IntoView {
    let items: Vec<_> = posts
        .into_iter()
        .map(|post| view! { <li>{post_row(post)}</li> })
        .collect();
    view! { <ul class="post-list">{items}</ul> }
}

fn empty_state(message: String) -> impl IntoView {
    view! { <p class="mt-6 text-ink-2">{message}</p> }
}

#[component]
pub fn PostsPage() -> impl IntoView {
    let posts = listed_posts();
    let has_posts = !posts.is_empty();
    let listing = view! {
        <Show
            when=move || has_posts
            fallback=|| empty_state("Nothing published yet — check back soon.".into())
        >
            <TagFilter posts=posts.clone() />
        </Show>
    };
    page(Some("posts — chris".into()), "posts", listing)
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
        <Show
            when=move || has_posts
            fallback=|| empty_state("Nothing published yet — check back soon.".into())
        >
            <div class="mt-4">{post_list(recent.clone())}</div>
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
