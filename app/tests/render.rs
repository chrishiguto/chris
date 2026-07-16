//! Snapshot tests for the AST renderer: every node type in the content IR must render.
#![cfg(feature = "ssr")]

use std::collections::BTreeMap;

use app::post::{PostData, PostPage};
use app::render::{render_document, render_nodes};
use common::{ssr, strip_markers, tag_containing};
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
fn code_block_renders_chrome_bar_with_language_label() {
    let html = html_of(vec![Node::CodeBlock {
        lang: Some("rust".into()),
        text: "fn main() {}".into(),
    }]);
    for needle in [
        "<div class=\"code-block\">",
        "<div class=\"code-bar\">",
        "<span class=\"code-lang\">rust</span>",
        "<pre><code class=\"language-rust\">fn main() {}</code></pre>",
    ] {
        assert!(html.contains(needle), "missing `{needle}`: {html}");
    }

    let html = html_of(vec![Node::CodeBlock {
        lang: None,
        text: "plain".into(),
    }]);
    assert!(
        html.contains("<span class=\"code-lang\">code</span>"),
        "bare fences must fall back to the `code` label: {html}"
    );
    assert!(
        html.contains("<pre><code>plain</code></pre>"),
        "bare fences must not carry a language class: {html}"
    );
}

// The copy button hydrates as an island with the source as its prop: the
// component owns its data instead of reading it back out of the DOM.
#[test]
fn copy_button_receives_the_source_as_its_prop() {
    let html = html_of(vec![Node::CodeBlock {
        lang: Some("rust".into()),
        text: "fn main() {}".into(),
    }]);
    assert!(
        html.contains("<leptos-island"),
        "the copy button must hydrate as an island: {html}"
    );
    assert!(
        html.contains("class=\"code-copy\"") && html.contains(">copy<"),
        "the button must SSR inert with its resting label: {html}"
    );
    let island = tag_containing(&html, "data-props");
    assert!(
        island.contains("fn main() {}"),
        "the island's props must carry the source: {html}"
    );
    assert!(
        html.contains("<code class=\"language-rust\">fn main() {}</code>"),
        "the source must still render inside the pre: {html}"
    );
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
        html.contains("<span class=\"callout-label\">warning</span>"),
        "kind label row missing: {html}"
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
    let label = html
        .find("callout-label\">note</span>")
        .expect("label row missing");
    let title = html.find("callout-title").expect("title missing");
    let body = html.find("callout-body").expect("body missing");
    assert!(
        label < title && title < body,
        "title must sit between the label row and the body: {html}"
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
    let source = include_str!("../../content/blog/ci-code-path/index.mdx");
    let doc = content::parse_validated(source, "test.mdx", &registry::manifest())
        .expect("fixture post must validate against the live manifest");
    let html = strip_markers(render_document(&doc).to_html());
    assert!(
        html.contains("class=\"callout callout-warning\""),
        "Callout missing: {html}"
    );
    assert!(
        html.contains("<code>post:ci-code-path</code>"),
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
        move || provide_context(PostData { post }),
        || leptos::view! { <PostPage /> },
    )
}

// A missing document is what the worker provides on a KV miss; the page
// renders a plain 404. (`PostData` no longer carries the URL's slug, so
// reflecting it is unrepresentable.)
#[test]
fn post_page_without_post_renders_404_content() {
    let html = page_html(None);
    assert!(html.contains("404"), "missing 404 heading: {html}");
    assert!(html.contains("href=\"/\""), "missing link home: {html}");
    assert!(
        html.contains("no such file or directory"),
        "the 404 speaks in the site's shell voice: {html}"
    );
    assert!(
        html.contains("no post lives at this address."),
        "the post miss keeps its own message: {html}"
    );
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
    assert!(
        tag_containing(&html, "article").contains("page-enter"),
        "the article mounts under the page-enter stagger: {html}"
    );
    assert!(html.contains("<h1>Hello, KV</h1>"), "title missing: {html}");
    assert!(
        html.contains("jul 04, 2026"),
        "formatted date missing: {html}"
    );
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

// Tag pills sit at the article bottom and land on the pre-filtered
// listing via the `?q=` filter query.
#[test]
fn post_tags_render_at_the_bottom_linking_the_filtered_listing() {
    let doc = doc_with_tags(vec!["rust".into(), "wasm".into()]);
    let html = strip_markers(render_document(&doc).to_html());
    assert!(
        html.contains("<ul class=\"post-tags\">"),
        "tag list missing: {html}"
    );
    for tag in ["rust", "wasm"] {
        assert!(
            html.contains(&format!("<a href=\"/posts?q={tag}\" class=\"tag\">")),
            "`{tag}` pill must link to the filtered listing: {html}"
        );
    }
    let body = html.find("post-body").expect("post body missing");
    let tags = html.find("post-tags").expect("tag list missing");
    assert!(tags > body, "tags must follow the article body: {html}");
    assert!(
        !html[..body].contains("post-tags"),
        "the header must not carry tags anymore: {html}"
    );
}

#[test]
fn post_omits_empty_tag_list() {
    let doc = doc_with_tags(vec![]);
    let html = strip_markers(render_document(&doc).to_html());
    assert!(
        !html.contains("post-tags"),
        "untagged post must not render an empty list: {html}"
    );
}

// The back control opens the article as an island (`history.back` needs
// JS), SSR'd as a plain listing link so it stays a way back without
// hydration. It never names the post's own URL — the renderer stays
// decoupled from URL knowledge.
#[test]
fn post_opens_with_a_back_link_island_to_the_listing() {
    let doc = doc_with_tags(vec![]);
    let html = strip_markers(render_document(&doc).to_html());
    let link = tag_containing(&html, "href=\"/posts\"");
    assert!(
        link.starts_with("<a"),
        "the no-JS fallback must link the listing: {html}"
    );
    let back = html.find("href=\"/posts\"").unwrap();
    assert!(
        html[..back].contains("<leptos-island"),
        "the back link must hydrate as an island: {html}"
    );
    assert!(
        back < html.find("<header").expect("header missing"),
        "the back link must sit above the header: {html}"
    );
    let arrow = tag_containing(&html, "←");
    assert!(
        arrow.contains("aria-hidden=\"true\""),
        "the arrow is decoration, hidden from readers: {html}"
    );
}

// The header meta row is mono chrome (`.post-meta`): formatted date, ink-3
// separator span, and a read time computed live from the AST the page holds.
#[test]
fn post_header_renders_formatted_date_and_read_time() {
    let doc = doc_with_tags(vec![]);
    let html = strip_markers(render_document(&doc).to_html());
    assert!(
        html.contains("<p class=\"post-meta\"><span>jul 04, 2026</span>")
            && html.contains("<span>1 min</span>"),
        "meta row must read `jul 04, 2026 · 1 min`: {html}"
    );
    let sep = tag_containing(&html, "·");
    assert!(
        sep.contains("aria-hidden=\"true\""),
        "the separator hides from readers: {html}"
    );
    let sep_at = html.find("·").unwrap();
    assert!(
        html.find("jul 04, 2026").unwrap() < sep_at && sep_at < html.find("1 min").unwrap(),
        "the meta row must read `date · minutes`: {html}"
    );
}

// One real post exercising every node type and Callout kind — if it renders,
// the whole vocabulary reached markup. Looks are the kitchen-sink read's job.
#[test]
fn kitchen_sink_fixture_exercises_every_node_type() {
    let source = include_str!("../../content/blog/kitchen-sink/index.mdx");
    let doc = content::parse_validated(source, "test.mdx", &registry::manifest())
        .expect("kitchen-sink post must validate against the live manifest");
    let html = strip_markers(render_document(&doc).to_html());
    for needle in [
        "<h2",
        "<h3",
        "<h4",
        "<h5",
        "<h6",
        "<em>",
        "<strong>",
        "<code",
        "<pre",
        "<a href",
        "<img",
        "<ol start",
        "<ul>",
        "<blockquote>",
        "<hr",
        "<br",
        "<kbd>",
        "class=\"post-tags\"",
        "callout callout-note",
        "callout callout-tip",
        "callout callout-warning",
        "callout callout-danger",
        "callout-label\">note</span>",
        "callout-label\">tip</span>",
        "callout-label\">warning</span>",
        "callout-label\">danger</span>",
        "<span class=\"code-lang\">rust</span>",
        "<span class=\"code-lang\">code</span>",
        "class=\"code-copy\"",
        "<leptos-island",
    ] {
        assert!(html.contains(needle), "kitchen sink missing {needle}");
    }
    assert!(
        !html.contains("component-error"),
        "no component may fail dispatch: {html}"
    );
}
