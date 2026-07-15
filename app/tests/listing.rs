//! Listing pages render from the worker-provided index context; drafts filter at render time.
#![cfg(feature = "ssr")]

use app::listing::{HomePage, IndexData, PostsPage, RECENT_POSTS};
use common::{ssr, tag_containing};
use content::{Frontmatter, IndexEntry};
use leptos::prelude::provide_context;

mod common;

/// Through the real constructor, so publish-computed fields (read time,
/// content hash) default here the same way they do in production.
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
fn posts_page_lists_rows_in_the_post_row_shape() {
    let html = posts_html(vec![
        tagged("newer", "the newer post", "2026-03-01", &["rust", "wasm"]),
        entry("older", "the older post", "2026-01-01"),
    ]);
    // The markup shape the CSS styles:
    // ul.post-list > li > a.post-row > .post-row-top (+ .post-row-desc).
    assert!(html.contains("<ul class=\"post-list"), "{html}");
    assert!(
        !html.contains("<li hidden"),
        "the server render is the unfiltered baseline — every row visible: {html}"
    );
    let row = tag_containing(&html, "class=\"post-row\"");
    assert!(row.contains("href=\"/posts/newer\""), "{html}");
    assert!(
        html.contains("<span class=\"post-row-title\">the newer post"),
        "{html}"
    );
    let lead = tag_containing(&html, "class=\"post-row-lead\"");
    assert!(
        lead.contains("aria-hidden=\"true\"") && html.contains(">→</span>"),
        "the hover arrow must ship in the row markup: {html}"
    );
    assert!(
        html.contains("<span class=\"post-row-meta\"><span>mar 01, 2026</span></span>"),
        "{html}"
    );
    let newer = html.find("/posts/newer").unwrap();
    let older = html.find("/posts/older").unwrap();
    assert!(newer < older, "index order must be preserved: {html}");
}

#[test]
fn post_rows_render_the_description_only_when_present() {
    let mut described = entry("has-desc", "Described", "2026-02-01");
    described.description = Some("one honest line about the post".into());
    let html = posts_html(vec![described, entry("bare", "Bare", "2026-01-01")]);
    assert!(
        html.contains("<span class=\"post-row-desc\">one honest line about the post</span>"),
        "{html}"
    );
    assert_eq!(
        html.matches("post-row-desc").count(),
        1,
        "descriptionless rows must not render an empty desc line: {html}"
    );
}

// Minutes are publish-populated; a pre-slice-10 index has none, and those
// rows must show the date alone — no dangling separator.
#[test]
fn post_rows_show_read_time_only_when_present() {
    let mut timed = entry("timed", "Timed", "2026-02-01");
    timed.reading_minutes = Some(4);
    let html = posts_html(vec![timed, entry("legacy", "Legacy", "2026-01-01")]);
    assert!(
        html.contains("<span class=\"post-row-meta\"><span>feb 01, 2026</span>")
            && html.contains("<span>4 min</span>"),
        "{html}"
    );
    let sep = tag_containing(&html, "·");
    assert!(
        sep.contains("aria-hidden=\"true\""),
        "the separator hides from readers: {html}"
    );
    let sep_at = html.find("·").unwrap();
    assert!(
        html.find("feb 01, 2026").unwrap() < sep_at && sep_at < html.find("4 min").unwrap(),
        "the meta row must read `date · minutes`: {html}"
    );
    assert_eq!(
        html.matches("·").count(),
        1,
        "rows without minutes must not render a separator: {html}"
    );
    assert!(
        html.contains("<span class=\"post-row-meta\"><span>jan 01, 2026</span></span>"),
        "{html}"
    );
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
        html.contains("nothing published yet"),
        "empty index needs a readable state, not a blank page: {html}"
    );
}

// The draft is the newest entry, so it sits inside the RECENT_POSTS window.
#[test]
fn home_page_filters_drafts() {
    let mut draft = entry("wip", "Not yet", "2026-05-01");
    draft.draft = true;
    let html = home_html(vec![draft, entry("live", "Live", "2026-04-01")]);
    assert!(!html.contains("wip"), "drafts must not be listed: {html}");
    assert!(html.contains("/posts/live"), "{html}");
}

