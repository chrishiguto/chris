use leptos::prelude::*;

/// The article header's meta line: a natural-language date, then `· N min`
/// when the read time is known — absent minutes render the date alone.
#[component]
pub(crate) fn PostMeta(date: String, minutes: Option<u32>) -> impl IntoView {
    view! {
        <p class="post-meta">
            <span>{format_post_date(&date)}</span>
            <ReadTime minutes=minutes />
        </p>
    }
}

/// Listing-row `date · minutes` content; the separator reads a step quieter
/// than either side. Post headers use their own prose date above.
#[component]
pub(crate) fn MetaRow(date: String, minutes: Option<u32>) -> impl IntoView {
    view! {
        <span>{format_date(&date)}</span>
        <ReadTime minutes=minutes />
    }
}

#[component]
fn ReadTime(minutes: Option<u32>) -> impl IntoView {
    minutes.map(|minutes| {
        view! {
            <span class="text-ink-3" aria-hidden="true">
                "·"
            </span>
            <span>{format!("{minutes} min")}</span>
        }
    })
}

const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];

const MONTH_NAMES: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

/// The post header reads its date as prose: `2026-07-04` becomes
/// `4 july 2026`. Invalid stored values pass through unchanged.
fn format_post_date(iso: &str) -> String {
    let parts: Vec<&str> = iso.split('-').collect();
    let [year, month, day] = parts[..] else {
        return iso.to_string();
    };
    if !(digits(year, 4) && digits(month, 2) && digits(day, 2)) {
        return iso.to_string();
    }
    let Some(name) = month
        .parse::<usize>()
        .ok()
        .and_then(|m| m.checked_sub(1))
        .and_then(|m| MONTH_NAMES.get(m))
    else {
        return iso.to_string();
    };
    day.parse::<u8>()
        .ok()
        .filter(|day| *day > 0)
        .map_or_else(|| iso.to_string(), |day| format!("{day} {name} {year}"))
}

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

#[cfg(test)]
mod tests {
    use super::{format_date, format_post_date};

    #[test]
    fn post_dates_read_as_words_without_zero_padding() {
        assert_eq!(format_post_date("2026-07-04"), "4 july 2026");
        assert_eq!(format_post_date("2026-12-31"), "31 december 2026");
    }

    #[test]
    fn malformed_post_dates_pass_through_unchanged() {
        for raw in ["someday", "", "2026-13-01", "2026-07-00", "2026-7-4"] {
            assert_eq!(format_post_date(raw), raw);
        }
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
