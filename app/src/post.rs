//! The `/posts/{slug}` page, rendered through [`crate::render`].

use content::Document;
use leptos::prelude::*;
use leptos_meta::{Meta, Title};

use crate::components::NotFound;
use crate::render::render_document;

/// Per-request post from the site worker: the requested slug and its
/// document; a `None` document means no such slug and renders as a plain 404.
#[derive(Clone)]
pub struct PostData {
    pub slug: String,
    pub post: Option<Document>,
}

#[component]
pub fn PostPage() -> impl IntoView {
    match use_context::<PostData>() {
        Some(PostData {
            slug,
            post: Some(doc),
        }) => post_article(doc, &slug).into_any(),
        _ => post_not_found().into_any(),
    }
}

fn post_article(doc: Document, slug: &str) -> impl IntoView {
    view! {
        <Title text=doc.frontmatter.title.clone() />
        <Meta property="article:published_time" content=doc.frontmatter.date.clone() />
        {render_document(&doc, slug)}
    }
}

fn post_not_found() -> impl IntoView {
    view! {
        <Title text="post not found" />
        <NotFound message="no post lives at this address." />
    }
}
