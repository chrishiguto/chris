use leptos::prelude::*;

/// Small tracked section label: semibold, tracked, `ink-2` — a lighter ink
/// fails WCAG AA at label sizes. One component shared by the home rail, the
/// writing header, and the about page so the AA-critical color can't drift.
/// Marker-free by design.
#[component]
pub(crate) fn SectionLabel(children: Children) -> impl IntoView {
    view! { <p class="text-sm font-semibold tracking-wide text-ink-2">{children()}</p> }
}
