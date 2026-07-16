use content::POSTS_PATH;
use leptos::prelude::*;

/// The article's way back. SSRs as a plain link to the listing; hydration
/// upgrades the click to `history.back()` when the reader arrived from this
/// site, returning them to whatever they left — a filtered listing, home,
/// another post. Direct visits, external referrers, and no-JS readers all
/// follow the href instead, so the control never strands anyone off-site.
#[island]
pub fn BackLink() -> impl IntoView {
    let back = move |ev: leptos::ev::MouseEvent| {
        // A same-origin referrer is the signal the previous entry is ours;
        // prefix-match up to a path boundary so a lookalike host can't pass.
        let referrer = document().referrer();
        let origin = window().location().origin().unwrap_or_default();
        let on_site = !referrer.is_empty()
            && referrer
                .strip_prefix(origin.as_str())
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'));
        if on_site {
            if let Ok(history) = window().history() {
                ev.prevent_default();
                history.back().ok();
            }
        }
    };

    view! {
        <a
            href=POSTS_PATH
            class="group inline-flex items-baseline gap-1.5 bg-none text-sm font-medium text-ink-2 transition-colors duration-120 ease-out hover:text-accent"
            on:click=back
        >
            // The arrow mirrors the listing rows' slide-in lead; reduced
            // motion leaves it parked.
            <span
                class="inline-block transition-transform duration-200 ease-out-expo motion-safe:group-hover:-translate-x-[3px]"
                aria-hidden="true"
            >
                "←"
            </span>
            "back"
        </a>
    }
}
