use leptos::prelude::*;

use crate::components::page;

/// The router fallback for URLs no route matches; other 404 surfaces
/// (like an unknown post slug) reuse it with their own message.
#[component]
pub fn NotFound(
    #[prop(default = "This page does not exist.")] message: &'static str,
) -> impl IntoView {
    page(
        None,
        "404",
        view! {
            <p class="mt-6 text-ink-muted">{message}</p>
            <a href="/" class="mt-4 inline-block text-accent underline">
                "back home"
            </a>
        },
    )
}
