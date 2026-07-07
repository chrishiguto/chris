//! `xtask` — workspace scripts over the shared content crates (the
//! cargo-xtask pattern; front door is the justfile):
//!
//! - `check` gates a content tree locally,
//! - `plan` lays the whole tree out as one immutable snapshot — `just
//!   publish` pipes the files into `wrangler kv bulk put` and flips the
//!   `current` pointer (break-glass),
//! - `pointer` resolves a captured `current` pointer to the previous
//!   snapshot's index key,
//! - `ast` prints one post's AST JSON for hand-seeding local KV.

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
    let content_dir = flag_value(args, "--content-dir")?.unwrap_or_else(default_content_dir);
    let posts = xtask::check_tree(Path::new(&content_dir), &app::manifest())
        .map_err(|diags| render_diags(&diags))?;
    Ok(format!("checked {} posts — all valid", posts.len()))
}

/// Break-glass publish plan: the whole local tree becomes one snapshot — a
/// broken post blocks the plan. The previous index feeds only the purge set.
fn plan(args: &[String]) -> Result<String, String> {
    let content_dir = flag_value(args, "--content-dir")?.unwrap_or_else(default_content_dir);
    let index_file = flag_value(args, "--index")?.ok_or("plan: --index FILE is required")?;
    let out_dir = flag_value(args, "--out")?.ok_or("plan: --out DIR is required")?;
    let sha = flag_value(args, "--sha")?.ok_or("plan: --sha LABEL is required")?;

    let manifest = app::manifest();
    let posts = xtask::check_tree(Path::new(&content_dir), &manifest)
        .map_err(|diags| render_diags(&diags))?;
    let prev_index = read_index(Path::new(&index_file))?;
    let plan = publish::snapshot(&prev_index, &posts, Vec::new(), &sha)
        .map_err(|err| format!("serializing snapshot plan: {err}"))?;

    let out = Path::new(&out_dir);
    std::fs::create_dir_all(out).map_err(|err| format!("creating {out_dir}: {err}"))?;
    // wrangler `kv bulk put` takes [{"key","value"}].
    let writes: Vec<serde_json::Value> = plan
        .writes
        .iter()
        .map(|w| serde_json::json!({ "key": w.key, "value": w.value }))
        .collect();
    write_json(out, "writes.json", &serde_json::Value::Array(writes))?;
    let pointer = serde_json::to_value(CurrentPointer { sha: sha.clone() })
        .map_err(|err| format!("serializing pointer: {err}"))?;
    write_json(out, "pointer.json", &pointer)?;
    // With --origin the purge file is curl-ready full URLs; without it,
    // bare paths (purge gets skipped anyway).
    let origin = flag_value(args, "--origin")?.unwrap_or_default();
    let purge: Vec<String> = plan
        .purge
        .iter()
        .map(|path| format!("{}{path}", origin.trim_end_matches('/')))
        .collect();
    write_json(out, "purge.json", &serde_json::json!(purge))?;

    Ok(format!(
        "planned snapshot {sha}: {} posts, {} purge paths → {out_dir}",
        plan.index.len(),
        plan.purge.len(),
    ))
}

/// Prints the KV key holding the previous snapshot's index, resolved from a
/// captured `current` pointer — the key grammar never leaks into bash.
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
    let doc = content::parse_named(&source, path).map_err(|diags| render_diags(&diags))?;
    serde_json::to_string_pretty(&doc).map_err(|err| format!("{path}: failed to serialize: {err}"))
}

/// Reads the previous snapshot's index; anything unexpected fails closed
/// (see [`xtask::parse_index`]) instead of silently planning a fresh index.
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
    "content/blog".into()
}

/// Value of `--flag VALUE`; errors when the flag dangles without a value.
fn flag_value(args: &[String], flag: &str) -> Result<Option<String>, String> {
    match args.iter().position(|a| a == flag) {
        Some(i) => args
            .get(i + 1)
            .filter(|v| !v.starts_with("--"))
            .map(|v| Some(v.clone()))
            .ok_or_else(|| format!("{flag} needs a value")),
        None => Ok(None),
    }
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
