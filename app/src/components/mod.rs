//! Site-wide UI components; post-embeddable ones live under [`blog`].

use content::{post_path, IndexEntry};
use leptos::prelude::*;
use leptos_meta::Title;
use serde::{Deserialize, Serialize};

pub mod back_link;
pub mod blog;
pub mod code_block;
pub mod footer;
pub mod header;
pub mod konami;
pub mod not_found;
pub mod tag_filter;
pub mod theme_toggle;

pub use back_link::BackLink;
pub use code_block::CodeBlock;
pub use footer::Footer;
pub use header::Header;
pub use konami::Konami;
pub use not_found::NotFound;
pub use tag_filter::TagFilter;
pub use theme_toggle::ThemeToggle;

/// The article header's meta line: formatted date, then `· N min` when the
/// read time is known — absent minutes render the date alone.
pub(crate) fn post_meta(date: &str, minutes: Option<u32>) -> impl IntoView {
    view! { <p class="post-meta">{meta_row(date, minutes)}</p> }
}

/// Shared `date · minutes` content for the article meta line and the row
/// meta; the separator reads a step quieter than either side.
fn meta_row(date: &str, minutes: Option<u32>) -> impl IntoView {
    let time = minutes.map(|minutes| {
        view! {
            <span class="text-ink-3" aria-hidden="true">
                "·"
            </span>
            <span>{format!("{minutes} min")}</span>
        }
    });
    view! {
        <span>{format_date(date)}</span>
        {time}
    }
}

const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];

/// `YYYY-MM-DD` → `jul 04, 2026`. Anything off-shape passes through
/// unchanged — display formatting must never panic on stored data.
fn format_date(iso: &str) -> String {
    let parts: Vec<&str> = iso.split('-').collect();
    let [year, month, day] = parts[..] else {
        return iso.to_string();
    };
    if !(digits(year, 4) && digits(month, 2) && digits(day, 2)) {
        return iso.to_string();
    }
    month
        .parse::<usize>()
        .ok()
        .and_then(|m| m.checked_sub(1))
        .and_then(|m| MONTHS.get(m))
        .map_or_else(|| iso.to_string(), |name| format!("{name} {day}, {year}"))
}

fn digits(part: &str, len: usize) -> bool {
    part.len() == len && part.bytes().all(|byte| byte.is_ascii_digit())
}

/// `{page} — ~/chris`: every non-home tab title hangs off the site title.
pub(crate) fn page_title(page: &str) -> String {
    format!("{page} — {}", content::SITE_TITLE)
}

/// Mono section label; shared by the home and about pages.
pub(crate) fn section_label(text: &'static str) -> impl IntoView {
    view! { <p class="font-mono text-xs tracking-wide text-ink-3">{text}</p> }
}

