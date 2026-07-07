//! Post page: the `/posts/{slug}` route over a [`Document`] the site worker
//! read from KV, rendered through [`crate::render`].

use content::Document;
use leptos::prelude::*;
use leptos_meta::{Meta, Title};

use crate::components::NotFound;
use crate::render::render_document;

/// Per-request payload provided by the site worker via context.
/// `None` means the slug had no KV entry — rendered as a plain 404
/// (never a rebuild trigger).
#[derive(Clone)]
pub struct PostData(pub Option<Document>);

#[component]
pub fn PostPage() -> impl IntoView {
    match use_context::<PostData>().and_then(|data| data.0) {
        Some(doc) => post_article(doc).into_any(),
        None => post_not_found().into_any(),
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
