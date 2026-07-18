use content::{post_path, IndexEntry};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use super::post_meta::MetaRow;
use crate::classed::classed;

/// One listed post, in the shape the pages render: the published subset of
/// an index entry. Internal fields (content hash, draft) never reach the
/// client — this is what the filter island serializes as props.
#[derive(Clone, Serialize, Deserialize)]
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

/// The whole listing shape — `ul.post-list > li > a.post-row` — declared
/// once. A row given a visibility signal hides reactively (the filter
/// island's live selection); without one it renders plain, so the server
/// baseline never carries `hidden`. Context spacing belongs to callers.
#[component]
pub(crate) fn PostList(
    rows: Vec<(ListedPost, Option<Signal<bool>>)>,
    spacing: &'static str,
) -> impl IntoView {
    let items: Vec<_> = rows
        .into_iter()
        .map(|(post, hidden)| {
            view! {
                <li hidden=move || hidden.is_some_and(|signal| signal.get())>
                    <PostRow post=post />
                </li>
            }
        })
        .collect();
    view! { <ul class=classed("post-list", spacing)>{items}</ul> }
}

/// The whole row is the link; the arrow slides in on hover via CSS.
#[component]
fn PostRow(post: ListedPost) -> impl IntoView {
    view! {
        <a href=post_path(&post.slug) class="post-row">
            <span class="post-row-top">
                <span class="post-row-title">
                    {post.title} <span class="post-row-lead" aria-hidden="true">
                        "→"
                    </span>
                </span>
                <span class="post-row-meta">
                    <MetaRow date=post.date minutes=post.reading_minutes />
                </span>
            </span>
            {post
                .description
                .map(|description| view! { <span class="post-row-desc">{description}</span> })}
        </a>
    }
}
