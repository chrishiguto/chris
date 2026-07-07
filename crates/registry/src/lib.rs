//! The component registry: one source of truth for which
//! components exist, with which props, consumed three ways — render dispatch
//! in the site worker, publish validation in the pipeline, and `xtask check`.
//!
//! The vocabulary *types* ([`Manifest`], [`ComponentSpec`], [`PropSpec`],
//! [`PropType`]) live in the `content` crate so the parser validates without
//! depending on this crate; this crate is the layer that *produces* a
//! [`Manifest`]: the `#[post_component]` macro, `inventory`-backed
//! registration, [`lookup`], and [`manifest`], all behind the `dispatch`
//! feature. The re-exports below keep macro-generated `::registry::…` paths
//! (and manifest-only consumers) working unchanged.

pub use content::{integral, ComponentSpec, Manifest, PropSpec, PropType, PropValue};

#[cfg(feature = "dispatch")]
mod dispatch;
#[cfg(feature = "dispatch")]
pub use dispatch::*;
