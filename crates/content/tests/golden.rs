//! Golden tests over the fixture corpus. The corpus doubles as the authoring
//! format spec: every construct of the subset appears in `fixtures/valid`,
//! every rejected construct in `fixtures/invalid`.
//!
//! Regenerate goldens with `UPDATE_GOLDEN=1 cargo test -p content --features parse`.

use std::fs;
use std::path::{Path, PathBuf};

fn fixtures(dir: &str) -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
    let mut paths: Vec<_> = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("mdx"))
        .collect();
    paths.sort();
    paths
}

#[test]
fn valid_fixtures_match_golden_ast_json() {
    let paths = fixtures("fixtures/valid");
    assert!(paths.len() >= 11, "fixture corpus shrank: {paths:?}");
    for path in paths {
        let source = fs::read_to_string(&path).unwrap();
        let doc = content::parse_named(&source, &path.display().to_string())
            .unwrap_or_else(|diags| panic!("{} failed to parse: {diags:#?}", path.display()));
        let actual = serde_json::to_value(&doc).unwrap();

        let golden_path = path.with_extension("json");
        if std::env::var_os("UPDATE_GOLDEN").is_some() {
            let mut pretty = serde_json::to_string_pretty(&actual).unwrap();
            pretty.push('\n');
            fs::write(&golden_path, pretty).unwrap();
        }
        let golden_source = fs::read_to_string(&golden_path).unwrap_or_else(|_| {
            panic!(
                "missing golden {}; run UPDATE_GOLDEN=1 cargo test -p content --features parse",
                golden_path.display()
            )
        });
        let golden: serde_json::Value = serde_json::from_str(&golden_source).unwrap();
        assert_eq!(actual, golden, "AST mismatch for {}", path.display());
    }
}

#[test]
fn golden_json_round_trips_through_content_ir() {
    for path in fixtures("fixtures/valid") {
        let golden_path = path.with_extension("json");
        let Ok(json) = fs::read_to_string(&golden_path) else {
            continue; // missing goldens are reported by the golden test
        };
        let doc = content::Document::from_json(&json)
            .unwrap_or_else(|err| panic!("{} does not round-trip: {err}", golden_path.display()));
        assert_eq!(doc.schema_version, content::SCHEMA_VERSION);
    }
}

#[test]
fn invalid_fixtures_all_produce_diagnostics() {
    let paths = fixtures("fixtures/invalid");
    assert!(paths.len() >= 4, "invalid corpus shrank: {paths:?}");
    for path in paths {
        let source = fs::read_to_string(&path).unwrap();
        let diags = content::parse_named(&source, &path.display().to_string())
            .expect_err(&format!("{} unexpectedly parsed", path.display()));
        assert!(!diags.is_empty());
        assert!(
            diags.iter().all(|d| d.line.is_some()),
            "diagnostic without location for {}: {diags:#?}",
            path.display()
        );
    }
}
