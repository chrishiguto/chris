use std::collections::BTreeSet;

use content::RSS_PATH;
use leptos::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::UrlSearchParams;

use super::post_meta::{format_post_date, post_year};
use crate::components::{HoverDateRow, ListedPost};

struct YearGroup {
    year: String,
    posts: Vec<ListedPost>,
}

/// The complete writing archive and its multi-tag union filter. The server
/// render is always the full list; hydration restores only known `?q=` tags
/// and mirrors later changes with `replaceState`, never navigation. Without
/// JS the tag words are inert links to the same `?q=` URLs.
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

    // Inert links at rest; once hydrated each word is a toggle, so it carries
    // the pressed state a multi-select filter needs (`aria-current` would
    // claim one current item).
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
                        role="button"
                        aria-pressed=move || if is_active.get() { "true" } else { "false" }
                        href=content::tag_filter_path(tag)
                        on:click=on_select
                    >
                        {tag.clone()}
                    </a>
                </li>
            }
        })
        .collect_view();

    // The union semantics live in `hides` alone, applied at every scope: a
    // row hides when it carries none of the selection, a year when none of
    // its rows' tags is selected, the page when no listed tag is. The restore
    // intersection keeps the selection inside the listed tags, so nothing can
    // empty the list; the page-level guard stays should that loosen.
    let all_tags: Vec<String> = tags.into_iter().collect();
    let none_visible = move || hides(active, &all_tags);

    let years = year_groups(posts)
        .into_iter()
        .map(|group| {
            let group_tags: Vec<String> = group
                .posts
                .iter()
                .flat_map(|post| post.tags.iter().cloned())
                .collect();
            let rows = group
                .posts
                .into_iter()
                .map(|post| {
                    let ListedPost {
                        slug,
                        title,
                        date,
                        tags,
                    } = post;
                    let href = content::post_path(&slug);
                    let date = format_post_date(&date, false);
                    view! {
                        <li hidden=move || hides(active, &tags)>
                            <HoverDateRow date=date href=href>
                                {title}
                            </HoverDateRow>
                        </li>
                    }
                })
                .collect_view();
            view! {
                <section class="writing-year" hidden=move || hides(active, &group_tags)>
                    <h3 class="writing-year-label tabular-nums">{group.year}</h3>
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
    active.with(|active| hidden_by(active, tags))
}

/// Union semantics: with a selection, anything carrying none of it hides;
/// with no selection, nothing does.
fn hidden_by(active: &BTreeSet<String>, tags: &[String]) -> bool {
    !active.is_empty() && !tags.iter().any(|tag| active.contains(tag))
}

/// The index arrives newest-first, so a year's posts are consecutive: fold
/// runs of the same year into one group, in that order.
fn year_groups(posts: Vec<ListedPost>) -> Vec<YearGroup> {
    posts
        .into_iter()
        .fold(Vec::<YearGroup>::new(), |mut groups, post| {
            let year = post_year(&post.date);
            match groups.last_mut() {
                Some(group) if group.year == year => group.posts.push(post),
                _ => groups.push(YearGroup {
                    year: year.to_string(),
                    posts: vec![post],
                }),
            }
            groups
        })
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{hidden_by, year_groups, ListedPost};

    fn post(slug: &str, date: &str, tags: &[&str]) -> ListedPost {
        ListedPost {
            slug: slug.into(),
            title: slug.into(),
            date: date.into(),
            tags: tags.iter().map(|tag| tag.to_string()).collect(),
        }
    }

    fn selection(tags: &[&str]) -> BTreeSet<String> {
        tags.iter().map(|tag| tag.to_string()).collect()
    }

    #[test]
    fn union_filter_hides_only_rows_carrying_none_of_the_selection() {
        let rust = ["rust".to_string()];
        assert!(
            !hidden_by(&selection(&[]), &rust),
            "no selection hides nothing"
        );
        assert!(!hidden_by(&selection(&["rust", "wasm"]), &rust));
        assert!(hidden_by(&selection(&["wasm"]), &rust));
        assert!(
            hidden_by(&selection(&["wasm"]), &[]),
            "an untagged row hides under any selection"
        );
    }

    #[test]
    fn newest_first_posts_fold_into_year_groups_in_order() {
        let groups = year_groups(vec![
            post("new", "2026-07-04", &[]),
            post("mid", "2026-01-01", &[]),
            post("old", "2025-02-01", &[]),
        ]);
        let shape: Vec<(&str, Vec<&str>)> = groups
            .iter()
            .map(|group| {
                (
                    group.year.as_str(),
                    group.posts.iter().map(|post| post.slug.as_str()).collect(),
                )
            })
            .collect();
        assert_eq!(
            shape,
            vec![("2026", vec!["new", "mid"]), ("2025", vec!["old"])]
        );
    }
}
