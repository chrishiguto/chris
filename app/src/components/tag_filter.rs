use std::collections::BTreeSet;

use leptos::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::UrlSearchParams;

use crate::components::{post_list, ListedPost, TagPill};

/// In-page tag filter for the writing page: one island owning the pill row,
/// the post list, and the `$ ls` empty state, so filtering is plain signal
/// state. The selection mirrors the `?q=` query — shareable, restored on
/// load, and never navigated to (clicks move the URL by `replaceState`), so
/// the server keeps rendering the one unfiltered listing whatever the query
/// says. The island's server render is that unfiltered list: without JS the
/// pills are inert links and the complete list stays visible.
#[island]
pub fn TagFilter(posts: Vec<ListedPost>) -> impl IntoView {
    let active = RwSignal::new(BTreeSet::<String>::new());
    // Deep links land pre-filtered; effects never run during SSR.
    Effect::new(move |_| active.set(query_tags()));

    let select = move |tag: String| {
        active.update(|active| {
            if !active.remove(&tag) {
                active.insert(tag);
            }
            replace_query(active);
        });
    };

    let tags: BTreeSet<String> = posts
        .iter()
        .flat_map(|post| post.tags.iter().cloned())
        .collect();

    let pills: Vec<_> = tags
        .iter()
        .map(|tag| {
            let is_active = Signal::derive({
                let tag = tag.clone();
                move || active.with(|active| active.contains(&tag))
            });
            let on_select = Callback::new({
                let tag = tag.clone();
                move |()| select(tag.clone())
            });
            view! { <TagPill tag=tag.clone() active=is_active on_select=on_select /> }
        })
        .collect();
    let pill_row = (!pills.is_empty()).then(|| view! { <ul class="post-tags mt-4.5">{pills}</ul> });

    // The union semantics live here alone: a row hides when it carries
    // none of the selection.
    let hides = move |tags: &[String]| {
        active.with(|active| !active.is_empty() && !tags.iter().any(|tag| active.contains(tag)))
    };

    // Hiding the union of every tag hides every row, so pill clicks can
    // never empty the list; only a deep link naming unknown tags can.
    let all_tags: Vec<String> = tags.into_iter().collect();
    let none_visible = move || hides(&all_tags);

    let rows: Vec<_> = posts
        .into_iter()
        .map(|post| {
            let tags = post.tags.clone();
            (post, Some(Signal::derive(move || hides(&tags))))
        })
        .collect();

    view! {
        {pill_row}
        {post_list(rows, "mt-8")}
        <Show when=none_visible>
            <p class="filter-empty">"$ ls — nothing here yet"</p>
        </Show>
    }
}

/// The active selection: every tag the URL's filter query names; the
/// browser's own query parser does the decoding.
fn query_tags() -> BTreeSet<String> {
    let Ok(params) = window()
        .location()
        .search()
        .and_then(|search| UrlSearchParams::new_with_str(&search))
    else {
        return BTreeSet::new();
    };
    content::tag_filter_selection(
        params
            .get_all(content::TAG_FILTER_PARAM)
            .iter()
            .filter_map(|value| value.as_string()),
    )
}

/// `replaceState`, not navigation: no history entry per click and no
/// scroll — the URL just mirrors the current selection.
fn replace_query(tags: &BTreeSet<String>) {
    let url = content::tag_filter_path_selected(tags);
    if let Ok(history) = window().history() {
        let _ = history.replace_state_with_url(&JsValue::NULL, "", Some(&url));
    }
}
