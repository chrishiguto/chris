use leptos::prelude::*;

use crate::classed::classed;

/// The pill row — `ul.post-tags` of [`TagPill`](super::TagPill)s — shared by
/// the article bottom and the filter rail; no pills, no row. Spacing
/// overrides belong to callers.
#[component]
pub(crate) fn TagRow<V: IntoView + 'static>(pills: Vec<V>, spacing: &'static str) -> impl IntoView {
    (!pills.is_empty()).then(|| view! { <ul class=classed("post-tags", spacing)>{pills}</ul> })
}
