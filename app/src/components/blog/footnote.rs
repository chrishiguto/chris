use leptos::prelude::*;
use registry::post_component;

/// A margin note. The children are the phrase the note belongs to; `note` is
/// the note itself. Both ship in the SSR HTML, so the note is always readable
/// and always in the accessibility tree — the prose sheet only decides where
/// it sits: hanging in the right gutter when the viewport can hold one,
/// inline in italic below that. The dagger and its label live here so
/// authors never hand-type presentation.
#[post_component]
#[component]
pub fn Footnote(note: String, children: Children) -> impl IntoView {
    view! {
        {children()}
        <sup class="footnote-ref" aria-label="footnote">
            "†"
        </sup>
        <span class="footnote-note">{note}</span>
    }
}
