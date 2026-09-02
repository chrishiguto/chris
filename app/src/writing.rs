//! The `/writing` archive. The worker supplies the newest-first index; this
//! boundary removes drafts before the filter island serializes its props.

use content::IndexEntry;
use leptos::prelude::*;

use leptos_meta::Title;

use crate::components::{page_title, GhostWord, GutterNav, ListedPost, WritingIndex};

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

#[component]
pub fn WritingPage() -> impl IntoView {
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
        <Title text=page_title("writing") />
        <section class="page-grid writing-page">
            <GutterNav href=content::HOME_PATH label="home" />
            <div class="page-column page-enter">
                <section class="ghost-section writing-archive">
                    <GhostWord label="writing" />
                    {panel}
                </section>
            </div>
        </section>
    }
}
