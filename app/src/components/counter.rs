use leptos::prelude::*;
use registry::post_component;

/// The v1 interactive demo island: server-rendered, hydrates client-side.
#[post_component]
#[island]
pub fn Counter(initial: i64) -> impl IntoView {
    let count = RwSignal::new(initial);

    view! {
        <div class="flex items-center gap-4 rounded-lg border border-neutral-300 p-4">
            <button
                class="h-10 w-10 rounded bg-neutral-900 text-lg text-white"
                on:click=move |_| count.update(|n| *n -= 1)
            >
                "−"
            </button>
            <span class="min-w-12 text-center text-2xl tabular-nums">{move || count.get()}</span>
            <button
                class="h-10 w-10 rounded bg-neutral-900 text-lg text-white"
                on:click=move |_| count.update(|n| *n += 1)
            >
                "+"
            </button>
        </div>
    }
}
