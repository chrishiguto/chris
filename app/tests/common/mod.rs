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
