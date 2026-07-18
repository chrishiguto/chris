//! The home page `/`: the writing front door. A masthead identity band over
//! the two-panel writing index (the [`WritingIndex`] island). Drafts are in
//! the index provided via context but filtered here.

use content::IndexEntry;
use leptos::prelude::*;

use crate::components::{Contacts, ListedPost, PageShell, WritingIndex, DISPLAY_HEADING_CLASS};

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

/// The front-door band: greeting, one voice line, external-only contacts. Nav
/// owns "about", so the masthead carries no in-app links.
#[component]
fn MastheadBand() -> impl IntoView {
    view! {
        <header class="border-b border-line pb-10">
            <h1 class=DISPLAY_HEADING_CLASS>"hey, i’m chris"</h1>
            <p class="mt-5 max-w-[58ch] text-lg leading-relaxed text-ink-2">
                "software engineer. this is everything i’m writing — code, systems, and figuring things out, in english e às vezes em português."
            </p>
            <Contacts lead="mt-6" />
        </header>
    }
}

#[component]
pub fn HomePage() -> impl IntoView {
    let posts = listed_posts();
    // A fixed per-request branch — outside islands nothing re-renders
    // client-side — so no reactive Show, and the island props move instead
    // of cloning. Type-erased: the island nested into the section overflows
    // rustc's query depth otherwise.
    let panel = if posts.is_empty() {
        view! { <NothingPublished /> }.into_any()
    } else {
        view! { <WritingIndex posts=posts /> }.into_any()
    };
    view! {
        <PageShell>
            <MastheadBand />
            {panel}
        </PageShell>
    }
}
