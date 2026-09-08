use leptos::html::Div;
use leptos::prelude::*;

/// One-way progressive disclosure over server-rendered children.
///
/// `children` stay a server `#[component]` blob — Leptos projects them into
/// the island as a single opaque `<leptos-children>`, so the rows never reach
/// wasm and their data is never serialized. That opacity is also why the open
/// state is expressed as CSS on the wrapper rather than control flow: the
/// island cannot reach inside the blob, and children absent from the SSR'd
/// HTML would never be sent at all.
///
/// `ready` is the progressive-enhancement gate. Effects never run during SSR,
/// so the server ships the content visible and the button `hidden`; only once
/// the island hydrates does CSS clip the content like `sr-only` — still in the
/// accessibility tree, out of sight — and reveal the button. No JavaScript, no
/// fold: the reader simply sees everything.
#[island]
pub fn Fold(label: String, children: Children) -> impl IntoView {
    let (open, set_open) = signal(false);
    let (ready, set_ready) = signal(false);
    let content: NodeRef<Div> = NodeRef::new();

    Effect::new(move |_| set_ready.set(true));
    // Opening hides the button that was focused, so focus moves to the revealed
    // content. Watching `open` instead of focusing inside the handler lets the
    // class land before the element is scrolled into view.
    Effect::new(move |_| {
        if open.get() {
            if let Some(el) = content.get() {
                let _ = el.focus();
            }
        }
    });

    view! {
        <div class="fold" class:is-ready=move || ready.get() class:is-open=move || open.get()>
            <button
                class="fold-button"
                type="button"
                aria-expanded=move || open.get().to_string()
                hidden=move || !ready.get()
                on:click=move |_| set_open.set(true)
            >
                <span aria-hidden="true">"(…)"</span>
                <span class="sr-only">{label}</span>
            </button>
            <div class="fold-content" node_ref=content tabindex="-1">
                {children()}
            </div>
        </div>
    }
}
