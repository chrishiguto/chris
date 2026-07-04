use content_ast::{Document, Frontmatter, Node, PropValue, SCHEMA_VERSION};

fn sample_document() -> Document {
    Document {
        schema_version: SCHEMA_VERSION,
        frontmatter: Frontmatter {
            title: "Hello".into(),
            date: "2026-07-04".into(),
            tags: vec!["rust".into(), "leptos".into()],
            draft: false,
        },
        ast: vec![
            Node::Heading {
                level: 1,
                children: vec![Node::Text {
                    value: "Hello".into(),
                }],
            },
            Node::Paragraph {
                children: vec![
                    Node::Text {
                        value: "Some ".into(),
                    },
                    Node::Emphasis {
                        children: vec![Node::Text {
                            value: "prose".into(),
                        }],
                    },
                    Node::Text { value: ".".into() },
                ],
            },
            Node::CodeBlock {
                lang: Some("rust".into()),
                text: "fn main() {}".into(),
            },
            Node::Component {
                name: "Callout".into(),
                props: [
                    ("kind".to_string(), PropValue::String("warning".into())),
                    ("level".to_string(), PropValue::Number(3.0)),
                    ("dismissable".to_string(), PropValue::Bool(true)),
                ]
                .into_iter()
                .collect(),
                children: vec![Node::Paragraph {
                    children: vec![Node::Text {
                        value: "Careful.".into(),
                    }],
                }],
            },
        ],
    }
}

#[test]
fn serde_round_trip_preserves_document() {
    let doc = sample_document();
    let json = doc.to_json().unwrap();
    let back = Document::from_json(&json).unwrap();
    assert_eq!(doc, back);
}

#[test]
fn current_schema_version_is_stamped() {
    let json = sample_document().to_json().unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["schema_version"], serde_json::json!(SCHEMA_VERSION));
}

#[test]
fn old_schema_version_fails_detectably() {
    let mut value: serde_json::Value =
        serde_json::from_str(&sample_document().to_json().unwrap()).unwrap();
    value["schema_version"] = serde_json::json!(0);
    let err = Document::from_json(&value.to_string()).unwrap_err();
    match err {
        content_ast::AstError::SchemaVersionMismatch { found, expected } => {
            assert_eq!(found, 0);
            assert_eq!(expected, SCHEMA_VERSION);
        }
        other => panic!("expected SchemaVersionMismatch, got {other:?}"),
    }
}

#[test]
fn old_version_fixture_fails_detectably() {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/old-schema-version.json"
    ))
    .unwrap();
    let err = Document::from_json(&json).unwrap_err();
    assert!(matches!(
        err,
        content_ast::AstError::SchemaVersionMismatch { found: 0, .. }
    ));
}

#[test]
fn missing_schema_version_fails_detectably() {
    let mut value: serde_json::Value =
        serde_json::from_str(&sample_document().to_json().unwrap()).unwrap();
    value.as_object_mut().unwrap().remove("schema_version");
    let err = Document::from_json(&value.to_string()).unwrap_err();
    assert!(matches!(err, content_ast::AstError::Json(_)));
}

#[test]
fn prop_values_serialize_as_bare_scalars() {
    let json = serde_json::to_value([
        PropValue::String("warning".into()),
        PropValue::Number(3.0),
        PropValue::Bool(true),
    ])
    .unwrap();
    assert_eq!(json, serde_json::json!(["warning", 3.0, true]));
}

#[test]
fn index_entry_matches_kv_index_shape() {
    let entry = content_ast::IndexEntry::new(
        "hello",
        &Frontmatter {
            title: "Hello".into(),
            date: "2026-07-04".into(),
            tags: vec!["rust".into()],
            draft: true,
        },
    );
    let json = serde_json::to_value([&entry]).unwrap();
    assert_eq!(
        json,
        serde_json::json!([{
            "slug": "hello",
            "title": "Hello",
            "date": "2026-07-04",
            "tags": ["rust"],
            "draft": true,
        }])
    );
    let back: Vec<content_ast::IndexEntry> = serde_json::from_value(json).unwrap();
    assert_eq!(back, vec![entry]);
}

#[test]
fn index_entry_defaults_tags_and_draft() {
    let entry: content_ast::IndexEntry = serde_json::from_value(serde_json::json!({
        "slug": "hello",
        "title": "Hello",
        "date": "2026-07-04",
    }))
    .unwrap();
    assert!(entry.tags.is_empty());
    assert!(!entry.draft);
}
