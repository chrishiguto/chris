//! Site-wide UI components; post-embeddable ones live under [`blog`].

use leptos::prelude::*;
use leptos_meta::Title;

pub mod blog;
pub mod copy_button;
pub mod footer;
pub mod header;
pub mod konami;
pub mod not_found;
pub mod tag_filter;
pub mod theme_toggle;

pub use copy_button::CopyButton;
pub use footer::Footer;
pub use header::Header;
pub use konami::Konami;
pub use not_found::NotFound;
pub use tag_filter::TagFilter;
pub use theme_toggle::ThemeToggle;

/// Meta-row content: formatted date, then `· N min` when the
/// read time is known — absent minutes render the date alone. The wrapper
/// (`p.post-meta` / `span.post-row-meta`) belongs to the caller.
pub(crate) fn meta_row(date: &str, minutes: Option<u32>) -> impl IntoView {
    let time = minutes.map(|minutes| {
        view! {
            <span class="meta-sep" aria-hidden="true">
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

/// Mono section label; shared by the home and about pages.
pub(crate) fn section_label(text: &'static str) -> impl IntoView {
    view! { <p class="font-mono text-xs tracking-wide text-ink-3">{text}</p> }
}

/// Tag pill: one `<li>` of a `.post-tags` row, shared by the
/// article bottom and the listing's filter row. Single-sourced because the
/// shape is a contract: the filter island reads the tag back out of the
/// href's fragment, and `a.tag` / `.tag-hash` key the CSS.
pub(crate) fn tag_pill(tag: String) -> impl IntoView {
    let href = content::tag_filter_path(&tag);
    view! {
        <li>
            <a class="tag" href=href>
                <span class="tag-hash" aria-hidden="true">
                    "#"
                </span>
                {tag}
            </a>
        </li>
    }
}

/// The page column every non-article page sizes to; shared so the geometry
/// can't drift between [`page`] and the about page's bespoke layout.
pub(crate) const PAGE_COLUMN: &str = "mx-auto max-w-2xl px-6 py-16";

/// Shared page scaffold; every page except the post article and the about
/// page (which opens with the prompt motif instead) renders through it.
pub(crate) fn page(
    title: Option<String>,
    heading: impl IntoView,
    body: impl IntoView,
) -> impl IntoView {
    view! {
        {title.map(|text| view! { <Title text=text /> })}
        <section class=PAGE_COLUMN>
            <h1 class="text-3xl font-semibold tracking-tight">{heading}</h1>
            {body}
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::format_date;

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
