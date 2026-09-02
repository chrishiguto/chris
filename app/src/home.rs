//! The index home: authored identity and work history beside a four-post
//! window into the runtime index. All copy and career data live here.

use content::IndexEntry;
use leptos::prelude::*;

use crate::components::post_meta::{format_post_date, format_year_range};
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

const HOME_SCRIPT: &str = r#"
document.currentScript.closest('.home-index').addEventListener('click', (event) => {
  const fold = event.target.closest('.home-fold-button');
  if (fold) {
    const root = fold.closest('.home-fold');
    root.classList.add('is-open');
    fold.setAttribute('aria-expanded', 'true');
    fold.disabled = true;
    root.querySelectorAll('.hover-date-row').forEach((row) => row.tabIndex = 0);
  }
  const edit = event.target.closest('.honest-edit');
  if (edit) edit.classList.toggle('is-revealed');
});
"#;

fn listed_posts() -> Vec<IndexEntry> {
    use_context::<IndexData>()
        .map(|data| data.0)
        .unwrap_or_default()
        .into_iter()
        .filter(IndexEntry::is_listed)
        .collect()
}

#[component]
fn WorkRow(stint: &'static Stint, #[prop(default = false)] folded: bool) -> impl IntoView {
    let date = format_year_range(stint.start, stint.end);
    let words = match stint.honest {
        Some(honest) => view! {
            <span class="honest-edit" tabindex="0">
                <span class="honest-original">{stint.role}</span>
                <span class="honest-insertion" aria-hidden="true">
                    {honest}
                </span>
            </span>
            " at "
            {stint.company}
        }
        .into_any(),
        None => view! {
            {stint.role}
            " at "
            {stint.company}
        }
        .into_any(),
    };
    view! {
        <HoverDateRow date=date current=stint.end.is_none() focusable=!folded>
            {words}
        </HoverDateRow>
    }
    .into_any()
}

#[component]
fn Work() -> impl IntoView {
    view! {
        <section class="ghost-section home-index-section">
            <GhostWord label="work" />
            <div class="hover-date-list">
                {STINTS[..3].iter().map(|stint| view! { <WorkRow stint=stint /> }).collect_view()}
                <Fold>
                    {STINTS[3..]
                        .iter()
                        .map(|stint| view! { <WorkRow stint=stint folded=true /> })
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
        <section class="ghost-section ghost-section-outer home-index-section">
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
                            <span class="honest-edit" tabindex="0">
                                <span class="honest-original">"figuring things out"</span>
                                <span class="honest-insertion" aria-hidden="true">
                                    "getting things wrong in public"
                                </span>
                            </span> ", in english e às vezes em português."
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
                <p class="mt-8 text-sm text-ink-3">
                    "hidden text borrows from " <a class="plink" href="https://igorbedesqui.com/">
                        "igorbedesqui.com"
                    </a> ", who credits " <a class="plink" href="https://ped.ro/">
                        "ped.ro"
                    </a> " and " <a class="plink" href="https://lfe.org/">
                        "lfe.org"
                    </a> "."
                </p>
                <script>{HOME_SCRIPT}</script>
            </div>
        </div>
    }
    .into_any()
}
