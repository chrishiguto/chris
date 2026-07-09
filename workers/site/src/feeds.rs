//! Feed and sitemap rendering over the KV index — pure string builders,
//! natively testable. The feed is Atom served at `/rss.xml` (Atom takes
//! ISO-8601 dates directly, unlike RSS's RFC-822).

use content::{post_path, tag_path, IndexEntry, LISTING_PAGES, RSS_PATH, STATIC_PAGES};

const SITE_TITLE: &str = "chris";
const AUTHOR: &str = "chris";
/// Feed-level `updated` when nothing is published yet (Atom requires one).
const EPOCH: &str = "1970-01-01";

/// Published entries in stored (newest-first) order.
fn published(index: &[IndexEntry]) -> impl Iterator<Item = &IndexEntry> {
    index.iter().filter(|entry| entry.is_listed())
}

fn xml_escape(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '&' => "&amp;".into(),
            '<' => "&lt;".into(),
            '>' => "&gt;".into(),
            '"' => "&quot;".into(),
            '\'' => "&apos;".into(),
            c => c.to_string(),
        })
        .collect()
}

/// Frontmatter dates are date-only; Atom wants a full timestamp.
fn atom_timestamp(date: &str) -> String {
    format!("{date}T00:00:00Z")
}

/// The Atom feed served at `/rss.xml`. `origin` is scheme + host, no
/// trailing slash.
pub fn atom(origin: &str, index: &[IndexEntry]) -> String {
    let updated = published(index)
        .map(|entry| entry.date.as_str())
        .max()
        .unwrap_or(EPOCH);

    let entries = published(index)
        .map(|entry| {
            let url = format!("{origin}{}", post_path(&entry.slug));
            // With no atom:content, Atom requires atom:summary — fall back
            // to the title when there is no description.
            let summary = entry.description.as_deref().unwrap_or(&entry.title);
            format!(
                "<entry>\
                 <title>{title}</title>\
                 <id>{url}</id>\
                 <link href=\"{url}\"/>\
                 <updated>{updated}</updated>\
                 <summary>{summary}</summary>\
                 </entry>\n",
                title = xml_escape(&entry.title),
                updated = atom_timestamp(&entry.date),
                summary = xml_escape(summary),
            )
        })
        .collect::<String>();

    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
         <feed xmlns=\"http://www.w3.org/2005/Atom\">\n\
         <title>{title}</title>\
         <id>{origin}/</id>\
         <link href=\"{origin}/\"/>\
         <link rel=\"self\" href=\"{origin}{rss}\"/>\
         <updated>{updated}</updated>\
         <author><name>{author}</name></author>\n\
         {entries}</feed>\n",
        title = xml_escape(SITE_TITLE),
        updated = atom_timestamp(updated),
        author = xml_escape(AUTHOR),
        rss = RSS_PATH,
    )
}

/// Home, listing pages, static pages, tag pages, and every published post
/// (publication date as `lastmod`).
pub fn sitemap(origin: &str, index: &[IndexEntry]) -> String {
    let tags: std::collections::BTreeSet<&str> = published(index)
        .flat_map(|entry| entry.tags.iter())
        .map(String::as_str)
        .collect();

    let urls = LISTING_PAGES
        .into_iter()
        .chain(STATIC_PAGES)
        .map(String::from)
        .chain(tags.into_iter().map(tag_path))
        .map(|path| format!("<url><loc>{origin}{path}</loc></url>\n"))
        .chain(published(index).map(|entry| {
            format!(
                "<url><loc>{origin}{path}</loc><lastmod>{date}</lastmod></url>\n",
                path = post_path(&entry.slug),
                date = entry.date,
            )
        }))
        .collect::<String>();

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n\
         {urls}</urlset>\n"
    )
}
