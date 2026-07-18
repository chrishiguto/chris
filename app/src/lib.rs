// Deeply nested tachys view types in the islands overflow rustc's default query depth.
#![recursion_limit = "256"]

pub mod about;
pub mod app;
mod classed;
pub mod components;
pub mod listing;
pub mod post;
pub mod render;

/// The component vocabulary from this crate's inventory registrations.
/// The `black_box` anchor is load-bearing: without a referenced `app` symbol
/// the linker drops the rlib and the vocabulary comes back empty.
pub fn manifest() -> content::Manifest {
    std::hint::black_box(app::GOOGLE_FONTS_URL);
    registry::manifest()
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    leptos::mount::hydrate_islands();
}
