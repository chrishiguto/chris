//! Shared SSR harness for the app's integration tests.
// Each test binary compiles this module separately and uses a subset of it.
#![allow(dead_code)]

use leptos::prelude::{Owner, RenderHtml};

/// `AnyView` emits `<!>` hydration-marker comments in SSR output; they are
/// invisible to browsers, so assertions compare with them stripped.
pub fn strip_markers(html: String) -> String {
    html.replace("<!>", "")
}

/// Renders a view the way the worker does: meta context plus whatever
/// `provide` adds, on a reactive owner, SSR'd with markers stripped.
pub fn ssr<V: RenderHtml>(provide: impl FnOnce(), view: impl FnOnce() -> V) -> String {
    let owner = Owner::new();
    owner.set();
    leptos_meta::provide_meta_context();
    provide();
    strip_markers(view().to_html())
}
