//! Site-wide UI components; post-embeddable ones live under [`blog`].
//! One component per module — this file only wires the re-exports.

pub mod blog;
pub mod code_block;
pub mod fold;
pub mod footer;
pub mod ghost_word;
pub mod gutter_nav;
pub mod heading;
pub mod hover_date_row;
pub mod not_found;
pub mod page;
pub mod post_list;
pub mod post_meta;
pub mod writing_index;

pub use code_block::CodeBlock;
pub use fold::Fold;
pub use footer::Footer;
pub use ghost_word::GhostWord;
pub use gutter_nav::GutterNav;
pub use hover_date_row::HoverDateRow;
pub use not_found::NotFound;
pub use writing_index::WritingIndex;

pub(crate) use heading::Heading;
pub(crate) use page::{page_title, Page};
pub(crate) use post_list::ListedPost;
pub(crate) use post_meta::PostMeta;
