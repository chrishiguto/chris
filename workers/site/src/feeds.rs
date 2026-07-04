//! Feed and sitemap rendering from the KV `index` (user story 21) — pure
//! string builders over [`IndexEntry`], natively testable; the wasm shim in
//! `server` only wires them to `/rss.xml` and `/sitemap.xml`.
//!
//! The feed is Atom (RFC 4287), served at `/rss.xml` per the PRD's URL set:
//! Atom takes ISO-8601 timestamps directly, so frontmatter dates need no
//! RFC-822 weekday arithmetic.

use content_ast::IndexEntry;

const SITE_TITLE: &str = "chris";
const AUTHOR: &str = "chris";
/// Feed-level `updated` when nothing is published yet (Atom requires one).
const EPOCH: &str = "1970-01-01";

/// Published entries only, in stored (newest-first) order.
fn published(index: &[IndexEntry]) -> impl Iterator<Item = &IndexEntry> {
    index.iter().filter(|entry| !entry.draft)
}

/// Escapes text for XML element content and attribute values.
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
            let url = format!("{origin}/posts/{}", entry.slug);
            // RFC 4287 §4.1.2: with no atom:content, atom:summary is
            // required — fall back to the title when there is no description.
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
         <link rel=\"self\" href=\"{origin}/rss.xml\"/>\
         <updated>{updated}</updated>\
         <author><name>{author}</name></author>\n\
         {entries}</feed>\n",
        title = xml_escape(SITE_TITLE),
        updated = atom_timestamp(updated),
        author = xml_escape(AUTHOR),
    )
}

/// `/sitemap.xml`: home, the post list, tag pages, and every published post
/// (with its publication date as `lastmod`).
pub fn sitemap(origin: &str, index: &[IndexEntry]) -> String {
    let tags: std::collections::BTreeSet<&str> = published(index)
        .flat_map(|entry| entry.tags.iter())
        .map(String::as_str)
        .collect();

    let urls = ["/", "/posts", "/tags"]
        .into_iter()
        .map(String::from)
        .chain(tags.into_iter().map(|tag| format!("/tags/{tag}")))
        .map(|path| format!("<url><loc>{origin}{path}</loc></url>\n"))
        .chain(published(index).map(|entry| {
            format!(
                "<url><loc>{origin}/posts/{slug}</loc><lastmod>{date}</lastmod></url>\n",
                slug = entry.slug,
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
