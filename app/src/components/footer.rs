use leptos::prelude::*;

/// The site footer: a single signature line. It sits outside `Routes` in
/// the app shell, so every page — the 404 fallback included — renders it.
#[component]
pub fn Footer() -> impl IntoView {
    view! {
        <footer class="site-footer">
            <span>"© 2026 christiano higuto — built slowly, on purpose"</span>
        </footer>
    }
}
