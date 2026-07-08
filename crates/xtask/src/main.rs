//! Workspace scripts binary (cargo-xtask); the justfile is the front door
//! and wrangler moves the bytes.

use std::path::Path;
use std::process::ExitCode;

use content::{CurrentPointer, Diagnostic};

const USAGE: &str = "usage:
  xtask check [--content-dir DIR]
  xtask plan --sha LABEL [--content-dir DIR] --out DIR
  xtask ast FILE.mdx";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("check") => check(&args[1..]),
        Some("plan") => plan(&args[1..]),
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
fn plan(args: &[String]) -> Result<String, String> {
    let flags = parse_flags(args, &["--sha", "--content-dir", "--out"])?;
    let content_dir = flags
        .get("--content-dir")
        .cloned()
        .unwrap_or_else(default_content_dir);
    let out_dir = flags.get("--out").ok_or("plan: --out DIR is required")?;
    let sha = flags
        .get("--sha")
        .ok_or("plan: --sha LABEL is required")?
        .clone();

    let manifest = app::manifest();
    let posts = xtask::check_tree(Path::new(&content_dir), &manifest)
        .map_err(|diags| render_diags(&diags))?;
    let plan = publish::snapshot(&posts, Vec::new(), &sha)
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

    Ok(format!(
        "planned snapshot {sha}: {} posts → {out_dir}",
        plan.index.len(),
    ))
}

fn ast(args: &[String]) -> Result<String, String> {
    let [path] = args else {
        return Err(USAGE.into());
    };
    let source = std::fs::read_to_string(path).map_err(|err| format!("{path}: {err}"))?;
    let doc = content::parse(&source, path).map_err(|diags| render_diags(&diags))?;
    serde_json::to_string_pretty(&doc).map_err(|err| format!("{path}: failed to serialize: {err}"))
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
