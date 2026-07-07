pub mod app;
pub mod components;
pub mod listing;
pub mod post;
pub mod render;

/// The deployed component vocabulary, collected from this crate's inventory
/// registrations — publish validation and `xtask check` validate against
/// exactly what the site renders with.
///
/// Lives here, beside the anchor symbol it references: the registrations
/// only link if at least one `app` symbol is referenced from the consuming
/// binary — with zero references the linker drops the rlib and the
/// vocabulary comes back empty (pinned by consumers' manifest tests).
pub fn manifest() -> content::Manifest {
    std::hint::black_box(app::PRELOADED_FONTS);
    registry::manifest()
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    leptos::mount::hydrate_islands();
}
