//! Listing pages: `/`, `/posts`, and tag browsing, rendered from the index
//! provided via context. Drafts are in the index but filtered here.

use content::{post_path, tag_path, IndexEntry};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::components::{page, section_label};

/// Per-request index from the site worker, newest-first.
#[derive(Clone)]
pub struct IndexData(pub Vec<IndexEntry>);

pub const RECENT_POSTS: usize = 3;

fn listed_entries() -> Vec<IndexEntry> {
    use_context::<IndexData>()
        .map(|data| data.0)
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| entry.is_listed())
        .collect()
}

/// Design PostRow: the whole row is the link; the arrow slides in on hover
/// via CSS. `data-tags` feeds the Slice 9 tag-filter island.
fn post_row(entry: IndexEntry) -> impl IntoView {
    view! {
        <li data-tags=entry.tags.join(" ")>
            <a href=post_path(&entry.slug) class="post-row">
                <span class="post-row-top">
                    <span class="post-row-title">
                        {entry.title} <span class="post-row-lead" aria-hidden="true">
                            "→"
                        </span>
                    </span>
                    <span class="post-row-meta">{entry.date}</span>
                </span>
                {entry
                    .description
                    .map(|description| view! { <span class="post-row-desc">{description}</span> })}
            </a>
        </li>
    }
}

/// Markup shape `post.css` styles: `ul.post-list > li[data-tags] > a.post-row`.
fn post_list(entries: Vec<IndexEntry>) -> impl IntoView {
    let items: Vec<_> = entries.into_iter().map(post_row).collect();
    view! { <ul class="post-list">{items}</ul> }
}

fn empty_state(message: String) -> impl IntoView {
    view! { <p class="mt-6 text-ink-2">{message}</p> }
}

const NOTHING_PUBLISHED: &str = "Nothing published yet — check back soon.";

#[component]
pub fn PostsPage() -> impl IntoView {
    let entries = listed_entries();
    let listing = if entries.is_empty() {
        empty_state(NOTHING_PUBLISHED.into()).into_any()
    } else {
        view! { <div class="mt-8">{post_list(entries)}</div> }.into_any()
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
        view! { <div class="mt-8">{post_list(matching)}</div> }.into_any()
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
    let total = entries.len();
    let recent: Vec<_> = entries.into_iter().take(RECENT_POSTS).collect();
    let latest = if recent.is_empty() {
        empty_state(NOTHING_PUBLISHED.into()).into_any()
    } else {
        view! {
            <div class="mt-4">{post_list(recent)}</div>
            <p class="mt-4 text-ink-2">
                "that's the latest three. " <a href="/posts" class="plink">
                    {format!("read all {total} posts →")}
                </a>
            </p>
        }
        .into_any()
    };
    page(
        None,
        "hey, i'm chris",
        view! {
            <p class="mt-5 max-w-[48ch] leading-relaxed text-ink-2">
                "software engineer. i write about code, systems, and figuring things out — in english e às vezes em português. this site is my notebook, left open on purpose."
            </p>
            <p class="mt-6 max-w-[52ch] leading-relaxed text-ink-2">
                "new here? start with " <a href="/posts" class="plink">
                    "the writing"
                </a> ", or read a little more " <a href="/about" class="plink">
                    "about me"
                </a> "."
            </p>
            <div class="mt-14">{section_label("latest writing")} {latest}</div>
        },
    )
}