#[test]
fn home_page_greets_with_intro_links_and_section_label() {
    let html = home_html(vec![entry("post", "A post", "2026-01-01")]);
    assert!(html.contains("hey, i'm chris"), "{html}");
    let writing = tag_containing(&html, ">the writing<");
    assert!(
        writing.contains("href=\"/posts\"") && writing.contains("plink"),
        "{html}"
    );
    let about = tag_containing(&html, ">about me<");
    assert!(
        about.contains("href=\"/about\"") && about.contains("plink"),
        "{html}"
    );
    assert!(html.contains("latest writing"), "{html}");
}

#[test]
fn home_page_read_all_link_counts_the_listed_archive() {
    let mut draft = entry("wip", "Not yet", "2026-05-01");
    draft.draft = true;
    let index: Vec<_> = std::iter::once(draft)
        .chain((0..4).map(|i| {
            entry(
                &format!("post-{i}"),
                &format!("Post {i}"),
                &format!("2026-01-{:02}", 20 - i),
            )
        }))
        .collect();
    let html = home_html(index);
    assert!(
        html.contains("read all 4 posts →"),
        "the count must be the real listed total, drafts excluded: {html}"
    );
}

#[test]
fn home_page_shows_only_recent_posts_and_links_to_all() {
    assert_eq!(RECENT_POSTS, 3, "the design shows the latest three");
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

// The filter island owns the pill row and the list, with the listed posts
// as its props: pills are the post-pill shape, deduped and sorted, linking
// the `?q=` filter contract.
#[test]
fn posts_page_wraps_sorted_filter_pills_in_the_island() {
    let html = posts_html(vec![
        tagged("newer", "the newer post", "2026-03-01", &["wasm", "rust"]),
        tagged("older", "the older post", "2026-01-01", &["rust"]),
    ]);
    assert!(
        html.contains("<leptos-island"),
        "the filter must hydrate as an island: {html}"
    );
    let island = tag_containing(&html, "data-props");
    assert!(
        island.contains("newer") && island.contains("older"),
        "the listed posts ride as island props: {html}"
    );
    let pill = tag_containing(&html, "href=\"/posts?q=rust\"");
    assert!(pill.contains("class=\"tag\""), "{html}");
    let hash = tag_containing(&html, "class=\"tag-hash\"");
    assert!(
        hash.contains("aria-hidden=\"true\"") && html.contains(">#</span>wasm"),
        "pills carry the post-pill hash glyph: {html}"
    );
    assert_eq!(
        html.matches("/posts?q=rust").count(),
        1,
        "pills dedupe across posts: {html}"
    );
    let rust = html.find("/posts?q=rust").unwrap();
    let wasm = html.find("/posts?q=wasm").unwrap();
    assert!(rust < wasm, "pills sort alphabetically: {html}");
}

#[test]
fn filter_pills_skip_draft_only_tags() {
    let mut draft = tagged("wip", "Not yet", "2026-05-01", &["secret", "rust"]);
    draft.draft = true;
    let html = posts_html(vec![draft, tagged("live", "Live", "2026-04-01", &["rust"])]);
    assert!(!html.contains("secret"), "{html}");
    assert!(html.contains("/posts?q=rust"), "{html}");
}

// The `$ ls` line is island state shown only when a filter leaves no rows;
// the server render never ships it, so no-JS readers can't see it under
// the full list.
#[test]
fn posts_page_ssr_omits_the_ls_empty_state() {
    let html = posts_html(vec![tagged("a", "A", "2026-01-01", &["rust"])]);
    assert!(
        !html.contains("$ ls"),
        "the empty state must not ship in the server render: {html}"
    );
}

#[test]
fn posts_page_without_tags_has_no_filter_row() {
    let html = posts_html(vec![entry("plain", "Plain", "2026-01-01")]);
    assert!(!html.contains("post-tags"), "{html}");
    assert!(
        !html.contains("$ ls"),
        "no pills means the `$ ls` state can never show: {html}"
    );
}

// The tag routes are deleted end-to-end; the app router falls through to
// the 404 page.
#[test]
fn tag_routes_fall_through_to_the_404_page() {
    for path in ["/tags", "/tags/rust"] {
        let html = common::app_at(path);
        assert!(html.contains("404"), "`{path}` must 404: {html}");
    }
}

#[test]
fn home_page_without_index_context_still_renders() {
    // A missing IndexData context must degrade to the empty state, never panic.
    let html = ssr(|| (), || leptos::view! { <HomePage /> });
    assert!(html.contains("nothing published yet"), "{html}");
}
