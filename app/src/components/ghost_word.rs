use leptos::prelude::*;

/// Decorative marginal section name paired with a real, visually hidden
/// heading. `outer` mirrors the mark into a spread's right gutter.
#[component]
pub fn GhostWord(label: &'static str, #[prop(default = false)] outer: bool) -> impl IntoView {
    view! {
        <h2 class="sr-only">{label}</h2>
        <span class="ghost-word" class:ghost-word-outer=outer aria-hidden="true">
            {label}
        </span>
    }
}
