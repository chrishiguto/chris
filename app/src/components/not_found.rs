use leptos::prelude::*;

/// The router fallback for URLs no route matches; other 404 surfaces
/// (like an unknown post slug) reuse it with their own message.
#[component]
pub fn NotFound(
    #[prop(default = "This page does not exist.")] message: &'static str,
) -> impl IntoView {
    view! {
        <section class="mx-auto max-w-2xl px-6 py-16">
            <h1 class="font-heading text-3xl font-bold">"404"</h1>
            <p class="mt-6 text-ink-muted">{message}</p>
            <a href="/" class="mt-4 inline-block text-accent underline">
                "back home"
            </a>
        </section>
    }
}
