use std::time::Duration;

use leptos::prelude::*;

/// Copy-to-clipboard for code-block chrome. Zero props by design: the button
/// reads the adjacent `<pre><code>` text out of the DOM at click time, so the
/// source ships in the page exactly once — never serialized again as island
/// props. Feedback flips to "copied ✓" and reverts after 1.4s; without JS
/// the SSR'd button is simply inert.
#[island]
pub fn CopyButton() -> impl IntoView {
    let (copied, set_copied) = signal(false);
    let button_ref = NodeRef::<leptos::html::Button>::new();
    let copy = move |_| {
        let Some(code) = button_ref
            .get_untracked()
            .and_then(|button| button.closest(".code-block").ok().flatten())
            .and_then(|block| block.query_selector("pre code").ok().flatten())
        else {
            return;
        };
        let text = code.text_content().unwrap_or_default();
        // Fire-and-forget: the promise resolves to nothing actionable, and a
        // denied clipboard just leaves the feedback as a harmless white lie.
        let _ = window().navigator().clipboard().write_text(&text);
        set_copied.set(true);
        set_timeout(move || set_copied.set(false), Duration::from_millis(1400));
    };

    view! {
        <button type="button" class="code-copy" node_ref=button_ref on:click=copy>
            {move || if copied.get() { "copied ✓" } else { "copy" }}
        </button>
    }
}
