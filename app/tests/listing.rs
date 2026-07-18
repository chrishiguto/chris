//! The home listing renders from the worker-provided index context; drafts
//! filter at render time, and the tag-filter island wraps the rows.
#![cfg(feature = "ssr")]

use app::listing::{HomePage, IndexData};
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

fn home_html(index: Vec<IndexEntry>) -> String {
    ssr(
        move || provide_context(IndexData(index)),
        || leptos::view! { <HomePage /> },
    )
}

#[test]
fn home_lists_rows_in_the_post_row_shape() {
    let html = home_html(vec![
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
    let html = home_html(vec![described, entry("bare", "Bare", "2026-01-01")]);
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
    let html = home_html(vec![timed, entry("legacy", "Legacy", "2026-01-01")]);
    assert!(
        html.contains("<span class=\"post-row-meta\"><span>feb 01, 2026</span>")
            && html.contains("<span>4 min</span>"),
        "{html}"
    );
    // The timed row's meta reads `date · minutes`, the separator hidden from
    // assistive tech. Scope to the region between date and minutes so the
    // writing header's own `·` can't be mistaken for the row separator.
    let date_at = html.find("feb 01, 2026").expect("date missing");
    let min_at = html.find("4 min").expect("minutes missing");
    assert!(date_at < min_at, "date must precede minutes: {html}");
    let between = &html[date_at..min_at];
    assert!(
        between.contains("·") && between.contains("aria-hidden=\"true\""),
        "a hidden `·` separator sits between date and minutes: {html}"
    );
    // The minutes-less row shows the date alone — no dangling separator.
    assert!(
        html.contains("<span class=\"post-row-meta\"><span>jan 01, 2026</span></span>"),
        "{html}"
    );
}

#[test]
fn home_filters_drafts() {
    let mut draft = entry("wip", "Not yet", "2026-05-01");
    draft.draft = true;
    let html = home_html(vec![draft, entry("live", "Live", "2026-04-01")]);
    assert!(!html.contains("wip"), "drafts must not be listed: {html}");
    assert!(html.contains("/posts/live"), "{html}");
}

#[test]
fn home_with_empty_index_says_so() {
    let html = home_html(Vec::new());
    assert!(
        html.contains("nothing published yet"),
        "empty index needs a readable state, not a blank page: {html}"
    );
}

#[test]
fn home_greets_with_masthead_and_external_contacts() {
    let html = home_html(vec![entry("post", "A post", "2026-01-01")]);
    assert!(
        html.contains("hey, i’m chris"),
        "the masthead greets: {html}"
    );
    // External-only contacts sharing the about page's contact-link component;
    // the nav owns "about", so the masthead carries no in-app links.
    assert!(
        html.contains("mailto:hi@chris.dev"),
        "the email contact ships: {html}"
    );
    assert!(
        html.contains("github.com/chris"),
        "the github contact ships: {html}"
    );
    assert!(
        !html.contains(">the writing<") && !html.contains(">about me<"),
        "the masthead carries no in-app writing/about links: {html}"
    );
}

// The writing header carries the listed total (drafts excluded) and the feed
// link — "writing (N) · rss".
#[test]
fn home_writing_header_counts_the_listed_archive() {
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
        html.contains("writing (4)"),
        "the header count must be the real listed total, drafts excluded: {html}"
    );
    let rss = tag_containing(&html, ">rss<");
    assert!(
        rss.contains("href=\"/rss.xml\""),
        "the header links the feed: {html}"
    );
}

// Writing is the home: the complete list shows — no recent-posts teaser, no
// "read all" link.
#[test]
fn home_lists_every_listed_post() {
    let index: Vec<_> = (0..5)
        .map(|i| {
            entry(
                &format!("post-{i}"),
                &format!("Post {i}"),
                &format!("2026-01-{:02}", 20 - i),
            )
        })
        .collect();
    let html = home_html(index);
    for i in 0..5 {
        assert!(
            html.contains(&format!("/posts/post-{i}")),
            "post-{i} must be listed: {html}"
        );
    }
    assert!(
        !html.contains("read all"),
        "the home lists every post — no teaser link: {html}"
    );
}

// The reserved search slot ships its field focusable — the resting/focus
// affordances stay live — but with no filtering wired yet. It must not be
// disabled, or the focus states the design wants can never fire.
#[test]
fn home_renders_the_reserved_search_slot() {
    let html = home_html(vec![entry("post", "A post", "2026-01-01")]);
    let search = tag_containing(&html, "type=\"search\"");
    assert!(
        !search.contains("disabled"),
        "the search slot stays focusable — reserved, not dead: {html}"
    );
}

// The filter island owns the pill rail and the list, with the listed posts as
// its props: pills are the post-pill shape, deduped and sorted, linking the
// `?q=` filter contract rooted at the home listing.
#[test]
fn home_wraps_sorted_filter_pills_in_the_island() {
    let html = home_html(vec![
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
    let pill = tag_containing(&html, "href=\"/?q=rust\"");
    assert!(pill.contains("class=\"tag\""), "{html}");
    let hash = tag_containing(&html, "class=\"tag-hash\"");
    assert!(
        hash.contains("aria-hidden=\"true\"") && html.contains(">#</span>wasm"),
        "pills carry the post-pill hash glyph: {html}"
    );
    assert_eq!(
        html.matches("/?q=rust").count(),
        1,
        "pills dedupe across posts: {html}"
    );
    let rust = html.find("/?q=rust").unwrap();
    let wasm = html.find("/?q=wasm").unwrap();
    assert!(rust < wasm, "pills sort alphabetically: {html}");
}

#[test]
fn filter_pills_skip_draft_only_tags() {
    let mut draft = tagged("wip", "Not yet", "2026-05-01", &["secret", "rust"]);
    draft.draft = true;
    let html = home_html(vec![draft, tagged("live", "Live", "2026-04-01", &["rust"])]);
    assert!(!html.contains("secret"), "{html}");
    assert!(html.contains("/?q=rust"), "{html}");
}

// The empty-state line is island state shown only when a filter leaves no
// rows; the server render never ships it, so no-JS readers can't see it
// under the full list.
#[test]
fn home_ssr_omits_the_filter_empty_state() {
    let html = home_html(vec![tagged("a", "A", "2026-01-01", &["rust"])]);
    assert!(
        !html.contains("nothing here yet"),
        "the empty state must not ship in the server render: {html}"
    );
}

// A tagless index drops the whole rail column with its divider, not just
// the pills: the list runs full width instead of sitting beside dead space.
#[test]
fn home_without_tags_has_no_filter_row() {
    let html = home_html(vec![entry("plain", "Plain", "2026-01-01")]);
    assert!(!html.contains("post-tags"), "{html}");
    assert!(!html.contains("<aside"), "no tags, no rail column: {html}");
    assert!(
        !html.contains("nothing here yet"),
        "no pills means the empty state can never show: {html}"
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
fn home_without_index_context_still_renders() {
    // A missing IndexData context must degrade to the empty state, never panic.
    let html = ssr(|| (), || leptos::view! { <HomePage /> });
    assert!(html.contains("nothing published yet"), "{html}");
}
