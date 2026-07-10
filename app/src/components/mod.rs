//! Site-wide UI components; post-embeddable ones live under [`blog`].

use leptos::prelude::*;
use leptos_meta::Title;

pub mod blog;
pub mod copy_button;
pub mod header;
pub mod not_found;
pub mod theme_toggle;

pub use copy_button::CopyButton;
pub use header::Header;
pub use not_found::NotFound;
pub use theme_toggle::ThemeToggle;

/// Mono section label (design SectionLabel); shared by the home and about pages.
pub(crate) fn section_label(text: &'static str) -> impl IntoView {
    view! { <p class="font-mono text-xs tracking-wide text-ink-3">{text}</p> }
}

/// Shared page scaffold; every page except the post article renders through it.
pub(crate) fn page(
    title: Option<String>,
    heading: impl IntoView,
    body: impl IntoView,
) -> impl IntoView {
    view! {
        {title.map(|text| view! { <Title text=text /> })}
        <section class="mx-auto max-w-2xl px-6 py-16">
            <h1 class="text-3xl font-semibold tracking-tight">{heading}</h1>
            {body}
        </section>
    }
}
