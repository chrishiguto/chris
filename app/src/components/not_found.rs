use leptos::prelude::*;

use crate::components::{page, page_title};

#[component]
pub fn NotFound(
    #[prop(default = "this page does not exist.")] message: &'static str,
) -> impl IntoView {
    page(
        Some(page_title("404")),
        "404",
        view! {
            <p class="mt-5 font-mono text-sm text-ink-3">"no such file or directory"</p>
            <p class="mt-4 text-ink-2">{message}</p>
            // No `underline` utility: the base link rule already grows one,
            // and stacking both double-underlined this link (slice-1 debt).
            <a href="/" class="mt-6 inline-block text-accent">
                "back home"
            </a>
        },
    )
}
