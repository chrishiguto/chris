use leptos::prelude::*;

/// The site footer: one full-width paper edge. It sits outside `Routes` in
/// the app shell, so every page — the 404 fallback included — renders it.
/// Its copy occupies the same middle column as the page while the hairline
/// reaches edge to edge.
#[component]
pub fn Footer() -> impl IntoView {
    view! {
        <footer class="border-t border-line text-xs text-ink-3">
            <div class="page-grid">
                <div class="col-start-2 flex flex-wrap items-baseline justify-between gap-6 py-5">
                    <span>"christiano higuto · são paulo"</span>
                    <span>
                        <a href=content::RSS_PATH>"rss"</a>
                        <span aria-hidden="true">" · "</span>
                        <a href="https://github.com/chrishiguto/chris">"source"</a>
                    </span>
                </div>
            </div>
        </footer>
    }
}
