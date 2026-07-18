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
pub mod not_found;
pub mod theme_toggle;
pub mod writing_index;

pub use back_link::BackLink;
pub use code_block::CodeBlock;
pub use footer::Footer;
pub use header::Header;
pub use not_found::NotFound;
pub use theme_toggle::ThemeToggle;
pub use writing_index::WritingIndex;

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

/// The section-label type: semibold, tracked, `ink-2` for WCAG AA (the old
/// `text-xs`/`ink-3` pairing failed it). `section_label` prepends `text-sm`;
/// inline labels that set their own size (the writing header) reuse this
/// alone, so the AA-critical color can't drift between them.
pub(crate) const SECTION_LABEL_CLASS: &str = "font-semibold tracking-wide text-ink-2";

/// Small tracked section label; shared by the home rail and the about page.
/// Marker-free by design.
pub(crate) fn section_label(text: &'static str) -> impl IntoView {
    view! { <p class=format!("text-sm {SECTION_LABEL_CLASS}")>{text}</p> }
}

/// External contact link: no house underline — the arrow nudges outward on
/// hover instead, and parks under reduced motion. Wrapped by [`contacts`].
fn contact_link(href: &'static str, label: &'static str) -> impl IntoView {
    view! {
        <a
            href=href
            class="group inline-flex items-baseline gap-1.5 bg-none text-sm font-medium text-ink-2"
        >
            {label}
            <span
                class="inline-block transition-transform duration-200 ease-out-expo motion-safe:group-hover:translate-x-[2px] motion-safe:group-hover:-translate-y-[2px]"
                aria-hidden="true"
            >
                "↗"
            </span>
        </a>
    }
}

/// The external contact cluster shared by the home masthead and the about
/// page: the email line over the github + linkedin links. Hrefs are
/// well-formed mocks until real handles exist — kept here so they live in one
/// place. `lead` spaces the email line for its context.
pub(crate) fn contacts(lead: &'static str) -> impl IntoView {
    view! {
        <p class=lead>
            <a href="mailto:hi@chris.dev" class="text-sm font-medium">
                "hi@chris.dev"
            </a>
        </p>
        <div class="mt-3 flex gap-6">
            {contact_link("https://github.com/chris", "github")}
            {contact_link("https://www.linkedin.com/in/chris", "linkedin")}
        </div>
    }
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

/// `base` plus caller spacing utilities; no trailing space when empty.
fn classed(base: &'static str, spacing: &'static str) -> String {
    if spacing.is_empty() {
        base.to_string()
    } else {
        format!("{base} {spacing}")
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
            view! { <li hidden=move || hidden.is_some_and(|signal| signal.get())>{post_row(post)}</li> }
        })
        .collect();
    view! { <ul class=classed("post-list", spacing)>{items}</ul> }
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
    (!pills.is_empty()).then(|| view! { <ul class=classed("post-tags", spacing)>{pills}</ul> })
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

/// The page frame every route mounts into: the centered column carrying the
/// `page-enter` transition every page's mount is pinned to. The home listing
/// and the post article compose their own masthead into it directly; [`page`]
/// adds the display heading on top for the about and 404 pages.
pub(crate) fn page_shell(children: impl IntoView) -> impl IntoView {
    view! { <section class="page-enter mx-auto max-w-2xl px-6 py-16">{children}</section> }
}

/// The display-face page heading: Fraunces, the display size, tight tracking.
/// Shared by [`page`] and the home masthead so the two front-page headings
/// can't drift in face, size, or tracking.
pub(crate) const DISPLAY_HEADING_CLASS: &str =
    "font-display text-display font-semibold tracking-[-0.01em]";

/// [`page_shell`] plus a title and the display heading: the about and 404
/// pages render through it. The home listing and post article open with their
/// own masthead and use the bare shell instead.
pub(crate) fn page(
    title: Option<String>,
    heading: impl IntoView,
    body: impl IntoView,
) -> impl IntoView {
    view! {
        {title.map(|text| view! { <Title text=text /> })}
        {page_shell(
            view! {
                <h1 class=DISPLAY_HEADING_CLASS>{heading}</h1>
                {body}
            },
        )}
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
