use leptos::prelude::*;

/// The article header's meta line: the date in words, then `· N min` when
/// the read time is known — absent minutes render the date alone. The
/// separator reads a step quieter than either side.
#[component]
pub(crate) fn PostMeta(date: String, minutes: Option<u32>) -> impl IntoView {
    let time = minutes.map(|minutes| {
        view! {
            <span class="text-ink-3" aria-hidden="true">
                "·"
            </span>
            <span>{format!("{minutes} min")}</span>
        }
    });
    view! {
        <p class="post-meta">
            <span class="tabular-nums">{format_post_date(&date, true)}</span>
            {time}
        </p>
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

/// The year a stored date falls in, for grouping. Same policy as
/// [`format_post_date`]: anything off-shape passes through unchanged rather
/// than growing an invented label.
pub fn post_year(iso: &str) -> &str {
    match iso.get(..4) {
        Some(year) if digits(year, 4) && iso.as_bytes().get(4) == Some(&b'-') => year,
        _ => iso,
    }
}

fn digits(part: &str, len: usize) -> bool {
    part.len() == len && part.bytes().all(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::{format_post_date, post_year};

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
    fn years_come_from_well_formed_dates_only() {
        assert_eq!(post_year("2026-07-04"), "2026");
        for raw in ["someday", "2026", "20260704", "abcd-07-04"] {
            assert_eq!(post_year(raw), raw);
        }
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
