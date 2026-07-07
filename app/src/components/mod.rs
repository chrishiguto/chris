//! Site-wide UI components. Post-embeddable components (the registry
//! vocabulary) live under [`blog`]; everything else here is app chrome.

pub mod blog;
pub mod header;
pub mod not_found;

pub use header::Header;
pub use not_found::NotFound;
