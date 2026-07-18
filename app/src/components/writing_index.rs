use std::collections::BTreeSet;

use content::RSS_PATH;
use leptos::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::UrlSearchParams;

use crate::components::{
    post_list, section_label, tag_row, ListedPost, TagPill, SECTION_LABEL_CLASS,
};

/// The writing front door's whole body: a two-panel split with the topics rail
/// on the left and the post list on the right, under a `writing (N) · rss`
/// header and a reserved search slot. It is one island so pill clicks toggle
/// row visibility through plain signal state; filtering is one behavior of the
/// index, not its whole job. The selection mirrors the `?q=` query — shareable,
/// restored on load, and never navigated to (clicks move the URL by
/// `replaceState`), so the server keeps rendering the one unfiltered listing
/// whatever the query says. Without JS the pills are inert links and the
/// complete list stays visible.
///
/// The search field above the list is a reserved slot: it renders F's look but
/// is inert until text search ships as its own feature.
#[island]
pub fn WritingIndex(posts: Vec<ListedPost>) -> impl IntoView {
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

    let total = posts.len();

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
    let rail = topics_rail(pills);

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
        <div class="mt-10 grid gap-12 md:grid-cols-[minmax(0,12rem)_minmax(0,1fr)] md:gap-0">
            <aside class="md:sticky md:top-24 md:self-start md:pr-10">{rail}</aside>
            <div class="min-w-0 md:border-l md:border-line md:pl-10">
                {writing_header(total)} <div class="mt-4">{search_field()}</div>
                {post_list(rows, "mt-6")} <Show when=none_visible>
                    <p class="mt-8 text-sm text-ink-3">"nothing here yet"</p>
                </Show>
            </div>
        </div>
    }
}

/// Above this many tags the rail clamps to a few rows behind a gradient fade
/// and a no-JS "show all". Below it the pills wrap freely — the fade would
/// otherwise float over blank space. Today's tag set sits well under it.
const RAIL_CLAMP: usize = 12;

/// The topics rail: the clean section label over the tag pills. `None` when
/// no post carries a tag, so the rail never shows a lonely label.
fn topics_rail<V: IntoView + 'static>(pills: Vec<V>) -> Option<impl IntoView> {
    let n = pills.len();
    if n == 0 {
        return None;
    }
    let body = if n > RAIL_CLAMP {
        clamped_topics(pills, n).into_any()
    } else {
        tag_row(pills, "mt-3").into_any()
    };
    Some(view! {
        {section_label("topics")}
        {body}
    })
}

/// Pills clamped to a few rows; a gradient overlay pinned to the clip's bottom
/// edge fades the cut so no half-row juts out, and a checkbox-peer "show all"
/// lifts both clamp and overlay (no JS).
fn clamped_topics<V: IntoView>(pills: Vec<V>, n: usize) -> impl IntoView {
    view! {
        <div class="relative mt-3">
            <input type="checkbox" id="topics-more" class="peer sr-only" />
            {tag_row(pills, "mt-0 max-h-[11rem] overflow-hidden peer-checked:max-h-[64rem]")}
            <div class="pointer-events-none absolute inset-x-0 top-[7rem] h-16 bg-gradient-to-t from-surface to-transparent peer-checked:hidden"></div>
            <label
                for="topics-more"
                class="mt-2 block cursor-pointer text-xs text-ink-2 hover:text-ink peer-checked:hidden"
            >
                {format!("show all {n} ↓")}
            </label>
            <label
                for="topics-more"
                class="mt-2 hidden cursor-pointer text-xs text-ink-2 hover:text-ink peer-checked:block"
            >
                "show less ↑"
            </label>
        </div>
    }
}

/// The writing header: the clean label with the post count, then the rss
/// link — "writing (N) · rss".
fn writing_header(total: usize) -> impl IntoView {
    view! {
        <p class="flex items-baseline gap-2 text-sm">
            <span class=SECTION_LABEL_CLASS>{format!("writing ({total})")}</span>
            <span class="text-ink-3" aria-hidden="true">
                "·"
            </span>
            <a href=RSS_PATH class="plink">
                "rss"
            </a>
        </p>
    }
}

/// The reserved search slot: F's pill-shaped, icon-led field. It keeps its
/// resting and focus affordances — on focus the magnifier warms to accent and
/// a discrete `line-2` border draws, the global focus ring suppressed — but no
/// filtering is wired: typing is inert until text search ships as its own
/// feature. The recessed fill is the `input-fill` token; the magnifier and
/// placeholder read `ink-2` for contrast.
fn search_field() -> impl IntoView {
    view! {
        <label class="group relative block">
            <span class="sr-only">"filter writing"</span>
            <svg
                viewBox="0 0 24 24"
                width="16"
                height="16"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                aria-hidden="true"
                class="pointer-events-none absolute left-4 top-1/2 h-4 w-4 -translate-y-1/2 text-ink-2 transition-colors group-focus-within:text-accent"
            >
                <circle cx="11" cy="11" r="7"></circle>
                <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
            </svg>
            <input
                type="search"
                placeholder="filter writing…"
                class="w-full appearance-none rounded-full border border-transparent bg-input-fill py-2.5 pl-11 pr-4 text-sm text-ink placeholder:text-ink-2 transition-colors focus:border-line-2 focus:outline-none focus-visible:outline-none"
            />
        </label>
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