/// One listed post, in the shape the pages render: the published subset of
/// an index entry. Internal fields (content hash, draft) never reach the
/// client — this is what the filter island serializes as props.
#[derive(Clone, Serialize, Deserialize)]
pub struct ListedPost {
    pub slug: String,
    pub title: String,
    pub date: String,
    pub reading_minutes: Option<u32>,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

impl From<IndexEntry> for ListedPost {
    fn from(entry: IndexEntry) -> Self {
        Self {
            slug: entry.slug,
            title: entry.title,
            date: entry.date,
            reading_minutes: entry.reading_minutes,
            description: entry.description,
            tags: entry.tags,
        }
    }
}

/// The whole listing shape — `ul.post-list > li > a.post-row` — declared
/// once. A row given a visibility signal hides reactively (the filter
/// island's live selection); without one it renders plain, so the server
/// baseline never carries `hidden`. Context spacing belongs to callers.
pub(crate) fn post_list(
    rows: Vec<(ListedPost, Option<Signal<bool>>)>,
    spacing: &'static str,
) -> impl IntoView {
    let items: Vec<_> = rows
        .into_iter()
        .map(|(post, hidden)| {
            view! { <li hidden=move || hidden.is_some_and(|hidden| hidden.get())>{post_row(post)}</li> }
        })
        .collect();
    view! { <ul class=format!("post-list {spacing}")>{items}</ul> }
}

/// The whole row is the link; the arrow slides in on hover via CSS.
fn post_row(post: ListedPost) -> impl IntoView {
    let meta = meta_row(&post.date, post.reading_minutes);
    view! {
        <a href=post_path(&post.slug) class="post-row">
            <span class="post-row-top">
                <span class="post-row-title">
                    {post.title} <span class="post-row-lead" aria-hidden="true">
                        "→"
                    </span>
                </span>
                <span class="post-row-meta">{meta}</span>
            </span>
            {post
                .description
                .map(|description| view! { <span class="post-row-desc">{description}</span> })}
        </a>
    }
}

/// The pill row — `ul.post-tags` of [`TagPill`]s — shared by the article
/// bottom and the filter; no pills, no row. Spacing overrides belong to
/// callers.
pub(crate) fn tag_row<V: IntoView>(pills: Vec<V>, spacing: &'static str) -> Option<impl IntoView> {
    let class = if spacing.is_empty() {
        "post-tags".to_string()
    } else {
        format!("post-tags {spacing}")
    };
    (!pills.is_empty()).then(|| view! { <ul class=class>{pills}</ul> })
}

/// Tag pill: one `<li>` of the [`tag_row`]. The filter island drives
/// `active` and `on_select`; rendered statically, the pill is a plain link
/// to the pre-filtered listing.
#[component]
pub(crate) fn TagPill(
    tag: String,
    #[prop(optional)] active: Option<Signal<bool>>,
    #[prop(optional)] on_select: Option<Callback<()>>,
) -> impl IntoView {
    let href = content::tag_filter_path(&tag);
    view! {
        <li>
            <a
                class="tag"
                class:tag-active=move || active.is_some_and(|active| active.get())
                href=href
                on:click=move |ev| {
                    if let Some(on_select) = on_select {
                        ev.prevent_default();
                        on_select.run(());
                    }
                }
            >
                <span class="tag-hash" aria-hidden="true">
                    "#"
                </span>
                {tag}
            </a>
        </li>
    }
}

/// Shared page scaffold; every page except the post article and the about
/// page (which opens with the prompt motif instead) renders through it.
pub(crate) fn page(
    title: Option<String>,
    heading: impl IntoView,
    body: impl IntoView,
) -> impl IntoView {
    view! {
        {title.map(|text| view! { <Title text=text /> })}
        <section class="page-enter mx-auto max-w-2xl px-6 py-16">
            <h1 class="text-3xl font-semibold tracking-tight">{heading}</h1>
            {body}
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::{format_date, page_title};

    // The literal pins the suffix shape; agreement with the tab and feed is
    // structural through `content::SITE_TITLE`.
    #[test]
    fn page_titles_hang_off_the_site_title() {
        assert_eq!(page_title("posts"), "posts — ~/chris");
    }

    #[test]
    fn dates_format_with_every_english_month_name() {
        for (i, name) in super::MONTHS.iter().enumerate() {
            assert_eq!(
                format_date(&format!("2026-{:02}-15", i + 1)),
                format!("{name} 15, 2026")
            );
        }
    }

    #[test]
    fn dates_keep_the_zero_padded_day() {
        assert_eq!(format_date("2026-07-04"), "jul 04, 2026");
        assert_eq!(format_date("2026-01-01"), "jan 01, 2026");
        assert_eq!(format_date("2026-12-31"), "dec 31, 2026");
    }

    // Display must never panic on stored data; anything off-shape passes through.
    #[test]
    fn malformed_dates_pass_through_unchanged() {
        for raw in [
            "someday",
            "",
            "2026-13-01",
            "2026-00-01",
            "2026-7-4",
            "2026-07",
        ] {
            assert_eq!(format_date(raw), raw);
        }
    }
}
