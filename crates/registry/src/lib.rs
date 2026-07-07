//! Component registry: `#[post_component]` dispatch and `inventory`-backed
//! registration, behind the `dispatch` feature. The re-exports keep
//! macro-generated `::registry::…` paths (and manifest-only consumers) working.

pub use content::{integral, ComponentSpec, Manifest, PropSpec, PropType, PropValue};

#[cfg(feature = "dispatch")]
mod dispatch;
#[cfg(feature = "dispatch")]
pub use dispatch::*;
