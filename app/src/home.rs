//! The home page `/`: the masthead identity band over the career timeline
//! ([`CareerTimeline`]). The writing surface lives on its own page,
//! [`crate::writing`].

use leptos::prelude::*;

use crate::components::{CareerTimeline, Contacts, Heading, PageShell};

/// The front-door band: greeting, one voice line, external-only contacts. Nav
/// owns "writing" and "about", so the masthead carries no in-app links.
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
