use leptos::prelude::*;
use registry::post_component;

/// Co-located with its post: this island ships in the same push
/// as the prose referencing it, and CI deploys it before the post publishes.
#[post_component]
#[island]
pub fn DeployStages(total: i64) -> impl IntoView {
    let done = RwSignal::new(0i64);

    view! {
        <div class="my-6 inline-flex items-center gap-4 rounded-lg border border-line bg-surface-raised p-4">
            <button
                class="rounded bg-ink px-3 py-1 font-mono text-surface hover:bg-accent"
                on:click=move |_| done.update(|n| *n = (*n + 1).min(total))
            >
                "advance"
            </button>
            <span class="font-mono tabular-nums">
                {move || format!("{}/{total} stages done", done.get())}
            </span>
        </div>
    }
}
