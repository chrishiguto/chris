//! Listing pages: `/` (recent posts), `/posts` (everything), and tag
//! browsing (`/tags`, `/tags/{tag}`), rendered from the KV `index` the site
//! worker provides via context. Drafts are stored in the index but filtered
//! here, at render time.

use content::{post_path, tag_path, IndexEntry};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::components::page;

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
        .filter(|entry| entry.is_listed())
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
                    <a href=post_path(&entry.slug)>
                        <h2>{entry.title}</h2>
                        <p class="post-date">{entry.date}</p>
                    </a>
                </li>
            }
        })
        .collect();
    view! { <ul class="post-list">{items}</ul> }
}

fn empty_state(message: String) -> impl IntoView {
    view! { <p class="mt-6 text-ink-muted">{message}</p> }
}

const NOTHING_PUBLISHED: &str = "Nothing published yet — check back soon.";

#[component]
pub fn PostsPage() -> impl IntoView {
    let entries = listed_entries();
    let listing = if entries.is_empty() {
        empty_state(NOTHING_PUBLISHED.into()).into_any()
    } else {
        post_list(entries).into_any()
    };
    page(Some("posts — chris".into()), "posts", listing)
}

/// Tags across published posts with how many posts carry each, alphabetical.
fn tag_counts(entries: &[IndexEntry]) -> Vec<(String, usize)> {
    entries
        .iter()
        .flat_map(|entry| entry.tags.iter())
        .fold(std::collections::BTreeMap::new(), |mut counts, tag| {
            *counts.entry(tag.clone()).or_insert(0) += 1;
            counts
        })
        .into_iter()
        .collect()
}

#[component]
pub fn TagsPage() -> impl IntoView {
    let tags = tag_counts(&listed_entries());
    let listing = if tags.is_empty() {
        empty_state("Nothing is tagged yet.".into()).into_any()
    } else {
        let pills: Vec<_> = tags
            .into_iter()
            .map(|(tag, count)| {
                view! {
                    <li class="tag">
                        <a href=tag_path(&tag)>{tag.clone()}</a>
                        " ×"
                        {count}
                    </li>
                }
            })
            .collect();
        view! { <ul class="post-tags mt-10">{pills}</ul> }.into_any()
    };
    page(Some("tags — chris".into()), "tags", listing)
}

/// The body of `/tags/{tag}`, with the tag as a plain prop so tests need no
/// router; [`TagPage`] reads it from the URL. An empty match renders a
/// readable state — the worker owns the 404 status for unknown tags.
#[component]
pub fn TagListing(tag: String) -> impl IntoView {
    let matching: Vec<_> = listed_entries()
        .into_iter()
        .filter(|entry| entry.tags.contains(&tag))
        .collect();
    let listing = if matching.is_empty() {
        empty_state(format!("Nothing is tagged \"{tag}\".")).into_any()
    } else {
        post_list(matching).into_any()
    };
    page(Some(format!("#{tag} — chris")), format!("#{tag}"), listing)
}

#[component]
pub fn TagPage() -> impl IntoView {
    let tag = use_params_map().read().get("tag").unwrap_or_default();
    view! { <TagListing tag=tag /> }
}

#[component]
pub fn HomePage() -> impl IntoView {
    let entries = listed_entries();
    let recent: Vec<_> = entries.into_iter().take(RECENT_POSTS).collect();
    let listing = if recent.is_empty() {
        empty_state(NOTHING_PUBLISHED.into()).into_any()
    } else {
        view! {
            {post_list(recent)}
            <a href="/posts" class="font-mono text-sm text-accent hover:underline">
                "all posts →"
            </a>
        }
        .into_any()
    };
    page(
        None,
        "chris",
        view! {
            <p class="mt-6 leading-relaxed text-ink-muted">
                "Engineering notes — Rust end-to-end: Leptos SSR on Cloudflare Workers."
            </p>
            <div class="mt-10">{listing}</div>
        },
    )
}
