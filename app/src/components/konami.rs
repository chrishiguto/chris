use std::time::Duration;

use leptos::leptos_dom::helpers::window_event_listener;
use leptos::prelude::*;

/// The code, as `KeyboardEvent.key` values.
const KONAMI: [&str; 10] = [
    "ArrowUp",
    "ArrowUp",
    "ArrowDown",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "ArrowLeft",
    "ArrowRight",
    "b",
    "a",
];

/// The footer easter egg: the hint and the payoff ship as one package. The
/// hint is plain server HTML; a window keydown listener (attached in an
/// effect, so only in the browser, and removed with the island so it can't
/// leak) watches the last ten keys and floats the toast for 3.2s on a match.
#[island]
pub fn Konami() -> impl IntoView {
    let (unlocked, set_unlocked) = signal(false);
    let keys = StoredValue::new(Vec::<String>::new());

    Effect::new(move |_| {
        let handle = window_event_listener(leptos::ev::keydown, move |ev| {
            keys.update_value(|buf| {
                buf.push(ev.key());
                if buf.len() > KONAMI.len() {
                    buf.remove(0);
                }
            });
            if keys.with_value(|buf| buf.iter().map(String::as_str).eq(KONAMI)) {
                set_unlocked.set(true);
                // Re-triggers stack timeouts, exactly like the design mock.
                set_timeout(move || set_unlocked.set(false), Duration::from_millis(3200));
            }
        });
        on_cleanup(move || handle.remove());
    });

    view! {
        <span class="konami-hint" aria-hidden="true">
            "↑↑↓↓←→←→ba"
        </span>
        <Show when=move || unlocked.get()>
            <div class="konami-toast shadow-md motion-safe:animate-fade-up" role="status">
                "achievement unlocked: you know the code "
                <span class="konami-cursor" aria-hidden="true">
                    "▊"
                </span>
            </div>
        </Show>
    }
}
