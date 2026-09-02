//! The writing route's SSR contract: a complete no-JS archive inside the
//! filter island, with drafts removed before props are serialized.
#![cfg(feature = "ssr")]

use app::writing::{IndexData, WritingPage};
use common::{ssr, tag_containing};
use content::{Frontmatter, IndexEntry};
use leptos::prelude::provide_context;

mod common;

fn entry(slug: &str, title: &str, date: &str) -> IndexEntry {
    IndexEntry::new(
        slug,
        &Frontmatter {
            title: title.into(),
            date: date.into(),
            description: None,
            tags: vec![],
            draft: false,
        },
    )
}

fn tagged(slug: &str, title: &str, date: &str, tags: &[&str]) -> IndexEntry {
    let mut entry = entry(slug, title, date);
    entry.tags = tags.iter().map(|tag| tag.to_string()).collect();
    entry
}

fn writing_html(index: Vec<IndexEntry>) -> String {
    ssr(
        move || provide_context(IndexData(index)),
        || leptos::view! { <WritingPage /> },
    )
}

#[test]
fn writing_opens_with_home_wayfinding_intro_and_ghost_heading() {
    let html = writing_html(vec![entry("one", "one", "2026-01-01")]);
    let nav = tag_containing(&html, "aria-label=\"back to home\"");
    assert!(nav.contains("class=\"gutter-nav\""), "{html}");
    assert!(html.contains("<a href=\"/\">← home</a>"), "{html}");
    assert!(html.contains("1 post · "), "{html}");
    assert!(html.contains("href=\"/rss.xml\""), "{html}");
    assert!(
        html.contains("<h2 class=\"sr-only\">writing</h2>"),
        "{html}"
    );
    assert!(html.contains("class=\"ghost-word\""), "{html}");
}

#[test]
fn writing_groups_newest_first_rows_by_year() {
    let html = writing_html(vec![
        entry("new", "new title", "2026-07-04"),
        entry("middle", "middle title", "2025-11-03"),
        entry("old", "old title", "2025-02-01"),
    ]);
    let y2026 = html.find(">2026</h2>").unwrap();
    let new = html.find("/posts/new").unwrap();
    let y2025 = html.find(">2025</h2>").unwrap();
    let middle = html.find("/posts/middle").unwrap();
    let old = html.find("/posts/old").unwrap();
    assert!(
        y2026 < new && new < y2025 && y2025 < middle && middle < old,
        "{html}"
    );
    assert!(html.contains("class=\"hover-date-row\""), "{html}");
    assert!(html.contains(">4 july</span>"), "{html}");
    assert!(
        !html.contains("post-row-desc"),
        "titles stay primary: {html}"
    );
}

#[test]
fn writing_serializes_only_listed_posts_into_the_filter_island() {
    let mut draft = tagged("wip", "not yet", "2026-05-01", &["secret"]);
    draft.draft = true;
    let html = writing_html(vec![draft, tagged("live", "live", "2026-04-01", &["rust"])]);
    assert!(html.contains("<leptos-island"), "{html}");
    let props = tag_containing(&html, "data-props");
    assert!(props.contains("live") && props.contains("rust"), "{html}");
    assert!(!html.contains("wip") && !html.contains("secret"), "{html}");
}

#[test]
fn writing_renders_sorted_inert_tag_words_for_no_js() {
    let html = writing_html(vec![
        tagged("new", "new", "2026-03-01", &["wasm", "rust"]),
        tagged("old", "old", "2025-01-01", &["rust"]),
    ]);
    let rust = html.find("href=\"/writing?q=rust\"").unwrap();
    let wasm = html.find("href=\"/writing?q=wasm\"").unwrap();
    assert!(rust < wasm, "tag words sort and dedupe: {html}");
    assert_eq!(html.matches("/writing?q=rust").count(), 1, "{html}");
    assert!(html.contains("class=\"writing-tag\""), "{html}");
    assert!(
        !html.contains("class=\"tag\""),
        "filter controls are words, not pills: {html}"
    );
    assert!(
        !html.contains(" hidden"),
        "SSR keeps every group and row visible: {html}"
    );
}

#[test]
fn writing_keeps_filter_visibility_and_empty_state_reactive_only() {
    let html = writing_html(vec![tagged("one", "one", "2026-01-01", &["rust"])]);
    assert!(html.contains("class=\"writing-year\""), "{html}");
    assert!(
        !html.contains("nothing here yet"),
        "SSR must not hide the full archive: {html}"
    );
}

#[test]
fn writing_deletes_search_topics_and_clamp_markup() {
    let html = writing_html(vec![tagged("one", "one", "2026-01-01", &["rust"])]);
    for retired in [
        "type=\"search\"",
        "topics",
        "topics-more",
        "show all",
        "show less",
    ] {
        assert!(
            !html.contains(retired),
            "retired `{retired}` leaked: {html}"
        );
    }
}

#[test]
fn writing_lists_every_post_and_handles_tagless_archives() {
    let html = writing_html(
        (0..5)
            .map(|i| entry(&format!("p-{i}"), &format!("p {i}"), "2026-01-01"))
            .collect(),
    );
    for i in 0..5 {
        assert!(html.contains(&format!("/posts/p-{i}")), "{html}");
    }
    assert!(html.contains("5 posts · "), "{html}");
    assert!(html.contains("class=\"writing-tags\"></ul>"), "{html}");
}

#[test]
fn writing_empty_or_missing_index_has_a_readable_state() {
    assert!(writing_html(Vec::new()).contains("nothing published yet"));
    let html = ssr(|| (), || leptos::view! { <WritingPage /> });
    assert!(html.contains("nothing published yet"), "{html}");
}

#[test]
fn retired_tag_routes_still_fall_through_to_404() {
    for path in ["/tags", "/tags/rust"] {
        let html = common::app_at(path);
        assert!(html.contains("404"), "`{path}` must 404: {html}");
    }
}
