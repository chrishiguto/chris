use leptos::prelude::*;

/// The page `<h1>` at the 30px title register.
/// One component shared by [`Page`](super::Page) and the home masthead so the
/// two front-page headings can't drift in face, size, or tracking.
#[component]
pub(crate) fn Heading(children: Children) -> impl IntoView {
    view! { <h1 class="text-3xl font-semibold tracking-[-0.01em]">{children()}</h1> }
}
