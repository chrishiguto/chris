//! Workspace scripts binary (cargo-xtask); the justfile is the front door
//! and wrangler moves the bytes.

use std::path::Path;
use std::process::ExitCode;

use content::{CurrentPointer, Diagnostic, IndexEntry};

const USAGE: &str = "usage:
  xtask check [--content-dir DIR]
  xtask plan --sha LABEL [--content-dir DIR] --index FILE --out DIR [--origin URL]
  xtask pointer FILE
  xtask ast FILE.mdx";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("check") => check(&args[1..]),
        Some("plan") => plan(&args[1..]),
        Some("pointer") => pointer(&args[1..]),
        Some("ast") => ast(&args[1..]),
        _ => Err(USAGE.into()),
    };
    match result {
        Ok(summary) => {
            println!("{summary}");
            ExitCode::SUCCESS
        }
        Err(failure) => {
            eprintln!("{failure}");
            ExitCode::FAILURE
        }
    }
}

fn check(args: &[String]) -> Result<String, String> {
    let flags = parse_flags(args, &["--content-dir"])?;
    let content_dir = flags
        .get("--content-dir")
        .cloned()
        .unwrap_or_else(default_content_dir);
    let posts = xtask::check_tree(Path::new(&content_dir), &app::manifest())
        .map_err(|diags| render_diags(&diags))?;
    Ok(format!("checked {} posts — all valid", posts.len()))
}

/// The whole tree becomes one snapshot — a broken post blocks the plan.
/// The previous index feeds only the purge set.
fn plan(args: &[String]) -> Result<String, String> {
    let flags = parse_flags(
        args,
        &["--sha", "--content-dir", "--index", "--out", "--origin"],
    )?;
    let content_dir = flags
        .get("--content-dir")
        .cloned()
        .unwrap_or_else(default_content_dir);
    let index_file = flags
        .get("--index")
        .ok_or("plan: --index FILE is required")?;
    let out_dir = flags.get("--out").ok_or("plan: --out DIR is required")?;
    let sha = flags
        .get("--sha")
        .ok_or("plan: --sha LABEL is required")?
        .clone();

    let manifest = app::manifest();
    let posts = xtask::check_tree(Path::new(&content_dir), &manifest)
        .map_err(|diags| render_diags(&diags))?;
    let prev_index = read_index(Path::new(&index_file))?;
    let plan = publish::snapshot(&prev_index, &posts, Vec::new(), &sha)
        .map_err(|err| format!("serializing snapshot plan: {err}"))?;

    let out = Path::new(&out_dir);
    std::fs::create_dir_all(out).map_err(|err| format!("creating {out_dir}: {err}"))?;
    // Posts first, index last: a torn bulk put must never leave an index
    // naming missing posts.
    let writes: Vec<_> = plan
        .post_writes
        .iter()
        .chain(std::iter::once(&plan.index_write))
        .collect();
    let writes =
        serde_json::to_value(&writes).map_err(|err| format!("serializing writes: {err}"))?;
    write_json(out, "writes.json", &writes)?;
    let pointer = serde_json::to_value(CurrentPointer { sha: sha.clone() })
        .map_err(|err| format!("serializing pointer: {err}"))?;
    write_json(out, "pointer.json", &pointer)?;
    // One purge-N.json per API-capped chunk; stale chunks from a previous,
    // larger plan must not ride along.
    for stale in purge_files(out)? {
        std::fs::remove_file(&stale).map_err(|err| format!("removing stale purge file: {err}"))?;
    }
    let origin = flags.get("--origin").cloned().unwrap_or_default();
    let chunks = plan.purge_chunks(&origin);
    for (n, chunk) in chunks.iter().enumerate() {
        write_json(out, &format!("purge-{n}.json"), &serde_json::json!(chunk))?;
    }

    Ok(format!(
        "planned snapshot {sha}: {} posts, {} purge paths in {} chunks → {out_dir}",
        plan.index.len(),
        plan.purge.len(),
        chunks.len(),
    ))
}

fn purge_files(dir: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let entries = std::fs::read_dir(dir).map_err(|err| format!("reading {dir:?}: {err}"))?;
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| format!("reading {dir:?}: {err}"))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("purge-") && name.ends_with(".json") {
            files.push(entry.path());
        }
    }
    Ok(files)
}

/// Resolves a captured `current` pointer to the previous index's KV key —
/// the key grammar never leaks into bash.
fn pointer(args: &[String]) -> Result<String, String> {
    let [path] = args else {
        return Err(USAGE.into());
    };
    let raw = std::fs::read_to_string(path).map_err(|err| format!("{path}: {err}"))?;
    let sha = xtask::parse_pointer(&raw).map_err(|err| format!("{path}: {err}"))?;
    Ok(content::index_key_at(sha.as_deref()))
}

fn ast(args: &[String]) -> Result<String, String> {
    let [path] = args else {
        return Err(USAGE.into());
    };
    let source = std::fs::read_to_string(path).map_err(|err| format!("{path}: {err}"))?;
    let doc = content::parse(&source, path).map_err(|diags| render_diags(&diags))?;
    serde_json::to_string_pretty(&doc).map_err(|err| format!("{path}: failed to serialize: {err}"))
}

/// Reads the previous index; unexpected content fails closed rather than
/// silently planning a fresh index.
fn read_index(path: &Path) -> Result<Vec<IndexEntry>, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|err| format!("{}: cannot read index: {err}", path.display()))?;
    xtask::parse_index(&raw).map_err(|err| format!("{}: {err}", path.display()))
}

fn write_json(dir: &Path, name: &str, value: &serde_json::Value) -> Result<(), String> {
    let path = dir.join(name);
    std::fs::write(&path, value.to_string())
        .map_err(|err| format!("writing {}: {err}", path.display()))
}

fn default_content_dir() -> String {
    content::CONTENT_ROOT.into()
}

/// `--flag VALUE` pairs; a typo'd flag errors rather than silently falling
/// back to a default.
fn parse_flags(
    args: &[String],
    known: &[&str],
) -> Result<std::collections::HashMap<String, String>, String> {
    let mut flags = std::collections::HashMap::new();
    let mut args = args.iter();
    while let Some(flag) = args.next() {
        if !known.contains(&flag.as_str()) {
            return Err(format!("unrecognized argument `{flag}`\n{USAGE}"));
        }
        let value = args
            .next()
            .filter(|value| !value.starts_with("--"))
            .ok_or_else(|| format!("{flag} needs a value"))?;
        flags.insert(flag.clone(), value.clone());
    }
    Ok(flags)
}

fn render_diags(diags: &[Diagnostic]) -> String {
    let mut lines: Vec<String> = diags.iter().map(ToString::to_string).collect();
    lines.push(format!(
        "{} problem{} found",
        diags.len(),
        if diags.len() == 1 { "" } else { "s" }
    ));
    lines.join("\n")
}
