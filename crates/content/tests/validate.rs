//! Manifest validation diagnostics: unknown components/props, type
//! mismatches, children rules — all with source locations.

use content::{parse_validated, Diagnostic};
use content::{ComponentSpec, Manifest, PropSpec, PropType};

fn manifest() -> Manifest {
    Manifest {
        components: vec![
            ComponentSpec {
                name: "Callout".into(),
                props: vec![
                    PropSpec {
                        name: "kind".into(),
                        ty: PropType::String,
                        required: true,
                    },
                    PropSpec {
                        name: "title".into(),
                        ty: PropType::String,
                        required: false,
                    },
                ],
                accepts_children: true,
            },
            ComponentSpec {
                name: "Counter".into(),
                props: vec![
                    PropSpec {
                        name: "initial".into(),
                        ty: PropType::Int,
                        required: true,
                    },
                    PropSpec {
                        name: "fancy".into(),
                        ty: PropType::Bool,
                        required: false,
                    },
                    PropSpec {
                        name: "ratio".into(),
                        ty: PropType::Float,
                        required: false,
                    },
                ],
                accepts_children: false,
            },
        ],
    }
}

fn post(body: &str) -> String {
    format!("---\ntitle: T\ndate: 2026-07-04\n---\n\n{body}\n")
}

fn diagnostics(body: &str) -> Vec<Diagnostic> {
    parse_validated(&post(body), "test.mdx", &manifest()).expect_err("expected diagnostics")
}

#[test]
fn valid_post_with_known_components_passes() {
    let doc = parse_validated(
        &post("<Callout kind=\"note\" title=\"Hi\">\n  some *markdown*\n</Callout>\n\n<Counter initial={3} fancy />"),
        "test.mdx",
        &manifest(),
    )
    .expect("valid post must parse");
    assert_eq!(doc.ast.len(), 2);
}

#[test]
fn unknown_component_suggests_closest_name() {
    let diags = diagnostics("<Calout kind=\"note\">x</Calout>");
    assert_eq!(diags.len(), 1);
    let diag = &diags[0];
    assert!(
        diag.message.contains("unknown component `<Calout>`"),
        "message: {}",
        diag.message
    );
    assert!(
        diag.message.contains("did you mean `<Callout>`"),
        "message: {}",
        diag.message
    );
    // Body starts after 4 frontmatter lines + 1 blank line.
    assert_eq!((diag.line, diag.column), (Some(6), Some(1)));
}

#[test]
fn unknown_component_without_close_match_gets_no_suggestion() {
    let diags = diagnostics("<Zorp />");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("unknown component `<Zorp>`"),
        "message: {}",
        diags[0].message
    );
    assert!(
        !diags[0].message.contains("did you mean"),
        "message: {}",
        diags[0].message
    );
}

#[test]
fn missing_required_prop_is_reported_with_type() {
    let diags = diagnostics("<Callout>oops</Callout>");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0]
            .message
            .contains("`<Callout>` is missing required prop `kind` (a string)"),
        "message: {}",
        diags[0].message
    );
    assert_eq!(diags[0].line, Some(6));
}

#[test]
fn quoted_number_for_int_prop_hints_braces() {
    let diags = diagnostics("<Counter initial=\"3\" />");
    assert_eq!(diags.len(), 1);
    let message = &diags[0].message;
    assert!(
        message.contains("prop `initial` on `<Counter>` expects an integer"),
        "message: {message}"
    );
    assert!(message.contains("initial={3}"), "message: {message}");
}

#[test]
fn number_for_string_prop_hints_quotes() {
    let diags = diagnostics("<Callout kind={3}>x</Callout>");
    assert_eq!(diags.len(), 1);
    let message = &diags[0].message;
    assert!(
        message.contains("prop `kind` on `<Callout>` expects a string"),
        "message: {message}"
    );
    assert!(message.contains("kind=\"3\""), "message: {message}");
}

#[test]
fn fractional_number_for_int_prop_is_a_mismatch() {
    let diags = diagnostics("<Counter initial={3.5} />");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0]
            .message
            .contains("prop `initial` on `<Counter>` expects an integer"),
        "message: {}",
        diags[0].message
    );
}

#[test]
fn arbitrary_string_for_bool_prop_gets_no_braces_hint() {
    let diags = diagnostics("<Counter initial={0} fancy=\"very\" />");
    assert_eq!(diags.len(), 1);
    let message = &diags[0].message;
    assert!(
        message.contains("prop `fancy` on `<Counter>` expects a boolean"),
        "message: {message}"
    );
    assert!(!message.contains("fancy={very}"), "message: {message}");
}

