//! Listing pages: `/posts` and `/` render from the KV `index`
//! provided by the site worker via context; drafts are filtered at render
//! time. Run with `cargo test -p app --features ssr`.
#![cfg(feature = "ssr")]

use app::listing::{HomePage, IndexData, PostsPage, TagListing, TagsPage, RECENT_POSTS};
use common::ssr;
use content::IndexEntry;
use leptos::prelude::provide_context;

mod common;

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

fn tagged(slug: &str, title: &str, date: &str, tags: &[&str]) -> IndexEntry {
    let mut entry = entry(slug, title, date);
    entry.tags = tags.iter().map(|t| t.to_string()).collect();
    entry
}

fn posts_html(index: Vec<IndexEntry>) -> String {
    ssr(
        move || provide_context(IndexData(index)),
        || leptos::view! { <PostsPage /> },
    )
}

fn home_html(index: Vec<IndexEntry>) -> String {
    ssr(
        move || provide_context(IndexData(index)),
        || leptos::view! { <HomePage /> },
    )
}

#[test]
fn posts_page_lists_title_and_date_linking_to_posts() {
    let html = posts_html(vec![
        entry("newer", "The newer post", "2026-03-01"),
        entry("older", "The older post", "2026-01-01"),
    ]);
    // The markup shape main.css styles: ul.post-list > li > a > h2 + p.post-date.
    assert!(html.contains("<ul class=\"post-list\">"), "{html}");
    assert!(html.contains("<a href=\"/posts/newer\">"), "{html}");
    assert!(html.contains("<h2>The newer post</h2>"), "{html}");
    assert!(
        html.contains("<p class=\"post-date\">2026-03-01</p>"),
        "{html}"
    );
    let newer = html.find("/posts/newer").unwrap();
    let older = html.find("/posts/older").unwrap();
    assert!(newer < older, "index order must be preserved: {html}");
}

#[test]
fn posts_page_filters_drafts() {
    let mut draft = entry("wip", "Not yet", "2026-05-01");
    draft.draft = true;
    let html = posts_html(vec![draft, entry("live", "Live", "2026-04-01")]);
    assert!(!html.contains("wip"), "drafts must not be listed: {html}");
    assert!(html.contains("/posts/live"), "{html}");
}

#[test]
fn posts_page_with_empty_index_says_so() {
    let html = posts_html(Vec::new());
    assert!(
        html.contains("Nothing published yet"),
        "empty index needs a readable state, not a blank page: {html}"
    );
}

// The home page shares the draft filter — a draft must not leak
// onto `/` even when it is the newest entry (and thus inside RECENT_POSTS).
#[test]
fn home_page_filters_drafts() {
    let mut draft = entry("wip", "Not yet", "2026-05-01");
    draft.draft = true;
    let html = home_html(vec![draft, entry("live", "Live", "2026-04-01")]);
    assert!(!html.contains("wip"), "drafts must not be listed: {html}");
    assert!(html.contains("/posts/live"), "{html}");
}

#[test]
fn home_page_shows_only_recent_posts_and_links_to_all() {
    let index: Vec<_> = (0..RECENT_POSTS + 2)
        .map(|i| {
            entry(
                &format!("post-{i}"),
                &format!("Post {i}"),
                &format!("2026-01-{:02}", 20 - i),
            )
        })
        .collect();
    let html = home_html(index);
    assert!(
        html.contains(&format!("/posts/post-{}", RECENT_POSTS - 1)),
        "{html}"
    );
    assert!(
        !html.contains(&format!("/posts/post-{RECENT_POSTS}")),
        "home shows at most {RECENT_POSTS} posts: {html}"
    );
    assert!(
        html.contains("href=\"/posts\""),
        "missing all-posts link: {html}"
    );
}

fn tags_html(index: Vec<IndexEntry>) -> String {
    ssr(
        move || provide_context(IndexData(index)),
        || leptos::view! { <TagsPage /> },
    )
}

fn tag_html(tag: &str, index: Vec<IndexEntry>) -> String {
    let tag = tag.to_string();
    ssr(
        move || provide_context(IndexData(index)),
        move || leptos::view! { <TagListing tag=tag /> },
    )
}

#[test]
fn tags_page_lists_tags_with_counts_linking_to_tag_pages() {
    let html = tags_html(vec![
        tagged("a", "A", "2026-03-01", &["rust", "wasm"]),
        tagged("b", "B", "2026-01-01", &["rust"]),
    ]);
    assert!(html.contains("<ul class=\"post-tags"), "{html}");
    assert!(html.contains("<a href=\"/tags/rust\">"), "{html}");
    assert!(html.contains("<a href=\"/tags/wasm\">"), "{html}");
    let rust = &html[html.find("/tags/rust").unwrap()..];
    assert!(
        rust.starts_with("/tags/rust\">rust</a> ×2"),
        "rust must show its post count: {html}"
    );
}

#[test]
fn tags_page_ignores_draft_only_tags() {
    let mut draft = tagged("wip", "Not yet", "2026-05-01", &["secret", "rust"]);
    draft.draft = true;
    let html = tags_html(vec![draft, tagged("live", "Live", "2026-04-01", &["rust"])]);
    assert!(!html.contains("secret"), "{html}");
    let rust = &html[html.find("/tags/rust").unwrap()..];
    assert!(
        rust.starts_with("/tags/rust\">rust</a> ×1"),
        "drafts must not count: {html}"
    );
}

#[test]
fn tags_page_with_no_tags_says_so() {
    let html = tags_html(vec![entry("untagged", "No tags here", "2026-01-01")]);
    assert!(html.contains("Nothing is tagged yet"), "{html}");
}

#[test]
fn tag_listing_shows_only_matching_posts() {
    let html = tag_html(
        "rust",
        vec![
            tagged("match", "Matches", "2026-03-01", &["rust"]),
            tagged("other", "Other", "2026-02-01", &["wasm"]),
        ],
    );
    assert!(html.contains("<ul class=\"post-list\">"), "{html}");
    assert!(html.contains("/posts/match"), "{html}");
    assert!(!html.contains("/posts/other"), "{html}");
}

#[test]
fn tag_listing_excludes_drafts() {
    let mut draft = tagged("wip", "Not yet", "2026-05-01", &["rust"]);
    draft.draft = true;
    let html = tag_html("rust", vec![draft]);
    assert!(!html.contains("/posts/wip"), "{html}");
}

#[test]
fn tag_listing_for_unknown_tag_renders_a_readable_state() {
    // The worker sets the 404 status (acceptance: unknown tag → 404); the
    // body still needs to read as a page.
    let html = tag_html("nope", vec![tagged("a", "A", "2026-01-01", &["rust"])]);
    assert!(html.contains("Nothing is tagged"), "{html}");
}

#[test]
fn home_page_without_index_context_still_renders() {
    // A missing IndexData context must degrade to the empty state, never
    // panic — nothing guarantees every future render site provides it.
    let html = ssr(|| (), || leptos::view! { <HomePage /> });
    assert!(html.contains("Nothing published yet"), "{html}");
}
