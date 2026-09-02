use leptos::prelude::*;

/// Decorative marginal section name paired with a real, visually hidden
/// heading. `outer` mirrors the mark into a spread's right gutter.
#[component]
pub fn GhostWord(label: &'static str, #[prop(default = false)] outer: bool) -> impl IntoView {
    let class = if outer {
        "ghost-word ghost-word-outer"
    } else {
        "ghost-word"
    };
    view! {
        <h2 class="sr-only">{label}</h2>
        <span class=class aria-hidden="true">
            {label}
        </span>
    }
}
