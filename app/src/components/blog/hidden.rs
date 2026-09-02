use leptos::prelude::*;
use registry::post_component;

/// Progressive disclosure for post prose. The server ships the complete
/// child tree; the tiny enhancement only changes its visual presentation.
#[post_component]
#[component]
pub fn Hidden(children: Children) -> impl IntoView {
    view! {
        <div class="fold">
            <button
                type="button"
                class="fold-trigger"
                aria-expanded="false"
                hidden
                onclick="this.setAttribute('aria-expanded','true');this.parentElement.classList.add('fold-open')"
            >
                <span aria-hidden="true">"(…)"</span>
                <span class="sr-only">"reveal hidden text"</span>
            </button>
            <div class="fold-content">{children()}</div>
            <script>
                "const fold=document.currentScript.parentElement;fold.classList.add('fold-ready');fold.querySelector('.fold-trigger').hidden=false;"
            </script>
        </div>
    }
}
