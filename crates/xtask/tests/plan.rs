//! Pins `xtask plan`'s output files — the wire contract fed to wrangler.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture(tree: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(tree)
        .join("content/blog")
        .display()
        .to_string()
}

fn out_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("xtask-plan-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn xtask(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .output()
        .expect("xtask must run")
}

fn read_json(path: &Path) -> serde_json::Value {
    let raw = std::fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("{} must exist: {err}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|err| panic!("{} must parse: {err}", path.display()))
}

#[test]
fn plan_writes_bulk_puts_with_the_index_last() {
    let out = out_dir("valid");
    std::fs::create_dir_all(&out).unwrap();

    let run = xtask(&[
        "plan",
        "--sha",
        "testsha",
        "--content-dir",
        &fixture("valid"),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert!(
        run.status.success(),
        "plan failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );

    // writes.json is wrangler `kv bulk put` input: [{"key","value"}], index last
    let writes = read_json(&out.join("writes.json"));
    let writes = writes.as_array().expect("writes.json must be an array");
    for write in writes {
        assert!(write["key"].is_string() && write["value"].is_string());
        assert_eq!(write.as_object().unwrap().len(), 2);
    }
    assert_eq!(
        writes.last().unwrap()["key"],
        "snapshot:testsha:index",
        "the index write must come last"
    );

    let pointer = read_json(&out.join("pointer.json"));
    assert_eq!(pointer["sha"], "testsha");

    std::fs::remove_dir_all(&out).unwrap();
}

/// A typo'd flag must fail, never silently plan against a default.
#[test]
fn unrecognized_flags_are_rejected() {
    let run = xtask(&["check", "--contnet-dir", "somewhere"]);
    assert!(!run.status.success());
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("unrecognized argument `--contnet-dir`"),
        "stderr was: {stderr}"
    );
}
