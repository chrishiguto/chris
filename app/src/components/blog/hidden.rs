use leptos::prelude::*;
use registry::post_component;

use crate::components::Fold;

/// Progressive disclosure for post prose: the home's fold, registered so
/// authors can write `<Hidden>…</Hidden>`. Children only, no props.
#[post_component]
#[component]
pub fn Hidden(children: Children) -> impl IntoView {
    view! { <Fold label="reveal hidden text">{children()}</Fold> }
}
