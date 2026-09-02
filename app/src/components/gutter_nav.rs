use leptos::prelude::*;

/// Inner-page wayfinding occupies the page grid's left gutter on wide
/// screens. CSS folds the same plain link above the title when no gutter fits.
#[component]
pub fn GutterNav(href: &'static str, label: &'static str) -> impl IntoView {
    view! {
        <nav class="gutter-nav" aria-label="back to home">
            <a href=href>{format!("← {label}")}</a>
        </nav>
    }
}
