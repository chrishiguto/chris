use leptos::prelude::*;

#[component]
pub fn Header() -> impl IntoView {
    view! {
        <header class="border-b border-line">
            <div class="mx-auto flex max-w-2xl items-baseline justify-between px-6 py-4">
                <a href="/" class="text-lg font-semibold tracking-tight">
                    "chris"
                </a>
                <nav class="flex gap-4">
                    <a href="/posts" class="font-mono text-sm text-ink-2 hover:text-accent">
                        "posts"
                    </a>
                    <a href="/tags" class="font-mono text-sm text-ink-2 hover:text-accent">
                        "tags"
                    </a>
                </nav>
            </div>
        </header>
    }
}
