//! Shared SSR harness for the app's integration tests.
// Compiled per test binary; each uses a subset, hence allow(dead_code).
#![allow(dead_code)]

use std::sync::Arc;

use hydration_context::SsrSharedContext;
use leptos::prelude::{Owner, RenderHtml};

/// SSR output carries `<!>` hydration markers, invisible to browsers; strip before asserting.
pub fn strip_markers(html: String) -> String {
    html.replace("<!>", "")
}

/// Renders a view the way the worker does: meta context plus `provide`, markers stripped.
pub fn ssr<V: RenderHtml>(provide: impl FnOnce(), view: impl FnOnce() -> V) -> String {
    // The shared context mirrors leptos_axum's per-request root owner;
    // islands with children unwrap it while serializing their slot.
    let owner = Owner::new_root(Some(Arc::new(SsrSharedContext::new())));
    owner.set();
    leptos_meta::provide_meta_context();
    provide();
    strip_markers(view().to_html())
}

/// `<App />` SSR'd at a request path, the way the worker serves a page.
pub fn app_at(path: &'static str) -> String {
    use leptos::prelude::provide_context;
    use leptos_router::location::RequestUrl;

    ssr(
        move || provide_context(RequestUrl::new(path)),
        || leptos::view! { <app::app::App /> },
    )
}

/// The opening tag around the first occurrence of `needle` — attribute
/// order in leptos output is an implementation detail, so assertions look
/// inside one tag instead of pinning full-tag strings.
pub fn tag_containing<'a>(html: &'a str, needle: &str) -> &'a str {
    let at = html
        .find(needle)
        .unwrap_or_else(|| panic!("no `{needle}` in: {html}"));
    let start = html[..at].rfind('<').expect("needle outside any tag");
    let end = at + html[at..].find('>').expect("unclosed tag");
    &html[start..=end]
}
