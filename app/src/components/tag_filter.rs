use std::collections::BTreeSet;

use leptos::prelude::*;
use wasm_bindgen::JsValue;

use crate::components::TagPill;
use crate::listing::{post_row, ListedPost};

/// In-page tag filter for the writing page: one island owning the pill row,
/// the post list, and the `$ ls` empty state, so filtering is plain signal
/// state. The active tag mirrors the URL hash — shareable, restored on
/// load, and invisible to the server, so the cache still sees exactly one
/// `/posts` page. The island's server render is the unfiltered list:
/// without JS the pills are inert links and the complete list stays
/// visible.
#[island]
pub fn TagFilter(posts: Vec<ListedPost>) -> impl IntoView {
    let active = RwSignal::new(None::<String>);
    // Deep links land pre-filtered; effects never run during SSR.
    Effect::new(move |_| active.set(hash_tag()));

    // Toggle the tag and mirror it into the hash.
    let select = move |tag: String| {
        let next = (active.get_untracked().as_deref() != Some(tag.as_str())).then_some(tag);
        replace_hash(next.as_deref());
        active.set(next);
    };

    let tags: BTreeSet<String> = posts
        .iter()
        .flat_map(|post| post.tags.iter().cloned())
        .collect();
    // A deep-linked hash can name a tag no post carries; only then is the
    // list empty, since pill clicks always come from a post's own tags.
    let known = tags.clone();
    let none_visible = move || active.get().is_some_and(|tag| !known.contains(&tag));

    let pills: Vec<_> = tags
        .into_iter()
        .map(|tag| {
            let is_active = Signal::derive({
                let tag = tag.clone();
                move || active.get().as_deref() == Some(tag.as_str())
            });
            let on_select = Callback::new({
                let tag = tag.clone();
                move |()| select(tag.clone())
            });
            view! { <TagPill tag=tag active=is_active on_select=on_select /> }
        })
        .collect();
    let pill_row = (!pills.is_empty()).then(|| view! { <ul class="post-tags mt-4.5">{pills}</ul> });

    let rows: Vec<_> = posts
        .into_iter()
        .map(|post| {
            let tags = post.tags.clone();
            let hidden = move || active.get().is_some_and(|tag| !tags.contains(&tag));
            view! { <li hidden=hidden>{post_row(post)}</li> }
        })
        .collect();

    view! {
        {pill_row}
        <div class="mt-8">
            <ul class="post-list">{rows}</ul>
        </div>
        <Show when=none_visible>
            <p class="filter-empty">"$ ls — nothing here yet"</p>
        </Show>
    }
}

/// The active tag: the URL hash's fragment, `None` when empty.
fn hash_tag() -> Option<String> {
    let hash = window().location().hash().ok()?;
    content::tag_filter_tag(&hash).map(str::to_string)
}

/// `replaceState`, not `location.hash`: no history entry per click and no
/// fragment scroll — the URL just mirrors the current filter.
fn replace_hash(tag: Option<&str>) {
    let url = match tag {
        Some(tag) => content::tag_filter_path(tag),
        None => match window().location().pathname() {
            Ok(path) => path,
            Err(_) => return,
        },
    };
    if let Ok(history) = window().history() {
        let _ = history.replace_state_with_url(&JsValue::NULL, "", Some(&url));
    }
}
