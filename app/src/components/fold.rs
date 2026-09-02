use leptos::prelude::*;

/// One-way progressive disclosure. The folded content remains in the HTML
/// and accessibility tree; CSS clips it visually until the tiny script adds
/// the open state. This stays server markup, not a hydration island.
#[component]
pub fn Fold(children: Children) -> impl IntoView {
    view! {
        <div class="home-fold">
            <button class="home-fold-button" type="button" aria-expanded="false">
                <span aria-hidden="true">"(…)"</span>
                <span class="sr-only">"show earlier work"</span>
            </button>
            <div class="home-fold-content">{children()}</div>
        </div>
    }
}
