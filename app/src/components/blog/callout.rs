use leptos::prelude::*;
use registry::post_component;

/// Highlighted aside wrapping markdown children; `kind` picks the visual
/// treatment.
#[post_component]
#[component]
pub fn Callout(kind: String, title: Option<String>, children: Children) -> impl IntoView {
    view! {
        <aside class=format!(
            "callout callout-{kind}",
        )>
            {title.map(|title| view! { <p class="callout-title">{title}</p> })}
            <div class="callout-body">{children()}</div>
        </aside>
    }
}
