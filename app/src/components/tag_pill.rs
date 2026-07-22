use leptos::prelude::*;

/// Tag pill: one `<li>` of the [`TagRow`](super::TagRow). The filter island
/// drives `active` and `on_select`; rendered statically, the pill is a plain
/// link to the pre-filtered listing.
#[component]
pub(crate) fn TagPill(
    tag: String,
    #[prop(optional)] active: Option<Signal<bool>>,
    #[prop(optional)] on_select: Option<Callback<()>>,
) -> impl IntoView {
    let href = content::tag_filter_path(&tag);
    view! {
        <li>
            <a
                class="tag"
                class:tag-active=move || active.is_some_and(|active| active.get())
                href=href
                on:click=move |ev| {
                    if let Some(on_select) = on_select {
                        ev.prevent_default();
                        on_select.run(());
                    }
                }
            >
                <span class="tag-hash" aria-hidden="true">
                    "#"
                </span>
                {tag}
            </a>
        </li>
    }
}
