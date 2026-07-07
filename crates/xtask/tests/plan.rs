//! Pins `xtask plan`'s output files — the wire contract fed to wrangler and curl.

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
fn plan_writes_bulk_puts_with_the_index_last_and_chunked_purge_urls() {
    let out = out_dir("valid");
    let index = out.join("prev-index.json");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(&index, "Value not found").unwrap();

    let run = xtask(&[
        "plan",
        "--sha",
        "testsha",
        "--content-dir",
        &fixture("valid"),
        "--index",
        index.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
        "--origin",
        "https://blog.example.com/",
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

    // purge-N.json chunks hold curl-ready full URLs under the origin.
    let purge = read_json(&out.join("purge-0.json"));
    let urls = purge.as_array().expect("purge chunk must be an array");
    assert!(urls.iter().all(|url| url
        .as_str()
        .unwrap()
        .starts_with("https://blog.example.com/")));
    assert!(!out.join("purge-1.json").exists(), "small plan, one chunk");

    std::fs::remove_dir_all(&out).unwrap();
}

/// A re-plan must not leave a previous, larger plan's purge chunks behind.
#[test]
fn plan_removes_stale_purge_chunks() {
    let out = out_dir("stale");
    let index = out.join("prev-index.json");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(&index, "").unwrap();
    std::fs::write(out.join("purge-7.json"), "[\"stale\"]").unwrap();

    let run = xtask(&[
        "plan",
        "--sha",
        "testsha",
        "--content-dir",
        &fixture("valid"),
        "--index",
        index.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert!(run.status.success());
    assert!(out.join("purge-0.json").exists());
    assert!(!out.join("purge-7.json").exists(), "stale chunk must go");

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
