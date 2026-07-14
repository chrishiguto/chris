use leptos::prelude::*;

/// The two-state theme switcher: flips `data-theme` on `<html>`
/// and persists the choice under [`crate::app::THEME_STORAGE_KEY`]. Unset follows
/// the system preference; the first explicit toggle opts out forever. Both
/// glyphs ship in the server HTML and CSS shows the one matching the
/// effective scheme, so the button can't flash a stale icon before
/// hydration — and stays a no-op without JS.
#[island]
pub fn ThemeToggle() -> impl IntoView {
    let toggle = move |_| {
        let root = document().document_element().expect("<html> must exist");
        let dark_now = match root.get_attribute("data-theme").as_deref() {
            Some("dark") => true,
            Some("light") => false,
            _ => window()
                .match_media("(prefers-color-scheme: dark)")
                .ok()
                .flatten()
                .is_some_and(|query| query.matches()),
        };
        let next = if dark_now { "light" } else { "dark" };
        root.set_attribute("data-theme", next).ok();
        // Storage can be denied (privacy modes); the flip still applies.
        if let Ok(Some(storage)) = window().local_storage() {
            storage.set_item(crate::app::THEME_STORAGE_KEY, next).ok();
        }
    };

    view! {
        <button type="button" class="theme-toggle" aria-label="toggle theme" on:click=toggle>
            // Moon invites the switch to dark, sun back.
            <span class="glyph when-light" aria-hidden="true">
                "☾"
            </span>
            <span class="glyph when-dark" aria-hidden="true">
                "☀"
            </span>
        </button>
    }
}
