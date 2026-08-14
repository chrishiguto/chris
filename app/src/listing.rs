//! The home page `/`: the masthead identity band over the career timeline
//! ([`CareerTimeline`]). The writing index moved off the home; its island
//! ([`crate::components::WritingIndex`]) stays in the tree for the writing
//! surface's next address.

use content::IndexEntry;
use leptos::prelude::*;

use crate::components::{CareerTimeline, Contacts, Heading, PageShell};

/// Per-request index from the site worker, newest-first. The home no longer
/// reads it, but the worker still provides it and the feeds and sitemap ride
/// the same index.
#[derive(Clone)]
pub struct IndexData(pub Vec<IndexEntry>);

/// The front-door band: greeting, one voice line, external-only contacts. Nav
/// owns "about", so the masthead carries no in-app links.
#[component]
fn MastheadBand() -> impl IntoView {
    view! {
        <header class="border-b border-line pb-10">
            <Heading>"hey, i’m chris"</Heading>
            <p class="mt-5 max-w-[58ch] text-lg leading-relaxed text-ink-2">
                "software engineer. this is everything i’m writing — code, systems, and figuring things out, in english e às vezes em português."
            </p>
            <Contacts lead="mt-6" />
        </header>
    }
}

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <PageShell>
            <MastheadBand />
            <CareerTimeline />
        </PageShell>
    }
}
