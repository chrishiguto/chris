//! Snapshot tests for the AST renderer: every node type in the content IR must render.
#![cfg(feature = "ssr")]

use std::collections::BTreeMap;

use app::post::{PostData, PostPage};
use app::render::{render_document, render_nodes};
use common::{ssr, strip_markers};
use content::{Document, Frontmatter, ListItem, Node, PropValue, SCHEMA_VERSION};
use leptos::prelude::RenderHtml;

mod common;

fn text(value: &str) -> Node {
    Node::Text {
        value: value.into(),
    }
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
fn callout_dispatches_through_registry_with_markdown_children() {
    let html = html_of(vec![Node::Component {
        name: "Callout".into(),
        props: BTreeMap::from([("kind".to_string(), PropValue::String("warning".into()))]),
        children: vec![Node::Paragraph {
            children: vec![text("still readable")],
        }],
    }]);
    assert!(
        html.contains("class=\"callout callout-warning\""),
        "kind prop must reach the component: {html}"
    );
    assert!(
        html.contains("<p>still readable</p>"),
        "markdown children must render inside: {html}"
    );
    assert!(
        !html.contains("callout-title"),
        "omitted optional title must render nothing: {html}"
    );
}

#[test]
fn callout_renders_optional_title_when_given() {
    let html = html_of(vec![Node::Component {
        name: "Callout".into(),
        props: BTreeMap::from([
            ("kind".to_string(), PropValue::String("note".into())),
            ("title".to_string(), PropValue::String("Psst".into())),
        ]),
        children: vec![text("body")],
    }]);
    assert!(
        html.contains("<p class=\"callout-title\">Psst</p>"),
        "title missing: {html}"
    );
}

#[test]
fn counter_island_ssrs_with_initial_value() {
    let html = html_of(vec![Node::Component {
        name: "Counter".into(),
        props: BTreeMap::from([("initial".to_string(), PropValue::Number(5.0))]),
        children: vec![],
    }]);
    assert!(
        html.contains("<leptos-island"),
        "island wrapper missing — hydration would never attach: {html}"
    );
    assert!(html.contains(">5<"), "initial value not SSR'd: {html}");
}

#[test]
fn unknown_component_renders_loud_error() {
    let html = html_of(vec![Node::Component {
        name: "OrbitSimulatr".into(),
        props: BTreeMap::new(),
        children: vec![],
    }]);
    assert!(
        html.contains("class=\"component-error\""),
        "must fail visibly, not silently: {html}"
    );
    assert!(
        html.contains("data-component=\"OrbitSimulatr\""),
        "error must carry the component name: {html}"
    );
}

#[test]
fn dispatch_error_renders_loud_error() {
    // `kind` is required; this shape only reaches KV by skipping publish validation.
    let html = html_of(vec![Node::Component {
        name: "Callout".into(),
        props: BTreeMap::new(),
        children: vec![text("x")],
    }]);
    assert!(
        html.contains("class=\"component-error\""),
        "must fail visibly: {html}"
    );
    assert!(
        html.contains("missing required prop"),
        "error must say what is wrong: {html}"
    );
}

#[test]
fn manifest_contains_the_v1_vocabulary() {
    let manifest = registry::manifest();
    let callout = manifest.get("Callout").expect("Callout registered");
    assert!(callout.accepts_children);
    assert!(callout.prop("kind").is_some_and(|p| p.required));
    assert!(callout.prop("title").is_some_and(|p| !p.required));
    let counter = manifest.get("Counter").expect("Counter registered");
    assert!(!counter.accepts_children);
    assert!(counter.prop("initial").is_some_and(|p| p.required));
}

// The full read path minus KV transport.
#[test]
fn fixture_post_renders_end_to_end() {
    let source = include_str!("../../content/blog/components-demo/index.mdx");
    let doc = content::parse_validated(source, "test.mdx", &registry::manifest())
        .expect("fixture post must validate against the live manifest");
    let html = strip_markers(render_document(&doc).to_html());
    assert!(
        html.contains("class=\"callout callout-warning\""),
        "Callout missing: {html}"
    );
    assert!(
        html.contains("parsed recursively"),
        "Callout markdown children missing: {html}"
    );
    assert!(
        html.contains("<leptos-island"),
        "Counter island missing: {html}"
    );
    assert!(
        !html.contains("component-error"),
        "no component may fail dispatch: {html}"
    );
}

fn page_html(post: Option<Document>) -> String {
    use leptos::prelude::provide_context;
    ssr(
        move || provide_context(PostData(post)),
        || leptos::view! { <PostPage /> },
    )
}

// `PostData(None)` is what the worker provides on a KV miss.
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
            description: None,
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

// Drafts stay reachable by slug; only listings, feeds, and the cache treat them specially.
#[test]
fn post_page_renders_a_draft_document() {
    let html = page_html(Some(Document {
        schema_version: SCHEMA_VERSION,
        frontmatter: Frontmatter {
            title: "Not yet".into(),
            date: "2026-07-04".into(),
            description: None,
            tags: vec![],
            draft: true,
        },
        ast: vec![Node::Paragraph {
            children: vec![text("draft body")],
        }],
    }));
    assert!(html.contains("<article"), "missing article: {html}");
    assert!(html.contains("<p>draft body</p>"), "missing body: {html}");
    assert!(
        !html.contains("404"),
        "a draft is not a missing post: {html}"
    );
}

#[test]
fn render_document_wraps_body_in_article_with_header() {
    let doc = Document {
        schema_version: SCHEMA_VERSION,
        frontmatter: Frontmatter {
            title: "Hello, KV".into(),
            date: "2026-07-04".into(),
            description: None,
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

fn doc_with_tags(tags: Vec<String>) -> Document {
    Document {
        schema_version: SCHEMA_VERSION,
        frontmatter: Frontmatter {
            title: "Tagged".into(),
            date: "2026-07-04".into(),
            description: None,
            tags,
            draft: false,
        },
        ast: vec![],
    }
}

#[test]
fn post_header_renders_frontmatter_tags() {
    let doc = doc_with_tags(vec!["rust".into(), "wasm".into()]);
    let html = strip_markers(render_document(&doc).to_html());
    assert!(
        html.contains("<ul class=\"post-tags\">"),
        "tag list missing: {html}"
    );
    assert!(
        html.contains("<li class=\"tag\">rust</li>"),
        "tag missing: {html}"
    );
    assert!(
        html.contains("<li class=\"tag\">wasm</li>"),
        "tag missing: {html}"
    );
}

#[test]
fn post_header_omits_empty_tag_list() {
    let doc = doc_with_tags(vec![]);
    let html = strip_markers(render_document(&doc).to_html());
    assert!(
        !html.contains("post-tags"),
        "untagged post must not render an empty list: {html}"
    );
}
