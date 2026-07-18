use leptos::prelude::*;

/// The section-label type: semibold, tracked, `ink-2` — a lighter ink fails
/// WCAG AA at label sizes. [`SectionLabel`] prepends `text-sm`;
/// inline labels that set their own size (the writing header) reuse this
/// alone, so the AA-critical color can't drift between them.
pub(crate) const SECTION_LABEL_CLASS: &str = "font-semibold tracking-wide text-ink-2";

/// Small tracked section label; shared by the home rail and the about page.
/// Marker-free by design.
#[component]
pub(crate) fn SectionLabel(text: &'static str) -> impl IntoView {
    view! { <p class=format!("text-sm {SECTION_LABEL_CLASS}")>{text}</p> }
}
