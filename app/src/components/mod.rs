//! Site-wide UI components; post-embeddable ones live under [`blog`].
//! One component per module — this file only wires the re-exports.

pub mod back_link;
pub mod blog;
pub mod code_block;
pub mod contacts;
pub mod footer;
pub mod header;
pub mod not_found;
pub mod page;
pub mod post_list;
pub mod post_meta;
pub mod section_label;
pub mod tag_pill;
pub mod tag_row;
pub mod theme_toggle;
pub mod writing_index;

pub use back_link::BackLink;
pub use code_block::CodeBlock;
pub use footer::Footer;
pub use header::Header;
pub use not_found::NotFound;
pub use theme_toggle::ThemeToggle;
pub use writing_index::WritingIndex;

pub(crate) use contacts::Contacts;
pub(crate) use page::{page_title, Page, PageShell, DISPLAY_HEADING_CLASS};
pub(crate) use post_list::{ListedPost, PostList};
pub(crate) use post_meta::PostMeta;
pub(crate) use section_label::{SectionLabel, SECTION_LABEL_CLASS};
pub(crate) use tag_pill::TagPill;
pub(crate) use tag_row::TagRow;
