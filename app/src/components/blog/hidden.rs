use leptos::prelude::*;
use registry::post_component;

/// Progressive disclosure for post prose. The server ships the complete
/// child tree; the tiny enhancement only changes its visual presentation.
/// Opening hides the trigger, so focus moves onto the revealed prose rather
/// than falling to the document.
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
                onclick="this.setAttribute('aria-expanded','true');this.parentElement.classList.add('fold-open');var c=this.parentElement.querySelector('.fold-content');c.tabIndex=-1;c.focus()"
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
