//! The `/posts/{slug}` page, rendered through [`crate::render`].

use content::Document;
use leptos::prelude::*;
use leptos_meta::{Meta, Title};

use crate::components::NotFound;
use crate::render::render_document;

/// Per-request post from the site worker; `None` means no document lives at
/// the requested slug and renders as a plain 404. The URL's slug never
/// reaches the page — nothing here holds it, so it can't reflect.
#[derive(Clone)]
pub struct PostData {
    pub post: Option<Document>,
}

#[component]
pub fn PostPage() -> impl IntoView {
    match use_context::<PostData>() {
        Some(PostData { post: Some(doc) }) => post_article(doc).into_any(),
        _ => post_not_found().into_any(),
    }
}

fn post_article(doc: Document) -> impl IntoView {
    view! {
        <Title text=doc.frontmatter.title.clone() />
        <Meta property="article:published_time" content=doc.frontmatter.date.clone() />
        {render_document(&doc)}
    }
}

fn post_not_found() -> impl IntoView {
    view! {
        <Title text="post not found" />
        <NotFound message="no post lives at this address." />
    }
}
