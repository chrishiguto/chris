use content::IndexEntry;
use serde::{Deserialize, Serialize};

/// One listed post, in the shape the archive renders: title, date, and tags.
/// Internal fields (content hash, draft, read time, description) never reach
/// the client — this is what the filter island serializes as props.
#[derive(Clone, Serialize, Deserialize)]
pub struct ListedPost {
    pub slug: String,
    pub title: String,
    pub date: String,
    pub tags: Vec<String>,
}

impl From<IndexEntry> for ListedPost {
    fn from(entry: IndexEntry) -> Self {
        Self {
            slug: entry.slug,
            title: entry.title,
            date: entry.date,
            tags: entry.tags,
        }
    }
}
