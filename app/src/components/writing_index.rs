use std::collections::BTreeSet;

use content::RSS_PATH;
use leptos::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::UrlSearchParams;

use crate::components::{ListedPost, PostList, SectionLabel, TagPill, TagRow};

/// The writing front door's whole body: the post list under a
/// `writing (N) · rss` header and a reserved search slot, with the topics
/// rail beside it whenever a post carries a tag — a tagless index renders
/// the list full-width, no leftover column or divider. It is one island so
/// pill clicks toggle row visibility through plain signal state; filtering
/// is one behavior of the index, not its whole job. The selection mirrors
/// the `?q=` query — shareable, restored on load, and never navigated to
/// (clicks move the URL by `replaceState`), so the server keeps rendering
/// the one unfiltered listing whatever the query says. Restore keeps only
/// tags a listed post carries, so a stale deep link degrades to the full
/// list instead of blanking the front door. Without JS the pills are inert
/// links and the complete list stays visible.
///
/// The search field above the list is a reserved slot: it renders the real
/// field's look but is inert until text search ships as its own feature.
#[island]
pub fn WritingIndex(posts: Vec<ListedPost>) -> impl IntoView {
    let total = posts.len();

    let tags: BTreeSet<String> = posts
        .iter()
        .flat_map(|post| post.tags.iter().cloned())
        .collect();

    let active = RwSignal::new(BTreeSet::<String>::new());
    // Deep links land pre-filtered; effects never run during SSR. Unknown
    // tags drop here, so no selection can ever hide every row.
    Effect::new({
        let known = tags.clone();
        move |_| active.set(&query_tags() & &known)
    });

    let select = move |tag: String| {
        active.update(|active| {
            if !active.remove(&tag) {
                active.insert(tag);
            }
            replace_query(active);
        });
    };

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

    // The union semantics live here alone: a row hides when it carries
    // none of the selection.
    let hides = move |tags: &[String]| {
        active.with(|active| !active.is_empty() && !tags.iter().any(|tag| active.contains(tag)))
    };

    // The restore intersection keeps the selection inside the listed tags,
    // so nothing can empty the list; the guard stays should that loosen.
    let all_tags: Vec<String> = tags.into_iter().collect();
    let none_visible = move || hides(&all_tags);

    let rows: Vec<_> = posts
        .into_iter()
        .map(|post| {
            let tags = post.tags.clone();
            (post, Some(Signal::derive(move || hides(&tags))))
        })
        .collect();

    let panel = view! {
        <WritingHeader total=total />
        <div class="mt-4">
            <SearchField />
        </div>
        <PostList rows=rows spacing="mt-6" />
        <Show when=none_visible>
            <p class="mt-8 text-sm text-ink-3">"nothing here yet"</p>
        </Show>
    };
    if pills.is_empty() {
        view! { <div class="mt-10">{panel}</div> }.into_any()
    } else {
        view! {
            <div class="mt-10 grid gap-12 md:grid-cols-[minmax(0,12rem)_minmax(0,1fr)] md:gap-0">
                <aside class="md:sticky md:top-24 md:self-start md:pr-10">
                    <TopicsRail pills=pills />
                </aside>
                <div class="min-w-0 md:border-l md:border-line md:pl-10">{panel}</div>
            </div>
        }
        .into_any()
    }
}

/// Above this many tags the rail clamps to a few rows behind a gradient fade
/// and a no-JS "show all". Below it the pills wrap freely — the fade would
/// otherwise float over blank space. Today's tag set sits well under it.
const RAIL_CLAMP: usize = 12;

/// The topics rail: the clean section label over the tag pills. The caller
/// guards the tagless case; the empty return here keeps the rail total.
#[component]
fn TopicsRail<V: IntoView + 'static>(pills: Vec<V>) -> impl IntoView {
    let n = pills.len();
    if n == 0 {
        return None;
    }
    let body = if n > RAIL_CLAMP {
        view! { <ClampedTopics pills=pills n=n /> }.into_any()
    } else {
        view! { <TagRow pills=pills spacing="mt-3" /> }.into_any()
    };
    Some(view! {
        <SectionLabel>"topics"</SectionLabel>
        {body}
    })
}

/// Both clamp-toggle faces share one look — plus a visible ring while the
/// hidden checkbox holds keyboard focus; each face adds its visibility pair.
const TOGGLE_LABEL_CLASS: &str = "mt-2 cursor-pointer text-xs text-ink-2 hover:text-ink \
     peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 \
     peer-focus-visible:outline-(--focus-ring)";

/// Pills clamped to a few rows; a gradient overlay pinned to the clip's
/// bottom edge fades the cut so no half-row juts out, and a checkbox-peer
/// "show all" lifts clamp and overlay entirely (no JS, no expanded ceiling).
#[component]
fn ClampedTopics<V: IntoView + 'static>(pills: Vec<V>, n: usize) -> impl IntoView {
    view! {
        <div class="relative mt-3">
            <input type="checkbox" id="topics-more" class="peer sr-only" />
            <TagRow
                pills=pills
                spacing="mt-0 max-h-[11rem] overflow-hidden peer-checked:max-h-none"
            />
            <div class="pointer-events-none absolute inset-x-0 top-[7rem] h-16 bg-gradient-to-t from-surface to-transparent peer-checked:hidden"></div>
            <label
                for="topics-more"
                class=format!("{TOGGLE_LABEL_CLASS} block peer-checked:hidden")
            >
                {format!("show all {n} ↓")}
            </label>
            <label
                for="topics-more"
                class=format!("{TOGGLE_LABEL_CLASS} hidden peer-checked:block")
            >
                "show less ↑"
            </label>
        </div>
    }
}

/// The writing header: the clean label with the post count, then the rss
/// link — "writing (N) · rss".
#[component]
fn WritingHeader(total: usize) -> impl IntoView {
    view! {
        <div class="flex items-baseline gap-2 text-sm">
            <SectionLabel>{format!("writing ({total})")}</SectionLabel>
            <span class="text-ink-3" aria-hidden="true">
                "·"
            </span>
            <a href=RSS_PATH class="plink">
                "rss"
            </a>
        </div>
    }
}

/// The reserved search slot: the pill-shaped, icon-led field. It keeps its
/// resting and focus affordances — on focus the magnifier warms to accent and
/// a discrete `line-2` border draws, the global focus ring suppressed — but no
/// filtering is wired: typing is inert until text search ships as its own
/// feature. The recessed fill is the `input-fill` token; the magnifier and
/// placeholder read `ink-2` for contrast.
#[component]
fn SearchField() -> impl IntoView {
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
/// scroll — the URL mirrors the selection while every unrelated query param
/// (a campaign tag, a referrer) rides along untouched.
fn replace_query(tags: &BTreeSet<String>) {
    let url = content::tag_filter_path_selected(tags);
    let others = window()
        .location()
        .search()
        .ok()
        .and_then(|search| UrlSearchParams::new_with_str(&search).ok())
        .map(|params| {
            params.delete(content::TAG_FILTER_PARAM);
            String::from(params.to_string())
        })
        .unwrap_or_default();
    let url = if others.is_empty() {
        url
    } else if url.contains('?') {
        format!("{url}&{others}")
    } else {
        format!("{url}?{others}")
    };
    if let Ok(history) = window().history() {
        let _ = history.replace_state_with_url(&JsValue::NULL, "", Some(&url));
    }
}
