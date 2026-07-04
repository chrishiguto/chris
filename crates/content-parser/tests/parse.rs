use content_ast::{Node, PropValue, SCHEMA_VERSION};
use content_parser::parse;

const MINIMAL: &str = "---\ntitle: Hello\ndate: 2026-07-04\n---\n\nJust prose.\n";

#[test]
fn parses_frontmatter_and_prose() {
    let doc = parse(MINIMAL).unwrap();
    assert_eq!(doc.schema_version, SCHEMA_VERSION);
    assert_eq!(doc.frontmatter.title, "Hello");
    assert_eq!(doc.frontmatter.date, "2026-07-04");
    assert!(doc.frontmatter.tags.is_empty());
    assert!(!doc.frontmatter.draft);
    assert_eq!(
        doc.ast,
        vec![Node::Paragraph {
            children: vec![Node::Text {
                value: "Just prose.".into()
            }]
        }]
    );
}

#[test]
fn parses_full_frontmatter() {
    let source = "---\ntitle: T\ndate: 2026-01-01\ntags: [rust, leptos]\ndraft: true\n---\n\nx\n";
    let doc = parse(source).unwrap();
    assert_eq!(doc.frontmatter.tags, vec!["rust", "leptos"]);
    assert!(doc.frontmatter.draft);
}

#[test]
fn parses_component_with_scalar_props_and_markdown_children() {
    let source = "---\ntitle: T\ndate: 2026-01-01\n---\n\n<Callout kind=\"warning\" level={3} dismissable={true}>\n  Some *emphasis* inside.\n</Callout>\n";
    let doc = parse(source).unwrap();
    let Node::Component {
        name,
        props,
        children,
    } = &doc.ast[0]
    else {
        panic!("expected component, got {:?}", doc.ast[0]);
    };
    assert_eq!(name, "Callout");
    assert_eq!(props["kind"], PropValue::String("warning".into()));
    assert_eq!(props["level"], PropValue::Number(3.0));
    assert_eq!(props["dismissable"], PropValue::Bool(true));
    assert_eq!(
        children[0],
        Node::Paragraph {
            children: vec![
                Node::Text {
                    value: "Some ".into()
                },
                Node::Emphasis {
                    children: vec![Node::Text {
                        value: "emphasis".into()
                    }]
                },
                Node::Text {
                    value: " inside.".into()
                },
            ]
        }
    );
}

#[test]
fn bare_prop_is_boolean_true() {
    let source = "---\ntitle: T\ndate: 2026-01-01\n---\n\n<Demo autoplay />\n";
    let doc = parse(source).unwrap();
    let Node::Component { props, .. } = &doc.ast[0] else {
        panic!("expected component");
    };
    assert_eq!(props["autoplay"], PropValue::Bool(true));
}

#[test]
fn lowercase_tags_pass_through_as_html() {
    let source =
        "---\ntitle: T\ndate: 2026-01-01\n---\n\nAn <abbr title=\"HyperText\">HT</abbr> here.\n";
    let doc = parse(source).unwrap();
    let Node::Paragraph { children } = &doc.ast[0] else {
        panic!("expected paragraph");
    };
    let Node::Html {
        tag,
        attrs,
        children: inner,
    } = &children[1]
    else {
        panic!("expected html node, got {:?}", children[1]);
    };
    assert_eq!(tag, "abbr");
    assert_eq!(attrs["title"], "HyperText");
    assert_eq!(inner, &vec![Node::Text { value: "HT".into() }]);
}
