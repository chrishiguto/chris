use leptos::prelude::*;

/// The page `<h1>` at the 30px title register, for pages that render
/// through [`Page`](super::Page) (today the 404).
#[component]
pub(crate) fn Heading(children: Children) -> impl IntoView {
    view! { <h1 class="text-3xl font-semibold tracking-[-0.01em]">{children()}</h1> }
}
