use leptos::prelude::*;

use crate::classed::classed;

/// The tag row — `ul.post-tags` — that closes an article with its tag words;
/// no items, no row. Spacing overrides belong to callers.
#[component]
pub(crate) fn TagRow<V: IntoView + 'static>(pills: Vec<V>, spacing: &'static str) -> impl IntoView {
    (!pills.is_empty()).then(|| view! { <ul class=classed("post-tags", spacing)>{pills}</ul> })
}
