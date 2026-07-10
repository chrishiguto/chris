//! The `/about` static page (PRD user story 19): prompt motif, prose,
//! currently list, contact block. Copy comes from the design mock; the
//! contact hrefs are well-formed mocks until real handles exist.

use leptos::prelude::*;
use leptos_meta::Title;

use crate::components::{section_label, PAGE_COLUMN};

fn contact_link(href: &'static str, label: &'static str) -> impl IntoView {
    view! {
        <a href=href class="contact-link">
            {label}
            <span class="link-arrow" aria-hidden="true">
                "↗"
            </span>
        </a>
    }
}

#[component]
pub fn AboutPage() -> impl IntoView {
    view! {
        <Title text="about — chris" />
        <section class=PAGE_COLUMN>
            <p class="flex items-baseline gap-2 font-mono text-sm">
                <span class="text-ink-3">"~/chris"</span>
                <span class="text-accent">"$"</span>
                <span>"cat about.md"</span>
            </p>
            <h1 class="mt-5 text-2xl font-semibold tracking-tight">"about"</h1>
            <div class="mt-6 max-w-[65ch] space-y-5 text-[1.0625rem] leading-relaxed">
                <p>
                    "i'm christiano higuto — chris. software engineer from brazil, curious by default. i've been paid to write code for a while now, and i still think the best part is the moment something finally clicks."
                </p>
                <p>
                    "this site is my personal space on the internet: a giant notebook where i write about code, systems, and the non-code parts of an engineering life. some posts are in english, some em português, all lowercase."
                </p>
                <p>
                    "away from the keyboard: coffee, books, long walks pretending to think about architecture."
                </p>
            </div>
            <div class="mt-9">
                {section_label("currently")}
                <ul class="mt-4 flex flex-col gap-2 font-mono text-sm text-ink-2">
                    <li>"reading · designing data-intensive applications (again)"</li>
                    <li>"learning · rust, slowly and stubbornly"</li>
                    <li>"listening · lo-fi and compiler talks"</li>
                </ul>
            </div>
            <div class="mt-12">
                {section_label("contact")}
                <p class="mt-4 max-w-[48ch] text-ink-2">
                    "say hi, ask anything, or tell me my code is wrong (politely). i read everything."
                </p> <p class="mt-4">
                    <a href="mailto:hi@chris.dev" class="font-mono text-sm">
                        "hi@chris.dev"
                    </a>
                </p>
                <div class="mt-3 flex gap-6">
                    {contact_link("https://github.com/chris", "github")}
                    {contact_link("https://www.linkedin.com/in/chris", "linkedin")}
                </div>
            </div>
        </section>
    }
}
