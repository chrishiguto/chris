//! Snapshot tests for the AST renderer: every prose node type in
//! `content-ast` must render, plus the Slice-3 component placeholder.
//! Run with `cargo test -p app --features ssr`.
#![cfg(feature = "ssr")]

use std::collections::BTreeMap;

use app::post::{render_document, render_nodes, PostData, PostPage};
use content_ast::{Document, Frontmatter, ListItem, Node, PropValue, SCHEMA_VERSION};
use leptos::prelude::RenderHtml;

fn text(value: &str) -> Node {
    Node::Text {
        value: value.into(),
    }
}

// `AnyView` emits `<!>` hydration-marker comments in SSR output; they are
// invisible to browsers, so snapshots compare with them stripped.
fn strip_markers(html: String) -> String {
    html.replace("<!>", "")
}

fn html_of(nodes: Vec<Node>) -> String {
    strip_markers(render_nodes(&nodes).to_html())
}

#[test]
fn headings_render_at_each_level() {
    for (level, tag) in [
        (1, "h1"),
        (2, "h2"),
        (3, "h3"),
        (4, "h4"),
        (5, "h5"),
        (6, "h6"),
    ] {
        let html = html_of(vec![Node::Heading {
            level,
            children: vec![text("Title")],
        }]);
        assert_eq!(html, format!("<{tag}>Title</{tag}>"));
    }
}

#[test]
fn paragraph_renders_inline_children() {
    let html = html_of(vec![Node::Paragraph {
        children: vec![
            text("plain "),
            Node::Emphasis {
                children: vec![text("em")],
            },
            text(" and "),
            Node::Strong {
                children: vec![text("bold")],
            },
        ],
    }]);
    assert_eq!(html, "<p>plain <em>em</em> and <strong>bold</strong></p>");
}

#[test]
fn text_is_escaped() {
    let html = html_of(vec![Node::Paragraph {
        children: vec![text("<script>alert(1)</script>")],
    }]);
    assert!(!html.contains("<script>"), "raw html leaked: {html}");
    assert!(
        html.contains("&lt;script&gt;"),
        "expected escaped text: {html}"
    );
}

#[test]
fn inline_code_renders() {
    let html = html_of(vec![Node::InlineCode {
        value: "let x = 1;".into(),
    }]);
    assert_eq!(html, "<code>let x = 1;</code>");
}

#[test]
fn link_renders_href_and_optional_title() {
    let html = html_of(vec![Node::Link {
        url: "https://example.com".into(),
        title: Some("Example".into()),
        children: vec![text("go")],
    }]);
    assert_eq!(
        html,
        "<a href=\"https://example.com\" title=\"Example\">go</a>"
    );

    let html = html_of(vec![Node::Link {
        url: "/about".into(),
        title: None,
        children: vec![text("about")],
    }]);
    assert_eq!(html, "<a href=\"/about\">about</a>");
}

#[test]
fn image_renders_src_alt_and_optional_title() {
    let html = html_of(vec![Node::Image {
        url: "/img/orbit.png".into(),
        alt: "an orbit".into(),
        title: Some("Orbit".into()),
    }]);
    assert_eq!(
        html,
        "<img src=\"/img/orbit.png\" alt=\"an orbit\" title=\"Orbit\">"
    );
}

#[test]
fn unordered_list_renders_items() {
    let html = html_of(vec![Node::List {
        ordered: false,
        start: None,
        items: vec![
            ListItem {
                children: vec![text("one")],
            },
            ListItem {
                children: vec![text("two")],
            },
        ],
    }]);
    assert_eq!(html, "<ul><li>one</li><li>two</li></ul>");
}

#[test]
fn ordered_list_renders_start() {
    let html = html_of(vec![Node::List {
        ordered: true,
        start: Some(3),
        items: vec![ListItem {
            children: vec![text("three")],
        }],
    }]);
    assert_eq!(html, "<ol start=\"3\"><li>three</li></ol>");
}

#[test]
fn code_block_renders_language_class() {
    let html = html_of(vec![Node::CodeBlock {
        lang: Some("rust".into()),
        text: "fn main() {}".into(),
    }]);
    assert_eq!(
        html,
        "<pre><code class=\"language-rust\">fn main() {}</code></pre>"
    );

    let html = html_of(vec![Node::CodeBlock {
        lang: None,
        text: "plain".into(),
    }]);
    assert_eq!(html, "<pre><code>plain</code></pre>");
}

