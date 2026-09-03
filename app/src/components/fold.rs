use leptos::prelude::*;

/// One-way progressive disclosure. The folded content ships in full and is
/// visible until a script marks the fold ready (`is-ready`): only then does
/// CSS clip it like `sr-only` — still in the accessibility tree, out of
/// sight — and unhide the button that opens it (`is-open`). No JavaScript,
/// no fold: the reader simply sees everything. `label` is the button's
/// spoken name.
#[component]
pub fn Fold(label: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="fold">
            <button class="fold-button" type="button" aria-expanded="false" hidden>
                <span aria-hidden="true">"(…)"</span>
                <span class="sr-only">{label}</span>
            </button>
            <div class="fold-content">{children()}</div>
        </div>
    }
}
