#![cfg(feature = "ssr")]

use app::{home::HomePage, writing::IndexData};
use common::{ssr, tag_containing};
use content::{Frontmatter, IndexEntry};
use leptos::prelude::provide_context;
use leptos_router::location::RequestUrl;

mod common;

fn entry(slug: &str, title: &str, date: &str) -> IndexEntry {
    IndexEntry::new(
        slug,
        &Frontmatter {
            title: title.into(),
            date: date.into(),
            description: Some(format!("description for {title}")),
            tags: vec![],
            draft: false,
        },
    )
}

fn home_html(index: Vec<IndexEntry>) -> String {
    ssr(
        move || provide_context(IndexData(index)),
        || leptos::view! { <HomePage /> },
    )
}

fn home_app_html(index: Vec<IndexEntry>) -> String {
    ssr(
        move || {
            provide_context(RequestUrl::new("/"));
            provide_context(IndexData(index));
        },
        || leptos::view! { <app::app::App /> },
    )
}

#[test]
fn home_renders_the_index_sections_in_order() {
    let html = home_app_html(vec![entry("one", "one post", "2026-07-04")]);
    let name = html.find("christiano higuto").unwrap();
    let intro = html.find("home-intro-copy").unwrap();
    let work = html.find(">work</span>").unwrap();
    let writing = html.find(">writing</span>").unwrap();
    let now = html.find(">now</span>").unwrap();
    let touched = html.find("last touched 2 september 2026").unwrap();
    let credit = html.find("hidden text borrows from").unwrap();
    let footer = html.find("<footer").unwrap();
    assert!(
        name < intro
            && intro < work
            && work < writing
            && writing < now
            && now < touched
            && touched < credit
            && credit < footer,
        "{html}"
    );
    assert!(html.contains("mailto:chrisshiguto@gmail.com"), "{html}");
    assert!(html.contains("github.com/chrishiguto/chris"), "{html}");
    for lineage in ["igorbedesqui.com", "ped.ro", "lfe.org"] {
        assert!(
            html.contains(lineage),
            "home missing `{lineage}` credit: {html}"
        );
    }
    assert!(!html.contains('—'), "home copy contains an em dash: {html}");
    assert!(
        html.contains("4 july") && !html.contains("2026-07-04"),
        "visible post dates must read as words: {html}"
    );
    assert!(
        !html.contains("post-row-desc"),
        "home has no post descriptions: {html}"
    );
    assert!(
        !html.contains("card") && !html.contains("chip"),
        "home has no card/chip markup: {html}"
    );
}

#[test]
fn home_lists_only_the_latest_four_published_titles() {
    let mut draft = entry("draft", "draft post", "2026-08-01");
    draft.draft = true;
    let mut index = vec![draft];
    index.extend((0..5).map(|i| entry(&format!("post-{i}"), &format!("post {i}"), "2026-07-04")));
    let html = home_html(index);
    for i in 0..4 {
        assert!(html.contains(&format!("/posts/post-{i}")), "{html}");
    }
    assert!(
        !html.contains("/posts/post-4") && !html.contains("draft post"),
        "{html}"
    );
    assert!(html.contains("all writing (5)"), "{html}");
}

#[test]
fn home_ships_accessible_ghost_rows_fold_and_prose_marks() {
    let html = home_html(vec![entry("one", "one post", "2026-07-04")]);
    assert!(html.matches("class=\"sr-only\"").count() >= 4, "{html}");
    assert!(html.contains("ghost-word-outer"), "{html}");
    let work_row = tag_containing(&html, "class=\"hover-date-row\"");
    assert!(work_row.contains("tabindex=\"0\""), "{html}");
    assert!(
        html.contains("hover-date-row-date-current\">since 2022"),
        "{html}"
    );
    assert!(
        html.contains("class=\"home-fold-button\"") && html.contains("aria-expanded=\"false\""),
        "{html}"
    );
    assert!(
        html.contains("class=\"home-fold-content\"") && html.contains("web developer"),
        "{html}"
    );
    assert!(
        html.contains("class=\"pencil\"") && html.contains("class=\"honest-edit\""),
        "{html}"
    );
    assert!(
        !html.contains("leptos-island"),
        "the home adds no island: {html}"
    );
}
