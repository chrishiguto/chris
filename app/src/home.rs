//! The index home: authored identity and work history beside a four-post
//! window into the runtime index. All copy and career data live here.

use content::IndexEntry;
use leptos::prelude::*;

use crate::components::post_meta::format_post_date;
use crate::components::{Fold, GhostWord, HoverDateRow};
use crate::writing::IndexData;

struct Stint {
    start: u16,
    end: Option<u16>,
    role: &'static str,
    company: &'static str,
    honest: Option<&'static str>,
}

const STINTS: &[Stint] = &[
    Stint {
        start: 2022,
        end: None,
        role: "lead frontend",
        company: "concepta tech",
        honest: Some("person who asks why"),
    },
    Stint {
        start: 2021,
        end: Some(2022),
        role: "frontend engineer",
        company: "ingaia",
        honest: None,
    },
    Stint {
        start: 2020,
        end: Some(2021),
        role: "software engineer",
        company: "clubpetro",
        honest: None,
    },
    Stint {
        start: 2019,
        end: Some(2020),
        role: "frontend developer",
        company: "navalabs",
        honest: None,
    },
    Stint {
        start: 2018,
        end: Some(2019),
        role: "web developer",
        company: "a small studio",
        honest: None,
    },
    Stint {
        start: 2017,
        end: Some(2020),
        role: "computer science",
        company: "grupo anchieta",
        honest: None,
    },
];

/// Career ranges read as prose: `since 2022` for the open stint, `2019 to
/// 2020` for a closed one.
fn format_year_range(start: u16, end: Option<u16>) -> String {
    match end {
        Some(end) => format!("{start} to {end}"),
        None => format!("since {start}"),
    }
}

fn listed_posts() -> Vec<IndexEntry> {
    use_context::<IndexData>()
        .map(|data| data.listed().collect())
        .unwrap_or_default()
}

/// A polite phrase that hover, focus, or a tap strikes through while the
/// honest one is written above the line. The insertion is decorative for
/// assistive tech; the polite phrase stays the readable text.
///
/// Hover and focus are pure CSS (`app/style/home.css`). Touch has neither, so
/// the tap toggle is the island's only job: one signal driving `is-revealed`,
/// the third selector in those same rules.
#[island]
fn HonestEdit(original: String, honest: String) -> impl IntoView {
    let (revealed, set_revealed) = signal(false);
    view! {
        <span
            class="honest-edit"
            class:is-revealed=move || revealed.get()
            tabindex="0"
            on:click=move |_| set_revealed.update(|on| *on = !*on)
        >
            <span class="honest-original">{original}</span>
            <span class="honest-insertion" aria-hidden="true">
                {honest}
            </span>
        </span>
    }
}

#[component]
fn WorkRow(stint: &'static Stint, #[prop(default = true)] focusable: bool) -> impl IntoView {
    let date = format_year_range(stint.start, stint.end);
    let role = match stint.honest {
        Some(honest) => {
            view! { <HonestEdit original=stint.role.to_string() honest=honest.to_string() /> }
                .into_any()
        }
        None => stint.role.into_any(),
    };
    view! {
        <HoverDateRow date=date current=stint.end.is_none() focusable=focusable>
            {role}
            " at "
            {stint.company}
        </HoverDateRow>
    }
}

#[component]
fn Work() -> impl IntoView {
    view! {
        <section class="ghost-section">
            <GhostWord label="work" />
            <div class="hover-date-list">
                {STINTS[..3].iter().map(|stint| view! { <WorkRow stint=stint /> }).collect_view()}
                <Fold label="show earlier work"
                    .to_string()>
                    {STINTS[3..]
                        .iter()
                        .map(|stint| view! { <WorkRow stint=stint focusable=false /> })
                        .collect_view()}
                </Fold>
            </div>
        </section>
    }
}

#[component]
fn Writing(posts: Vec<IndexEntry>) -> impl IntoView {
    let total = posts.len();
    let rows = posts
        .into_iter()
        .take(4)
        .map(|post| {
            let date = format_post_date(&post.date, false);
            let href = content::post_path(&post.slug);
            view! {
                <HoverDateRow date=date href=href>
                    {post.title}
                </HoverDateRow>
            }
        })
        .collect_view();
    view! {
        <section class="ghost-section">
            <GhostWord label="writing" outer=true />
            <div class="hover-date-list">{rows}</div>
            <a class="all-writing" href=content::WRITING_PATH>
                {format!("all writing ({total})")}
            </a>
        </section>
    }
}

#[component]
pub fn HomePage() -> impl IntoView {
    let posts = listed_posts();
    view! {
        <div class="home-index page-grid">
            <div class="home-index-content page-column page-enter">
                <header class="home-intro">
                    <h1>
                        <span>"christiano higuto"</span>
                        <span>"software engineer · são paulo"</span>
                    </h1>
                    <div class="home-intro-copy">
                        <p>
                            "i build products end to end and "
                            <span class="pencil">"keep asking what can be simpler"</span>
                            ". say hello by " <a class="plink" href="mailto:chrisshiguto@gmail.com">
                                "email"
                            </a> " or read the "
                            <a class="plink" href="https://github.com/chrishiguto/chris">
                                "code"
                            </a> "."
                        </p>
                        <p>
                            "this is my notebook for code, systems, and "
                            <HonestEdit
                                original="figuring things out".to_string()
                                honest="getting things wrong in public".to_string()
                            /> ", in english e às vezes em português."
                        </p>
                    </div>
                </header>
                <div class="home-spread">
                    <Work />
                    <Writing posts=posts />
                </div>
                <section class="ghost-section now-section">
                    <GhostWord label="now" />
                    <p>
                        "building small tools, learning rust slowly, and leaving room for long walks."
                    </p>
                    <p class="last-touched tabular-nums">"last touched 2 september 2026"</p>
                </section>
            </div>
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::format_year_range;

    #[test]
    fn ranges_distinguish_current_and_closed_work() {
        assert_eq!(format_year_range(2025, None), "since 2025");
        assert_eq!(format_year_range(2022, Some(2025)), "2022 to 2025");
    }
}
