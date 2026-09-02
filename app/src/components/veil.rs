use leptos::prelude::*;

/// The empty top edge is intentionally inert: it dissolves scrolling text
/// without becoming another navigation surface.
#[component]
pub fn Veil() -> impl IntoView {
    view! { <div class="site-veil" aria-hidden="true"></div> }
}
