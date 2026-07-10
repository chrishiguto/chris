use leptos::prelude::*;
use wasm_bindgen::{JsCast, JsValue};

/// In-page tag filter for the writing page (ADR-0012). The island owns the
/// pill row it wraps and serializes nothing: the active tag is the URL hash,
/// each pill's tag is the fragment of its own SSR'd href, and rows are
/// matched by their `data-tags` — all read out of the DOM. The server and
/// cache still see exactly one `/posts` page; without JS the pills are inert
/// anchors and the complete SSR list stays visible.
#[island]
pub fn TagFilter(children: Children) -> impl IntoView {
    // Deep links land pre-filtered; effects never run during SSR.
    Effect::new(|_| apply(hash_tag().as_deref()));
    view! {
        <div class="tag-filter" on:click=toggle>
            {children()}
        </div>
    }
}

/// Pill clicks are delegated from the island's wrapper, so the SSR'd pills
/// need no handlers of their own: toggle the tag, mirror it into the hash,
/// refilter.
fn toggle(ev: web_sys::MouseEvent) {
    let Some(pill) = ev
        .target()
        .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
        .and_then(|el| el.closest("a.tag").ok().flatten())
    else {
        return;
    };
    ev.prevent_default();
    let Some(tag) = pill_tag(&pill) else { return };
    let next = (hash_tag().as_deref() != Some(&tag)).then_some(tag);
    replace_hash(next.as_deref());
    apply(next.as_deref());
}

/// Shows exactly the rows carrying `tag` (every row when `None`), marks the
/// matching pill active, and swaps the `$ ls` empty state in when nothing is
/// left. Filter state is DOM attributes only, so toggling off restores the
/// SSR baseline.
fn apply(tag: Option<&str>) {
    let mut visible = 0;
    for row in elements("ul.post-list > li[data-tags]") {
        let tags = row.get_attribute("data-tags").unwrap_or_default();
        let shown = tag.is_none_or(|tag| tags.split_whitespace().any(|t| t == tag));
        set_hidden(&row, !shown);
        visible += usize::from(shown);
    }
    for pill in elements(".tag-filter a.tag") {
        let active = tag.is_some() && pill_tag(&pill).as_deref() == tag;
        let _ = pill.class_list().toggle_with_force("tag-active", active);
    }
    if let Ok(Some(empty)) = document().query_selector(".filter-empty") {
        set_hidden(&empty, visible != 0);
    }
}

/// The active tag: the URL hash minus its `#`, `None` when empty.
fn hash_tag() -> Option<String> {
    let hash = window().location().hash().ok()?;
    let tag = hash.strip_prefix('#').unwrap_or(&hash);
    (!tag.is_empty()).then(|| tag.to_string())
}

/// A pill's tag is the hash fragment of its own href — never a second copy.
fn pill_tag(pill: &web_sys::Element) -> Option<String> {
    let href = pill.get_attribute("href")?;
    let (_, tag) = href.split_once('#')?;
    (!tag.is_empty()).then(|| tag.to_string())
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

fn set_hidden(el: &web_sys::Element, hidden: bool) {
    if hidden {
        let _ = el.set_attribute("hidden", "");
    } else {
        let _ = el.remove_attribute("hidden");
    }
}

/// Every element matching `selector`; empty on any DOM error.
fn elements(selector: &str) -> Vec<web_sys::Element> {
    document()
        .query_selector_all(selector)
        .map(|list| {
            (0..list.length())
                .filter_map(|i| list.get(i).and_then(|node| node.dyn_into().ok()))
                .collect()
        })
        .unwrap_or_default()
}
