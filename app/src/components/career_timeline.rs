//! The home page's career timeline: alternating ledger cards on a center
//! spine (left rail on phones). Markup shape is `div.tl > div.tl-spine +
//! ol.tl-list > li.tl-item`, each item grid-placed by `tl-side-a`/`tl-side-b`;
//! the look and motion live in `style/timeline.css`. Design settled by the
//! career-timeline prototype (branch `prototype/career-timeline`).

use leptos::prelude::*;

use crate::components::SectionLabel;

/// One career stint.
struct Stint {
    /// The full range, shown small under the year numeral.
    years: &'static str,
    /// The start year, shown as the display-face numeral opposite the card.
    year: &'static str,
    role: &'static str,
    org: &'static str,
    summary: &'static str,
    tags: &'static [&'static str],
    /// Marks the ongoing stint; its spine dot breathes.
    current: bool,
}

/// Mocked career data carried over from the prototype — swap for the real
/// history before this ships anywhere public.
const STINTS: &[Stint] = &[
    Stint {
        years: "2025 — now",
        year: "2025",
        role: "staff engineer",
        org: "edgeline",
        summary: "rust on the edge: moved the platform's rendering into wasm \
                  workers. these days i mostly delete code and write adrs.",
        tags: &["rust", "wasm", "edge"],
        current: true,
    },
    Stint {
        years: "2022 — 2025",
        year: "2022",
        role: "senior engineer",
        org: "nimbus labs",
        summary: "platform team of four. took the deploy pipeline from forty \
                  minutes to four, then made it boring enough to forget.",
        tags: &["platform", "ci", "kubernetes"],
        current: false,
    },
    Stint {
        years: "2020 — 2022",
        year: "2020",
        role: "engineer",
        org: "parallel.fm",
        summary: "audio streaming at scale — learned that the hard part is \
                  never the audio, it's the retries.",
        tags: &["go", "streaming"],
        current: false,
    },
    Stint {
        years: "2018 — 2020",
        year: "2018",
        role: "engineer",
        org: "bancoteca",
        summary: "fintech em são paulo: ledgers, pix, and a healthy fear of \
                  off-by-one cents.",
        tags: &["kotlin", "fintech"],
        current: false,
    },
    Stint {
        years: "2016 — 2018",
        year: "2016",
        role: "junior dev",
        org: "agência pixel",
        summary: "first job: wordpress themes by day, everything else by \
                  night.",
        tags: &["php", "js"],
        current: false,
    },
    Stint {
        years: "2012 — 2016",
        year: "2012",
        role: "b.sc. computer science",
        org: "usp",
        summary: "where it started — graph theory and a lot of coffee.",
        tags: &[],
        current: false,
    },
];

/// The whole career section: the clean label over the timeline.
#[component]
pub(crate) fn CareerTimeline() -> impl IntoView {
    let items = STINTS.iter().enumerate().map(entry).collect_view();
    view! {
        <section class="mt-10">
            <SectionLabel>"career"</SectionLabel>
            <div class="tl mt-8">
                <div class="tl-spine" aria-hidden="true"></div>
                <ol class="tl-list">{items}</ol>
            </div>
        </section>
    }
}

/// One timeline entry: the spine dot, the year numeral over the range in the
/// aside, and the card. Sides alternate; on phones both columns sit right of
/// the rail, aside above card.
fn entry((i, s): (usize, &'static Stint)) -> impl IntoView {
    let side = if i.is_multiple_of(2) {
        "tl-side-a"
    } else {
        "tl-side-b"
    };
    let dot = if s.current { "tl-dot tl-now" } else { "tl-dot" };
    view! {
        <li class=format!("tl-item {side}")>
            <span class="tl-node">
                <span class=dot></span>
            </span>
            <div class="tl-aside">
                <p class="tl-yearnum font-display text-2xl font-semibold tracking-tight">
                    {s.year}
                </p>
                <p class="mt-0.5 text-xs text-ink-3">{s.years}</p>
            </div>
            <div class="tl-body">
                <article class="tl-card">{card_body(s)}</article>
            </div>
        </li>
    }
}

/// The card interior: "role @ org" — the `@` muted, the org a quiet step up
/// in medium weight — over the summary and the tag chips.
fn card_body(s: &'static Stint) -> impl IntoView {
    let chips = (!s.tags.is_empty()).then(|| {
        let chips = s
            .tags
            .iter()
            .map(|t| {
                view! {
                    <span class="tl-chip rounded-full border border-line px-2 py-0.5 text-xs text-ink-2">
                        {*t}
                    </span>
                }
            })
            .collect_view();
        view! { <div class="mt-3 flex flex-wrap gap-1.5">{chips}</div> }
    });
    view! {
        <h3 class="text-base font-semibold tracking-tight">
            {s.role} <span class="font-normal text-ink-3">" @ "</span>
            <span class="font-medium text-ink-2">{s.org}</span>
        </h3>
        <p class="mt-1.5 text-sm leading-normal text-ink-2">{s.summary}</p>
        {chips}
    }
}
