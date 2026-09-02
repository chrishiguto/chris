use leptos::prelude::*;

/// Inner-page wayfinding occupies the page grid's left gutter on wide
/// screens. CSS folds the same plain link above the title when no gutter fits.
#[component]
pub fn GutterNav(href: &'static str, label: &'static str) -> impl IntoView {
    let accessible_label = format!("back to {label}");
    view! {
        <nav class="gutter-nav" aria-label=accessible_label>
            <a href=href>{format!("← {label}")}</a>
        </nav>
    }
}
