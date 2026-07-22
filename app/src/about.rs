//! The `/about` static page: prose, currently list, contact block. The
//! contact hrefs are well-formed mocks until real handles exist.

use leptos::prelude::*;

use crate::components::{page_title, Contacts, Page, SectionLabel};

#[component]
pub fn AboutPage() -> impl IntoView {
    view! {
        <Page title=page_title("about") heading="about">
            <div class="mt-6 max-w-[65ch] space-y-5 text-[1.0625rem] leading-relaxed">
                <p>
                    "i’m christiano higuto — chris. software engineer from brazil, curious by default. i’ve been paid to write code for a while now, and i still think the best part is the moment something finally clicks."
                </p>
                <p>
                    "this site is my personal space on the internet: a giant notebook where i write about code, systems, and the non-code parts of an engineering life. some posts are in english, some em português, all lowercase."
                </p>
                <p>
                    "away from the keyboard: coffee, books, long walks pretending to think about architecture."
                </p>
            </div>
            <div class="mt-9">
                <SectionLabel>"currently"</SectionLabel>
                <ul class="mt-4 flex flex-col gap-2 text-sm text-ink-2">
                    <li>"reading · designing data-intensive applications (again)"</li>
                    <li>"learning · rust, slowly and stubbornly"</li>
                    <li>"listening · lo-fi and compiler talks"</li>
                </ul>
            </div>
            <div class="mt-12">
                <SectionLabel>"contact"</SectionLabel>
                <p class="mt-4 max-w-[48ch] text-ink-2">
                    "say hi, ask anything, or tell me my code is wrong (politely). i read everything."
                </p>
                <Contacts lead="mt-4" />
            </div>
        </Page>
    }
}
