use leptos::prelude::*;
use registry::post_component;

/// The v1 interactive demo island: server-rendered, hydrates client-side.
#[post_component]
#[island]
pub fn Counter(initial: i64) -> impl IntoView {
    let count = RwSignal::new(initial);

    view! {
        <div class="my-6 inline-flex items-center gap-4 rounded-lg border border-line bg-surface-raised p-4">
            <button
                class="h-10 w-10 rounded bg-ink font-mono text-lg text-surface hover:bg-accent"
                on:click=move |_| count.update(|n| *n -= 1)
            >
                "−"
            </button>
            <span class="min-w-12 text-center font-mono text-2xl tabular-nums">
                {move || count.get()}
            </span>
            <button
                class="h-10 w-10 rounded bg-ink font-mono text-lg text-surface hover:bg-accent"
                on:click=move |_| count.update(|n| *n += 1)
            >
                "+"
            </button>
        </div>
    }
}
