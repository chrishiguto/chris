use leptos::prelude::*;
use registry::post_component;

// The wrong order: leptos expands first, so #[post_component] sees the
// generated Props signature instead of the original one.
#[component]
#[post_component]
pub fn Bad(msg: String) -> impl IntoView {
    view! { <span>{msg}</span> }
}

fn main() {}
