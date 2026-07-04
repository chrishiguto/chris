//! Native tests for feed/sitemap rendering over a fixture index — the pure
//! half of the site worker (the wasm shim only wires these to routes).

use content_ast::IndexEntry;
use site::feeds;

const ORIGIN: &str = "https://example.com";

fn entry(slug: &str, title: &str, date: &str) -> IndexEntry {
    IndexEntry {
        slug: slug.into(),
        title: title.into(),
        date: date.into(),
        description: None,
        tags: vec![],
        draft: false,
    }
}

fn fixture_index() -> Vec<IndexEntry> {
    let mut newer = entry("newer", "The newer post", "2026-03-01");
    newer.description = Some("A summary of the newer post.".into());
    newer.tags = vec!["rust".into(), "wasm".into()];
    let mut older = entry("older", "The older post", "2026-01-01");
    older.tags = vec!["rust".into()];
    let mut draft = entry("wip", "Not yet", "2026-05-01");
    draft.draft = true;
    draft.tags = vec!["secret".into()];
    vec![draft, newer, older]
}

#[test]
fn feed_includes_title_date_link_and_description_per_post() {
    let xml = feeds::atom(ORIGIN, &fixture_index());
    assert!(xml.contains("<title>The newer post</title>"), "{xml}");
    assert!(
        xml.contains("<updated>2026-03-01T00:00:00Z</updated>"),
        "{xml}"
    );
    assert!(
        xml.contains("<link href=\"https://example.com/posts/newer\"/>"),
        "{xml}"
    );
    assert!(
        xml.contains("<summary>A summary of the newer post.</summary>"),
        "{xml}"
    );
}

#[test]
fn feed_is_a_well_formed_atom_document() {
    let xml = feeds::atom(ORIGIN, &fixture_index());
    assert!(
        xml.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>"),
        "{xml}"
    );
    assert!(
        xml.contains("<feed xmlns=\"http://www.w3.org/2005/Atom\">"),
        "{xml}"
    );
    assert!(
        xml.contains("<link rel=\"self\" href=\"https://example.com/rss.xml\"/>"),
        "{xml}"
    );
    // Feed-level updated is the newest published entry's date.
    assert!(
        xml.contains("<updated>2026-03-01T00:00:00Z</updated>"),
        "{xml}"
    );
    assert!(xml.contains("<author><name>"), "{xml}");
    assert!(xml.trim_end().ends_with("</feed>"), "{xml}");
    // Every entry needs an id and — absent atom:content — a summary (RFC 4287).
    assert_eq!(xml.matches("<entry>").count(), 2, "{xml}");
    assert_eq!(xml.matches("<id>").count(), 3, "{xml}");
    assert_eq!(xml.matches("<summary>").count(), 2, "{xml}");
}

#[test]
fn feed_summary_falls_back_to_the_title_when_description_is_absent() {
    let xml = feeds::atom(ORIGIN, &fixture_index());
    assert!(xml.contains("<summary>The older post</summary>"), "{xml}");
}

#[test]
fn feed_excludes_drafts() {
    let xml = feeds::atom(ORIGIN, &fixture_index());
    assert!(!xml.contains("wip"), "drafts must stay out of feeds: {xml}");
    assert!(!xml.contains("Not yet"), "{xml}");
}

#[test]
fn feed_escapes_xml_special_characters() {
    let mut spicy = entry("spicy", "Q&A: <tags> & \"quotes\"", "2026-02-01");
    spicy.description = Some("less <than & more".into());
    let xml = feeds::atom(ORIGIN, &[spicy]);
    assert!(
        xml.contains("<title>Q&amp;A: &lt;tags&gt; &amp; &quot;quotes&quot;</title>"),
        "{xml}"
    );
    assert!(
        xml.contains("<summary>less &lt;than &amp; more</summary>"),
        "{xml}"
    );
}

#[test]
fn empty_index_still_yields_a_valid_feed() {
    let xml = feeds::atom(ORIGIN, &[]);
    assert!(xml.contains("<feed"), "{xml}");
    assert!(
        xml.contains("<updated>1970-01-01T00:00:00Z</updated>"),
        "an empty feed still needs a feed-level updated: {xml}"
    );
    assert!(!xml.contains("<entry>"), "{xml}");
}

#[test]
fn sitemap_lists_home_post_list_posts_and_tag_pages() {
    let xml = feeds::sitemap(ORIGIN, &fixture_index());
    for loc in [
        "<loc>https://example.com/</loc>",
        "<loc>https://example.com/posts</loc>",
        "<loc>https://example.com/tags</loc>",
        "<loc>https://example.com/tags/rust</loc>",
        "<loc>https://example.com/tags/wasm</loc>",
        "<loc>https://example.com/posts/newer</loc>",
        "<loc>https://example.com/posts/older</loc>",
    ] {
        assert!(xml.contains(loc), "missing {loc}: {xml}");
    }
    assert!(
        xml.contains("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">"),
        "{xml}"
    );
    // Post URLs carry their publication date as lastmod.
    assert!(xml.contains("<lastmod>2026-03-01</lastmod>"), "{xml}");
}

#[test]
fn sitemap_excludes_drafts_and_their_tags() {
    let xml = feeds::sitemap(ORIGIN, &fixture_index());
    assert!(!xml.contains("/posts/wip"), "{xml}");
    assert!(
        !xml.contains("/tags/secret"),
        "a tag only drafts carry has no page: {xml}"
    );
}
