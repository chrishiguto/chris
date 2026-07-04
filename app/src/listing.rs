//! Listing pages: `/` (recent posts) and `/posts` (everything), rendered
//! from the KV `index` the site worker provides via context. Drafts are
//! stored in the index but filtered here, at render time (PRD "KV schema").

use content_ast::IndexEntry;
use leptos::prelude::*;
use leptos_meta::Title;

/// Per-request payload provided by the site worker: the deserialized KV
/// `index`, newest-first. Empty when nothing has been published yet.
#[derive(Clone)]
pub struct IndexData(pub Vec<IndexEntry>);

/// How many posts the home page shows before deferring to `/posts`.
pub const RECENT_POSTS: usize = 5;

/// Published entries only, in stored (newest-first) order.
fn listed_entries() -> Vec<IndexEntry> {
    use_context::<IndexData>()
        .map(|data| data.0)
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| !entry.draft)
        .collect()
}

/// The markup shape `main.css` styles:
/// `ul.post-list > li > a > h2 + p.post-date`.
fn post_list(entries: Vec<IndexEntry>) -> impl IntoView {
    let items: Vec<_> = entries
        .into_iter()
        .map(|entry| {
            view! {
                <li>
                    <a href=format!("/posts/{}", entry.slug)>
                        <h2>{entry.title}</h2>
                        <p class="post-date">{entry.date}</p>
                    </a>
                </li>
            }
        })
        .collect();
    view! { <ul class="post-list">{items}</ul> }
}

fn empty_state() -> impl IntoView {
    view! { <p class="mt-6 text-ink-muted">"Nothing published yet — check back soon."</p> }
}

#[component]
pub fn PostsPage() -> impl IntoView {
    let entries = listed_entries();
    view! {
        <Title text="posts — chris" />
        <section class="mx-auto max-w-2xl px-6 py-16">
            <h1 class="font-heading text-3xl font-bold">"posts"</h1>
            {if entries.is_empty() {
                empty_state().into_any()
            } else {
                post_list(entries).into_any()
            }}
        </section>
    }
}

#[component]
pub fn HomePage() -> impl IntoView {
    let entries = listed_entries();
    let recent: Vec<_> = entries.into_iter().take(RECENT_POSTS).collect();
    let listing = if recent.is_empty() {
        empty_state().into_any()
    } else {
        view! {
            {post_list(recent)}
            <a href="/posts" class="font-mono text-sm text-accent hover:underline">
                "all posts →"
            </a>
        }
        .into_any()
    };
    view! {
        <section class="mx-auto max-w-2xl px-6 py-16">
            <h1 class="font-heading text-3xl font-bold">"chris"</h1>
            <p class="mt-6 leading-relaxed text-ink-muted">
                "Engineering notes — Rust end-to-end: Leptos SSR on Cloudflare Workers."
            </p>
            <div class="mt-10">{listing}</div>
        </section>
    }
}
