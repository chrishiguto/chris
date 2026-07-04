//! Listing pages (Slice 5): `/posts` and `/` render from the KV `index`
//! provided by the site worker via context; drafts are filtered at render
//! time (PRD "KV schema"). Run with `cargo test -p app --features ssr`.
#![cfg(feature = "ssr")]

use app::listing::{HomePage, IndexData, PostsPage, RECENT_POSTS};
use content_ast::IndexEntry;
use leptos::prelude::RenderHtml;

fn entry(slug: &str, title: &str, date: &str) -> IndexEntry {
    IndexEntry {
        slug: slug.into(),
        title: title.into(),
        date: date.into(),
        tags: vec![],
        draft: false,
    }
}

fn strip_markers(html: String) -> String {
    html.replace("<!>", "")
}

// Renders a listing page the way the worker does: contexts on a reactive
// owner, then SSR'd to a string.
fn page_html(view: impl FnOnce() -> leptos::prelude::AnyView, index: Vec<IndexEntry>) -> String {
    use leptos::prelude::{provide_context, Owner};

    let owner = Owner::new();
    owner.set();
    leptos_meta::provide_meta_context();
    provide_context(IndexData(index));
    strip_markers(view().to_html())
}

fn posts_html(index: Vec<IndexEntry>) -> String {
    page_html(
        || leptos::prelude::IntoAny::into_any(leptos::view! { <PostsPage /> }),
        index,
    )
}

fn home_html(index: Vec<IndexEntry>) -> String {
    page_html(
        || leptos::prelude::IntoAny::into_any(leptos::view! { <HomePage /> }),
        index,
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

#[test]
fn home_page_without_index_context_still_renders() {
    // generate_route_list runs the App tree outside a request; no context.
    let owner = leptos::prelude::Owner::new();
    owner.set();
    leptos_meta::provide_meta_context();
    let html = strip_markers(leptos::view! { <HomePage /> }.to_html());
    assert!(html.contains("Nothing published yet"), "{html}");
}
