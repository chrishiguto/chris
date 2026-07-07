use leptos::prelude::*;

/// The site-wide header: branding plus top-level navigation.
#[component]
pub fn Header() -> impl IntoView {
    view! {
        <header class="border-b border-line">
            <div class="mx-auto flex max-w-2xl items-baseline justify-between px-6 py-4">
                <a href="/" class="font-heading text-lg font-bold">
                    "chris"
                </a>
                <nav class="flex gap-4">
                    <a href="/" class="font-mono text-sm text-ink-muted hover:text-accent">
                        "posts"
                    </a>
                    <a href="/tags" class="font-mono text-sm text-ink-muted hover:text-accent">
                        "tags"
                    </a>
                </nav>
            </div>
        </header>
    }
}
