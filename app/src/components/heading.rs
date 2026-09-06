use leptos::prelude::*;

/// One `<h1>` shared by [`Page`](super::Page) and the home masthead so the two
/// front-page headings can't drift in size or tracking.
#[component]
pub(crate) fn Heading(children: Children) -> impl IntoView {
    view! { <h1 class="text-3xl font-semibold tracking-[-0.01em]">{children()}</h1> }
}
