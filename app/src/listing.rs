//! Listing pages: `/`, `/posts`, and tag browsing, rendered from the index
//! provided via context. Drafts are in the index but filtered here.

use content::{post_path, tag_path, IndexEntry};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::components::page;

/// Per-request index from the site worker, newest-first.
#[derive(Clone)]
pub struct IndexData(pub Vec<IndexEntry>);

pub const RECENT_POSTS: usize = 5;

fn listed_entries() -> Vec<IndexEntry> {
    use_context::<IndexData>()
        .map(|data| data.0)
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| entry.is_listed())
        .collect()
}

/// Markup shape `main.css` styles: `ul.post-list > li > a > h2 + p.post-date`.
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

/// Tag as a plain prop so tests need no router. An empty match renders a
/// readable state; the worker owns the 404 status for unknown tags.
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
