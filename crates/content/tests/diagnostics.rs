use content::{parse, Diagnostic};

fn diags(source: &str) -> Vec<Diagnostic> {
    parse(source, "test.mdx").expect_err("expected diagnostics")
}

#[test]
fn missing_frontmatter_is_reported_at_line_one() {
    let all = diags("Just prose, no frontmatter.\n");
    assert!(all[0].message.contains("missing frontmatter"), "{all:?}");
    assert_eq!(all[0].line, Some(1));
}

#[test]
fn malformed_frontmatter_reports_yaml_error_with_location() {
    // `title` is fine; line 3 of the file (line 2 of the YAML) is garbage.
    let source = "---\ntitle: T\ndate: [unclosed\n---\n\nx\n";
    let all = diags(source);
    assert!(all[0].message.contains("malformed frontmatter"), "{all:?}");
    assert_eq!(all[0].line, Some(3), "{all:?}");
}

#[test]
fn frontmatter_missing_required_field_is_malformed() {
    let source = "---\ntitle: Only a title\n---\n\nx\n";
    let all = diags(source);
    assert!(all[0].message.contains("malformed frontmatter"), "{all:?}");
    assert!(all[0].message.contains("date"), "{all:?}");
}

#[test]
fn import_statement_is_rejected_with_location() {
    let source = "---\ntitle: T\ndate: 2026-01-01\n---\n\nimport Thing from './thing'\n\nx\n";
    let all = diags(source);
    assert!(all[0].message.contains("`import` statements"), "{all:?}");
    assert_eq!(all[0].line, Some(6), "{all:?}");
}

#[test]
fn export_statement_is_rejected_with_location() {
    let source = "---\ntitle: T\ndate: 2026-01-01\n---\n\nexport const x = 1\n";
    let all = diags(source);
    assert!(all[0].message.contains("`export` statements"), "{all:?}");
    assert_eq!(all[0].line, Some(6), "{all:?}");
}

#[test]
fn flow_expression_is_rejected_with_location() {
    let source = "---\ntitle: T\ndate: 2026-01-01\n---\n\n{1 + 1}\n";
    let all = diags(source);
    assert!(all[0].message.contains("JS expressions"), "{all:?}");
    assert_eq!(all[0].line, Some(6), "{all:?}");
}

#[test]
fn inline_expression_is_rejected_with_location() {
    let source = "---\ntitle: T\ndate: 2026-01-01\n---\n\nThe answer is {6 * 7}.\n";
    let all = diags(source);
    assert!(all[0].message.contains("JS expressions"), "{all:?}");
    assert_eq!(all[0].line, Some(6), "{all:?}");
}

#[test]
fn non_literal_prop_is_rejected_with_location() {
    let source = "---\ntitle: T\ndate: 2026-01-01\n---\n\n<Callout kind={theme.kind} />\n";
    let all = diags(source);
    assert!(all[0].message.contains("non-literal prop"), "{all:?}");
    assert!(all[0].message.contains("kind"), "{all:?}");
    assert_eq!(all[0].line, Some(6), "{all:?}");
}

#[test]
fn non_finite_number_props_are_rejected() {
    let source = "---\ntitle: T\ndate: 2026-01-01\n---\n\n<Demo count={inf} rate={NaN} />\n";
    let all = diags(source);
    assert_eq!(all.len(), 2, "{all:?}");
    assert!(
        all.iter().all(|d| d.message.contains("non-literal prop")),
        "{all:?}"
    );
}

#[test]
fn braced_string_prop_suggests_plain_quotes() {
    let source = "---\ntitle: T\ndate: 2026-01-01\n---\n\n<Callout kind={\"warning\"} />\n";
    let all = diags(source);
    assert!(all[0].message.contains("drop the braces"), "{all:?}");
}

#[test]
fn spread_attribute_is_rejected() {
    let source = "---\ntitle: T\ndate: 2026-01-01\n---\n\n<Callout {...props} />\n";
    let all = diags(source);
    assert!(all[0].message.contains("spread"), "{all:?}");
}

#[test]
fn multiple_problems_are_all_reported() {
    let source = "---\ntitle: T\ndate: 2026-01-01\n---\n\n{1 + 1}\n\n<Callout kind={x} />\n";
    let all = diags(source);
    assert_eq!(all.len(), 2, "{all:?}");
}

#[test]
fn parse_named_stamps_the_file_into_diagnostics() {
    let source = "no frontmatter";
    let all = parse(source, "content/blog/hello/index.mdx").unwrap_err();
    assert_eq!(all[0].file.as_deref(), Some("content/blog/hello/index.mdx"));
    let rendered = all[0].to_string();
    assert!(
        rendered.starts_with("content/blog/hello/index.mdx:1:1: "),
        "{rendered}"
    );
}

#[test]
fn duplicate_html_attributes_are_reported() {
    let all =
        diags("---\ntitle: T\ndate: 2026-01-01\n---\n\n<abbr title=\"a\" title=\"b\">x</abbr>\n");
    assert_eq!(all.len(), 1, "{all:#?}");
    assert!(
        all[0].message.contains("duplicate attribute `title`"),
        "{}",
        all[0]
    );
}
