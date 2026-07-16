use std::time::Duration;

use leptos::prelude::*;

/// Chromed code panel: a bar naming the fence language (or `code`) with the
/// copy button, over the source in a `<pre>`.
#[component]
pub fn CodeBlock(lang: Option<String>, text: String) -> impl IntoView {
    let label = lang.clone().unwrap_or_else(|| "code".into());
    // `class=Option::None` still emits `class=""`, so branch instead.
    let code = match lang {
        Some(lang) => {
            view! { <code class=format!("language-{lang}")>{text.clone()}</code> }.into_any()
        }
        None => view! { <code>{text.clone()}</code> }.into_any(),
    };
    view! {
        <div class="code-block">
            <div class="code-bar">
                <span class="code-lang">{label}</span>
                <CopyButton text=text />
            </div>
            <pre>{code}</pre>
        </div>
    }
}

/// Copies the block's source, carried as a prop so the button never reads
/// the DOM. Feedback flips to "copied ✓" and reverts after 1.4s; without JS
/// the SSR'd button is simply inert.
#[island]
pub fn CopyButton(text: String) -> impl IntoView {
    let (copied, set_copied) = signal(false);
    let copy = move |_| {
        // Fire-and-forget: the promise resolves to nothing actionable, and a
        // denied clipboard just leaves the feedback as a harmless white lie.
        let _ = window().navigator().clipboard().write_text(&text);
        set_copied.set(true);
        set_timeout(move || set_copied.set(false), Duration::from_millis(1400));
    };

    view! {
        <button type="button" class="code-copy" on:click=copy>
            {move || if copied.get() { "copied ✓" } else { "copy" }}
        </button>
    }
}
