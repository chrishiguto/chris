//! The home page `/`: the writing front door. A masthead identity band over
//! the two-panel writing index (the [`WritingIndex`] island). Drafts are in
//! the index provided via context but filtered here.

use content::IndexEntry;
use leptos::prelude::*;

use crate::components::{contacts, page_shell, ListedPost, WritingIndex, DISPLAY_HEADING_CLASS};

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
fn nothing_published() -> impl IntoView {
    view! { <p class="mt-6 text-ink-2">"nothing published yet — check back soon."</p> }
}

/// The front-door band: greeting, one voice line, external-only contacts. Nav
/// owns "about", so the masthead carries no in-app links.
fn masthead_band() -> impl IntoView {
    view! {
        <header class="border-b border-line pb-10">
            <h1 class=DISPLAY_HEADING_CLASS>"hey, i’m chris"</h1>
            <p class="mt-5 max-w-[58ch] text-lg leading-relaxed text-ink-2">
                "software engineer. this is everything i’m writing — code, systems, and figuring things out, in english e às vezes em português."
            </p>
            {contacts("mt-6")}
        </header>
    }
}

#[component]
pub fn HomePage() -> impl IntoView {
    let posts = listed_posts();
    let has_posts = !posts.is_empty();
    // Type-erased: the island nested through Show into the section overflows
    // rustc's query depth otherwise.
    let panel = view! {
        <Show when=move || has_posts fallback=nothing_published>
            <WritingIndex posts=posts.clone() />
        </Show>
    }
    .into_any();
    page_shell(view! {
        {masthead_band()}
        {panel}
    })
}
