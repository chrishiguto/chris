use std::collections::BTreeSet;

use content::RSS_PATH;
use leptos::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::UrlSearchParams;

use super::post_meta::format_post_date;
use crate::components::{HoverDateRow, ListedPost};

struct YearGroup {
    year: String,
    posts: Vec<ListedPost>,
}

/// The complete writing archive and its multi-tag union filter. The server
/// render is always the full list; hydration restores only known `?q=` tags
/// and mirrors later changes with `replaceState`, never navigation.
#[island]
pub fn WritingIndex(posts: Vec<ListedPost>) -> impl IntoView {
    let total = posts.len();
    let tags: BTreeSet<String> = posts
        .iter()
        .flat_map(|post| post.tags.iter().cloned())
        .collect();
    let active = RwSignal::new(BTreeSet::<String>::new());

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

    let tag_words = tags
        .iter()
        .map(|tag| {
            let is_active = Signal::derive({
                let tag = tag.clone();
                move || active.with(|active| active.contains(&tag))
            });
            let on_select = {
                let tag = tag.clone();
                move |ev: leptos::ev::MouseEvent| {
                    ev.prevent_default();
                    select(tag.clone());
                }
            };
            view! {
                <li>
                    <a
                        class="writing-tag"
                        class:writing-tag-active=move || is_active.get()
                        aria-current=move || is_active.get().then_some("true")
                        href=content::tag_filter_path(tag)
                        on:click=on_select
                    >
                        {tag.clone()}
                    </a>
                </li>
            }
        })
        .collect_view();

    let groups = year_groups(posts);
    let visible = groups
        .iter()
        .flat_map(|group| group.posts.iter())
        .map(|post| {
            let tags = post.tags.clone();
            Signal::derive(move || !hides(active, &tags))
        })
        .collect::<Vec<_>>();
    let none_visible = {
        let visible = visible.clone();
        move || !visible.iter().any(|row| row.get())
    };

    let years = groups
        .into_iter()
        .map(|group| {
            let group_visible = group
                .posts
                .iter()
                .map(|post| {
                    let tags = post.tags.clone();
                    Signal::derive(move || !hides(active, &tags))
                })
                .collect::<Vec<_>>();
            let rows = group
                .posts
                .into_iter()
                .zip(group_visible.iter().copied())
                .map(|(post, is_visible)| {
                    let href = content::post_path(&post.slug);
                    let date = format_post_date(&post.date, false);
                    view! {
                        <li hidden=move || !is_visible.get()>
                            <HoverDateRow date=date href=href>
                                {post.title}
                            </HoverDateRow>
                        </li>
                    }
                })
                .collect_view();
            view! {
                <section
                    class="writing-year"
                    hidden=move || !group_visible.iter().any(|row| row.get())
                >
                    <h2 class="writing-year-label tabular-nums">{group.year}</h2>
                    <ul class="writing-year-rows hover-date-list">{rows}</ul>
                </section>
            }
        })
        .collect_view();

    view! {
        <p class="writing-intro">
            {format!("{total} {} · ", if total == 1 { "post" } else { "posts" })}
            <a class="plink" href=RSS_PATH>
                "rss"
            </a>
        </p>
        <ul class="writing-tags" aria-label="filter by tag">
            {tag_words}
        </ul>
        <div class="writing-years">{years}</div>
        <Show when=none_visible>
            <p class="writing-empty">"nothing here yet"</p>
        </Show>
    }
}

fn hides(active: RwSignal<BTreeSet<String>>, tags: &[String]) -> bool {
    active.with(|active| !active.is_empty() && !tags.iter().any(|tag| active.contains(tag)))
}

fn year_groups(posts: Vec<ListedPost>) -> Vec<YearGroup> {
    let mut groups: Vec<YearGroup> = Vec::new();
    for post in posts {
        let year = post
            .date
            .get(..4)
            .filter(|year| year.bytes().all(|byte| byte.is_ascii_digit()))
            .unwrap_or("undated")
            .to_string();
        match groups.iter_mut().find(|group| group.year == year) {
            Some(group) => group.posts.push(post),
            None => groups.push(YearGroup {
                year,
                posts: vec![post],
            }),
        }
    }
    groups
}

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