#[test]
fn code_block_escapes_source() {
    let html = html_of(vec![Node::CodeBlock {
        lang: Some("html".into()),
        text: "<b>&</b>".into(),
    }]);
    assert!(
        html.contains("&lt;b&gt;&amp;&lt;/b&gt;"),
        "unescaped code: {html}"
    );
}

#[test]
fn blockquote_renders_block_children() {
    let html = html_of(vec![Node::Blockquote {
        children: vec![Node::Paragraph {
            children: vec![text("quoted")],
        }],
    }]);
    assert_eq!(html, "<blockquote><p>quoted</p></blockquote>");
}

#[test]
fn thematic_break_and_hard_break_render() {
    assert_eq!(html_of(vec![Node::ThematicBreak]), "<hr>");
    assert_eq!(html_of(vec![Node::Break]), "<br>");
}

#[test]
fn html_passthrough_renders_tag_attrs_and_markdown_children() {
    let html = html_of(vec![Node::Html {
        tag: "abbr".into(),
        attrs: BTreeMap::from([("title".to_string(), "HyperText".to_string())]),
        children: vec![Node::Strong {
            children: vec![text("HT")],
        }],
    }]);
    assert_eq!(html, "<abbr title=\"HyperText\"><strong>HT</strong></abbr>");
}

#[test]
fn component_renders_placeholder_with_name_and_children() {
    let html = html_of(vec![Node::Component {
        name: "Callout".into(),
        props: BTreeMap::from([("kind".to_string(), PropValue::String("warning".into()))]),
        children: vec![Node::Paragraph {
            children: vec![text("still readable")],
        }],
    }]);
    assert!(
        html.contains("data-component=\"Callout\""),
        "placeholder must carry the component name: {html}"
    );
    assert!(html.contains("Callout"), "name must be visible: {html}");
    assert!(
        html.contains("<p>still readable</p>"),
        "children must not be lost: {html}"
    );
}

// Renders `PostPage` the way the worker does: contexts (meta + `PostData`)
// provided on a reactive owner, then SSR'd to a string.
fn page_html(post: Option<Document>) -> String {
    use leptos::prelude::{provide_context, Owner};

    let owner = Owner::new();
    owner.set();
    leptos_meta::provide_meta_context();
    provide_context(PostData(post));
    strip_markers(leptos::view! { <PostPage /> }.to_html())
}

// The worker provides `PostData(None)` on a KV miss; the page must still be
// a proper error page (heading + way home), never a blank body.
#[test]
fn post_page_without_post_renders_404_content() {
    let html = page_html(None);
    assert!(html.contains("404"), "missing 404 heading: {html}");
    assert!(html.contains("href=\"/\""), "missing link home: {html}");
}

#[test]
fn post_page_with_post_renders_article() {
    let html = page_html(Some(Document {
        schema_version: SCHEMA_VERSION,
        frontmatter: Frontmatter {
            title: "Hello, KV".into(),
            date: "2026-07-04".into(),
            tags: vec![],
            draft: false,
        },
        ast: vec![Node::Paragraph {
            children: vec![text("body text")],
        }],
    }));
    assert!(html.contains("<article"), "missing article: {html}");
    assert!(html.contains("<p>body text</p>"), "missing body: {html}");
}

#[test]
fn render_document_wraps_body_in_article_with_header() {
    let doc = Document {
        schema_version: SCHEMA_VERSION,
        frontmatter: Frontmatter {
            title: "Hello, KV".into(),
            date: "2026-07-04".into(),
            tags: vec!["rust".into()],
            draft: false,
        },
        ast: vec![Node::Paragraph {
            children: vec![text("body text")],
        }],
    };
    let html = strip_markers(render_document(&doc).to_html());
    assert!(
        html.starts_with("<article"),
        "expected article root: {html}"
    );
    assert!(html.contains("<h1>Hello, KV</h1>"), "title missing: {html}");
    assert!(html.contains("2026-07-04"), "date missing: {html}");
    assert!(html.contains("<p>body text</p>"), "body missing: {html}");
}
