use leptos::prelude::*;

/// The article header's meta line: formatted date, then `· N min` when the
/// read time is known — absent minutes render the date alone.
#[component]
pub(crate) fn PostMeta(date: String, minutes: Option<u32>) -> impl IntoView {
    view! {
        <p class="post-meta">
            <MetaRow date=date minutes=minutes />
        </p>
    }
}

/// Shared `date · minutes` content for the article meta line and the row
/// meta; the separator reads a step quieter than either side.
#[component]
pub(crate) fn MetaRow(date: String, minutes: Option<u32>) -> impl IntoView {
    let time = minutes.map(|minutes| {
        view! {
            <span class="text-ink-3" aria-hidden="true">
                "·"
            </span>
            <span>{format!("{minutes} min")}</span>
        }
    });
    view! {
        <span class="tabular-nums">{format_post_date(&date, true)}</span>
        {time}
    }
}

const MONTHS: [&str; 12] = [
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

/// `YYYY-MM-DD` → `4 july` inside a year group, or `4 july 2026` in
/// standalone metadata. Anything malformed passes through unchanged.
pub fn format_post_date(iso: &str, include_year: bool) -> String {
    let parts: Vec<&str> = iso.split('-').collect();
    let [year, month, day] = parts[..] else {
        return iso.to_string();
    };
    if !(digits(year, 4) && digits(month, 2) && digits(day, 2)) {
        return iso.to_string();
    }
    let Some(day) = day.parse::<u8>().ok().filter(|day| (1..=31).contains(day)) else {
        return iso.to_string();
    };
    month
        .parse::<usize>()
        .ok()
        .and_then(|m| m.checked_sub(1))
        .and_then(|m| MONTHS.get(m))
        .map_or_else(
            || iso.to_string(),
            |name| {
                if include_year {
                    format!("{day} {name} {year}")
                } else {
                    format!("{day} {name}")
                }
            },
        )
}

/// Career ranges read as prose rather than compact data.
pub fn format_year_range(start: u16, end: Option<u16>) -> String {
    match end {
        Some(end) => format!("{start} to {end}"),
        None => format!("since {start}"),
    }
}

fn digits(part: &str, len: usize) -> bool {
    part.len() == len && part.bytes().all(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::{format_post_date, format_year_range};

    #[test]
    fn dates_format_with_every_english_month_name() {
        for (i, name) in super::MONTHS.iter().enumerate() {
            assert_eq!(
                format_post_date(&format!("2026-{:02}-15", i + 1), false),
                format!("15 {name}")
            );
        }
    }

    #[test]
    fn standalone_dates_include_the_year_without_zero_padding() {
        assert_eq!(format_post_date("2026-07-04", true), "4 july 2026");
        assert_eq!(format_post_date("2026-01-01", true), "1 january 2026");
        assert_eq!(format_post_date("2026-12-31", true), "31 december 2026");
    }

    #[test]
    fn ranges_distinguish_current_and_closed_work() {
        assert_eq!(format_year_range(2025, None), "since 2025");
        assert_eq!(format_year_range(2022, Some(2025)), "2022 to 2025");
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
            assert_eq!(format_post_date(raw, true), raw);
        }
    }
}
