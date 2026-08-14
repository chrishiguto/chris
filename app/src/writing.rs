//! The writing page `/writing`: the full listing under the filter island
//! ([`WritingIndex`]). Drafts are in the worker-provided index but filtered
//! here, so the island never serializes an unpublished post into its props.

use content::IndexEntry;
use leptos::prelude::*;
use leptos_meta::Title;

use crate::components::{page_title, ListedPost, PageShell, WritingIndex};

/// Per-request index from the site worker, newest-first.
#[derive(Clone)]
pub struct IndexData(pub Vec<IndexEntry>);

fn listed_posts() -> Vec<ListedPost> {
    use_context::<IndexData>()
        .map(|data| data.0)
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| entry.is_listed())
        .map(Into::into)
        .collect()
}

/// Shown when the index carries no published post.
#[component]
fn NothingPublished() -> impl IntoView {
    view! { <p class="mt-6 text-ink-2">"nothing published yet — check back soon."</p> }
}

/// The island opens with its own `writing (N) · rss` header, so the page
/// uses the bare shell — no display heading on top of it.
#[component]
pub fn WritingPage() -> impl IntoView {
    let posts = listed_posts();
    // A fixed per-request branch — outside islands nothing re-renders
    // client-side — so no reactive Show; type-erased to unify the arms, and
    // the island props move instead of cloning.
    let panel = if posts.is_empty() {
        view! { <NothingPublished /> }.into_any()
    } else {
        view! { <WritingIndex posts=posts /> }.into_any()
    };
    view! {
        <Title text=page_title("writing") />
        <PageShell>{panel}</PageShell>
    }
}
