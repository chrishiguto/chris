use leptos::prelude::*;

/// The site footer: a single signature line. It sits outside `Routes` in
/// the app shell, so every page — the 404 fallback included — renders it.
/// The border runs full-bleed; the inner wrapper caps the line to the page
/// column so it lines up with the header and body.
#[component]
pub fn Footer() -> impl IntoView {
    view! {
        <footer class="border-t border-line text-xs text-ink-3">
            <div class="mx-auto max-w-2xl px-6 py-5">
                <span>"© 2026 christiano higuto — built slowly, on purpose"</span>
            </div>
        </footer>
    }
}
