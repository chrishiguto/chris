//! Listing pages: `/` and `/posts`, rendered from the index provided via
//! context. Drafts are in the index but filtered here.

use std::collections::BTreeSet;

use content::{post_path, tag_filter_path, IndexEntry};
use leptos::prelude::*;

use crate::components::{page, section_label, TagFilter};

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

/// Every tag on a listed post, deduped and sorted — the filter pill set.
fn all_tags(entries: &[IndexEntry]) -> Vec<String> {
    entries
        .iter()
        .flat_map(|entry| entry.tags.iter())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .cloned()
        .collect()
}

/// Same pill shape as the article-bottom tags; the href is both the deep
/// link and the tag the filter island reads back out of the DOM.
fn filter_pill(tag: String) -> impl IntoView {
    let href = tag_filter_path(&tag);
    view! {
        <li>
            <a class="tag" href=href>
                <span class="tag-hash" aria-hidden="true">
                    "#"
                </span>
                {tag}
            </a>
        </li>
    }
}

/// The design's `$ ls` empty state; ships hidden — only the filter island
/// ever shows it, so no-JS readers never see it under the full list.
const FILTER_EMPTY: &str = "$ ls — nothing here yet";

#[component]
pub fn PostsPage() -> impl IntoView {
    let entries = listed_entries();
    let listing = if entries.is_empty() {
        empty_state(NOTHING_PUBLISHED.into()).into_any()
    } else {
        let tags = all_tags(&entries);
        let pills: Vec<_> = tags.into_iter().map(filter_pill).collect();
        let filter = (!pills.is_empty()).then(|| {
            view! {
                <TagFilter>
                    <ul class="post-tags mt-4.5">{pills}</ul>
                </TagFilter>
            }
        });
        let ls_empty = filter.is_some().then(|| {
            view! {
                <p class="filter-empty" hidden>
                    {FILTER_EMPTY}
                </p>
            }
        });
        view! {
            {filter}
            <div class="mt-8">{post_list(entries)}</div>
            {ls_empty}
        }
        .into_any()
    };
    page(Some("posts — chris".into()), "posts", listing)
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
