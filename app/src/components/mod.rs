//! Site-wide UI components. Post-embeddable components (the registry
//! vocabulary) live under [`blog`]; everything else here is app chrome.

use leptos::prelude::*;
use leptos_meta::Title;

pub mod blog;
pub mod header;
pub mod not_found;

pub use header::Header;
pub use not_found::NotFound;

/// The one page scaffold — shared container and heading, with an optional
/// `<Title>`. Every page except the post article renders through this.
pub(crate) fn page(
    title: Option<String>,
    heading: impl IntoView,
    body: impl IntoView,
) -> impl IntoView {
    view! {
        {title.map(|text| view! { <Title text=text /> })}
        <section class="mx-auto max-w-2xl px-6 py-16">
            <h1 class="font-heading text-3xl font-bold">{heading}</h1>
            {body}
        </section>
    }
}
