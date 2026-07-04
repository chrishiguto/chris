pub mod callout;
pub mod counter;

// Co-located per-post components (ADR-0004), discovered by build.rs from
// content/blog/*/components.rs.
include!(concat!(env!("OUT_DIR"), "/post_components.rs"));