#[test]
fn quoted_exponent_for_float_prop_hints_braces() {
    let diags = diagnostics("<Counter initial={0} ratio=\"1e3\" />");
    assert_eq!(diags.len(), 1);
    let message = &diags[0].message;
    assert!(
        message.contains("prop `ratio` on `<Counter>` expects a number"),
        "message: {message}"
    );
    assert!(message.contains("ratio={1e3}"), "message: {message}");
}

#[test]
fn non_finite_string_for_float_prop_gets_no_braces_hint() {
    let diags = diagnostics("<Counter initial={0} ratio=\"inf\" />");
    assert_eq!(diags.len(), 1);
    let message = &diags[0].message;
    assert!(
        message.contains("prop `ratio` on `<Counter>` expects a number"),
        "message: {message}"
    );
    assert!(!message.contains("ratio={inf}"), "message: {message}");
}

#[test]
fn unknown_prop_suggests_closest_name() {
    let diags = diagnostics("<Counter initil={3} />");
    // The typo'd prop is unknown AND `initial` ends up missing: both surface.
    assert_eq!(diags.len(), 2);
    let unknown = diags
        .iter()
        .find(|d| d.message.contains("unknown prop"))
        .expect("unknown-prop diagnostic");
    assert!(
        unknown
            .message
            .contains("unknown prop `initil` on `<Counter>`"),
        "message: {}",
        unknown.message
    );
    assert!(
        unknown.message.contains("did you mean `initial`"),
        "message: {}",
        unknown.message
    );
    assert!(
        diags
            .iter()
            .any(|d| d.message.contains("missing required prop `initial`")),
        "missing-prop diagnostic expected"
    );
}

#[test]
fn children_on_childless_component_are_rejected() {
    let diags = diagnostics("<Counter initial={1}>kids</Counter>");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0]
            .message
            .contains("`<Counter>` does not accept children"),
        "message: {}",
        diags[0].message
    );
}

#[test]
fn nested_components_are_validated_too() {
    let diags = diagnostics("<Callout kind=\"note\">\n  <Zorp />\n</Callout>");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("unknown component `<Zorp>`"),
        "message: {}",
        diags[0].message
    );
    assert_eq!(diags[0].line, Some(7));
}

#[test]
fn parse_validated_named_stamps_the_file() {
    let diags = parse_validated(&post("<Zorp />"), "content/blog/x/index.mdx", &manifest())
        .expect_err("expected diagnostics");
    assert!(diags
        .iter()
        .all(|d| d.file.as_deref() == Some("content/blog/x/index.mdx")));
}

#[test]
fn duplicate_props_are_reported() {
    let diags = diagnostics("<Counter initial={1} initial={2} />");
    assert_eq!(diags.len(), 1, "{diags:#?}");
    assert!(
        diags[0].message.contains("duplicate prop `initial`"),
        "{}",
        diags[0]
    );
}

#[test]
fn rejected_required_prop_does_not_cascade_into_missing() {
    let diags = diagnostics("<Counter initial={x} />");
    assert_eq!(diags.len(), 1, "{diags:#?}");
    assert!(
        diags[0].message.contains("non-literal prop"),
        "{}",
        diags[0]
    );
}

#[test]
fn validated_parse_rejects_a_malformed_date() {
    let source = "---\ntitle: T\ndate: someday\n---\n\nx\n";
    let diags = parse_validated(source, "test.mdx", &manifest()).expect_err("expected diagnostics");
    assert_eq!(diags.len(), 1, "{diags:#?}");
    assert!(diags[0].message.contains("YYYY-MM-DD"), "{}", diags[0]);
    assert_eq!(diags[0].line, Some(3), "date is declared on line 3");
}

#[test]
fn validated_parse_rejects_a_non_slug_tag() {
    let source = "---\ntitle: T\ndate: 2026-01-01\ntags: [ok, \"Not A Slug\"]\n---\n\nx\n";
    let diags = parse_validated(source, "test.mdx", &manifest()).expect_err("expected diagnostics");
    assert_eq!(diags.len(), 1, "{diags:#?}");
    assert!(diags[0].message.contains("Not A Slug"), "{}", diags[0]);
    assert_eq!(diags[0].line, Some(4), "tags are declared on line 4");
}
