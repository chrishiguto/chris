//! PROTOTYPE — career timeline in place of the home writing panel.
//!
//! Round 3: "big years" won round 2 but the numeral was too loud, so every
//! variant is now that layout with the year stepped down to a middle size
//! (`text-2xl`); the variants differ only in the `@ company` color — accent,
//! accent-2, or quiet ink-2. Switchable via `?variant=a|b|c` and the floating
//! bottom bar (arrow keys cycle too). Dev builds only — `listing.rs` gates
//! the mount on `debug_assertions`. Throwaway with
//! `style/timeline-prototype.css`; delete both together.

use leptos::prelude::*;

use crate::components::SectionLabel;

/// One career stint, mocked.
struct Stint {
    years: &'static str,
    year: &'static str,
    role: &'static str,
    org: &'static str,
    summary: &'static str,
    tags: &'static [&'static str],
    current: bool,
}

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

/// The one axis left this round: the company name's color class.
const VARIANTS: [(&str, &str, &str); 3] = [
    ("a", "org in accent", "text-accent"),
    ("b", "org in accent-2", "text-accent-2"),
    ("c", "org in ink-2", "text-ink-2"),
];

/// Arrow keys cycle variants; inert while a field or editable node is focused.
const SWITCHER_KEYS: &str = r#"document.addEventListener("keydown",function(e){if(e.defaultPrevented)return;var t=e.target;if(t&&(/^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName)||t.isContentEditable))return;if(e.key!=="ArrowLeft"&&e.key!=="ArrowRight")return;var v=["a","b","c"];var p=new URLSearchParams(location.search);var i=v.indexOf(p.get("variant"));if(i<0)i=0;p.set("variant",v[(i+(e.key==="ArrowRight"?1:v.length-1))%v.length]);location.search=p.toString();});"#;

/// The prototype mount: section header, the picked variant, the switcher.
#[component]
pub(crate) fn TimelinePrototype(variant: String) -> impl IntoView {
    let idx = VARIANTS
        .iter()
        .position(|(k, _, _)| *k == variant.as_str())
        .unwrap_or(0);
    view! {
        <section class="mt-10">
            <div class="flex items-baseline gap-2 text-sm">
                <SectionLabel>"career"</SectionLabel>
                <span class="text-ink-3" aria-hidden="true">
                    "·"
                </span>
                <span class="text-xs text-ink-3">"prototype"</span>
            </div>
            <BigYears org_class=VARIANTS[idx].2 />
        </section>
        <PrototypeSwitcher idx=idx />
    }
}

/// The alternating side class for entry `i`; side-a carries content on the
/// left of the spine (md+), side-b on the right.
fn side(i: usize) -> &'static str {
    if i.is_multiple_of(2) {
        "tlp-side-a"
    } else {
        "tlp-side-b"
    }
}

/// The card interior: "role @ org" (the `@` stays muted, the company takes
/// this round's color under test), the summary, and the tag chips.
fn card_body(s: &'static Stint, org_class: &'static str) -> impl IntoView {
    let chips = (!s.tags.is_empty()).then(|| {
        let chips = s
            .tags
            .iter()
            .map(|t| {
                view! {
                    <span class="tlp-chip rounded-full border border-line px-2 py-0.5 text-xs text-ink-2">
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
            <span class=format!("font-medium {org_class}")>{s.org}</span>
        </h3>
        <p class="mt-1.5 text-sm leading-normal text-ink-2">{s.summary}</p>
        {chips}
    }
}

/// The winning layout: dated cards on alternating sides, the year as a
/// middle-sized display numeral opposite the card (warming to accent when
/// its card is hovered), range in small type beneath it.
#[component]
fn BigYears(org_class: &'static str) -> impl IntoView {
    let items = STINTS
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let dot = if s.current {
                "tlp-dot tlp-now"
            } else {
                "tlp-dot"
            };
            view! {
                <li class=format!("tlp-item {}", side(i))>
                    <span class="tlp-node">
                        <span class=dot></span>
                    </span>
                    <div class="tlp-aside">
                        <p class="tlp-yearnum font-display text-2xl font-semibold tracking-tight">
                            {s.year}
                        </p>
                        <p class="mt-0.5 text-xs text-ink-3">{s.years}</p>
                    </div>
                    <div class="tlp-body">
                        <article class="tlp-card">{card_body(s, org_class)}</article>
                    </div>
                </li>
            }
        })
        .collect_view();
    view! {
        <div class="tlp tlp-a mt-8">
            <div class="tlp-spine" aria-hidden="true"></div>
            <ol class="tlp-list">{items}</ol>
        </div>
    }
}

/// The floating switcher: an inverted pill pinned bottom-center, obviously
/// not part of the design under review. Arrows navigate (full reload — the
/// page is server-rendered); `←`/`→` keys do the same via [`SWITCHER_KEYS`].
#[component]
fn PrototypeSwitcher(idx: usize) -> impl IntoView {
    let (current, name, _) = VARIANTS[idx];
    let prev = VARIANTS[(idx + VARIANTS.len() - 1) % VARIANTS.len()].0;
    let next = VARIANTS[(idx + 1) % VARIANTS.len()].0;
    let arrow = "bg-none px-1.5 py-0.5 text-surface hover:text-accent-2";
    view! {
        <div class="fixed bottom-6 left-1/2 z-50 flex -translate-x-1/2 items-center gap-2 rounded-full bg-ink px-4 py-2 shadow-md">
            <a href=format!("/?variant={prev}") class=arrow aria-label="previous variant">
                "←"
            </a>
            <span class="min-w-[9rem] text-center text-xs font-semibold tracking-wide text-surface">
                {format!("{current} — {name}")}
            </span>
            <a href=format!("/?variant={next}") class=arrow aria-label="next variant">
                "→"
            </a>
        </div>
        <script inner_html=SWITCHER_KEYS></script>
    }
}
