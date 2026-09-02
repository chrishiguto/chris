//! Site-wide UI components; post-embeddable ones live under [`blog`].
//! One component per module — this file only wires the re-exports.

pub mod blog;
pub mod code_block;
pub mod contacts;
pub mod footer;
pub mod gutter_nav;
pub mod heading;
pub mod not_found;
pub mod page;
pub mod post_list;
pub mod post_meta;
pub mod section_label;
pub mod tag_pill;
pub mod tag_row;
pub mod veil;
pub mod writing_index;

pub use code_block::CodeBlock;
pub use footer::Footer;
pub use gutter_nav::GutterNav;
pub use not_found::NotFound;
pub use veil::Veil;
pub use writing_index::WritingIndex;

pub(crate) use contacts::Contacts;
pub(crate) use heading::Heading;
pub(crate) use page::{page_title, Page, PageShell};
pub(crate) use post_list::{ListedPost, PostList};
pub(crate) use post_meta::PostMeta;
pub(crate) use section_label::SectionLabel;
pub(crate) use tag_pill::TagPill;
pub(crate) use tag_row::TagRow;
