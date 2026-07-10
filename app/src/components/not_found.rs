use leptos::prelude::*;

use crate::components::page;

#[component]
pub fn NotFound(
    #[prop(default = "This page does not exist.")] message: &'static str,
) -> impl IntoView {
    page(
        None,
        "404",
        view! {
            <p class="mt-6 text-ink-2">{message}</p>
            // No `underline` utility: the base link rule already grows one,
            // and stacking both double-underlined this link (slice-1 debt).
            <a href="/" class="mt-4 inline-block text-accent">
                "back home"
            </a>
        },
    )
}
