use leptos::prelude::*;

use super::Konami;

/// The site footer: copyright left, konami hint right. It sits
/// outside `Routes` in the app shell, so every page — the 404 fallback
/// included — renders it.
#[component]
pub fn Footer() -> impl IntoView {
    view! {
        <footer class="site-footer">
            <span>"© 2026 christiano higuto — built slowly, on purpose"</span>
            <Konami />
        </footer>
    }
}
