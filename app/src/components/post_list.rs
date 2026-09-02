use content::IndexEntry;
use serde::{Deserialize, Serialize};

/// One listed post, in the shape the pages render: the published subset of
/// an index entry. Internal fields (content hash, draft) never reach the
/// client — this is what the filter island serializes as props.
#[derive(Clone, Serialize, Deserialize)]
pub struct ListedPost {
    pub slug: String,
    pub title: String,
    pub date: String,
    pub reading_minutes: Option<u32>,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

impl From<IndexEntry> for ListedPost {
    fn from(entry: IndexEntry) -> Self {
        Self {
            slug: entry.slug,
            title: entry.title,
            date: entry.date,
            reading_minutes: entry.reading_minutes,
            description: entry.description,
            tags: entry.tags,
        }
    }
}
