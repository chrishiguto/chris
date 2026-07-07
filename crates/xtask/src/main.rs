//! `xtask` — workspace scripts over the shared content crates (the
//! cargo-xtask pattern; front door is the justfile):
//!
//! - `check` gates a content tree locally,
//! - `plan` turns the tree into wrangler-ready KV bulk files — `just publish`
//!   pipes them into `wrangler kv bulk put/delete` (break-glass),
//! - `ast` prints one post's AST JSON for hand-seeding local KV.

use std::path::Path;
use std::process::ExitCode;

use content::{Diagnostic, IndexEntry};
use publish::ParsedPost;

const USAGE: &str = "usage:
  xtask check [--content-dir DIR]
  xtask plan (--all | SLUG...) [--content-dir DIR] --index FILE --out DIR
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
    let content_dir = flag_value(args, "--content-dir")?.unwrap_or_else(default_content_dir);
    let posts = xtask::check_tree(Path::new(&content_dir), &app::manifest())
        .map_err(|diags| render_diags(&diags))?;
    Ok(format!("checked {} posts — all valid", posts.len()))
}

fn plan(args: &[String]) -> Result<String, String> {
    let content_dir = flag_value(args, "--content-dir")?.unwrap_or_else(default_content_dir);
    let index_file = flag_value(args, "--index")?.ok_or("plan: --index FILE is required")?;
    let out_dir = flag_value(args, "--out")?.ok_or("plan: --out DIR is required")?;
    let all = args.iter().any(|a| a == "--all");
    let slugs: Vec<&String> = positional(args);
    if all != slugs.is_empty() {
        return Err("plan: pass either --all or at least one SLUG".into());
    }

    let content_dir = Path::new(&content_dir);
    let manifest = app::manifest();
    let changed = if all {
        xtask::check_tree(content_dir, &manifest).map_err(|diags| render_diags(&diags))?
    } else {
        selected_posts(content_dir, &slugs, &manifest)?
    };

    let prev_index = read_index(Path::new(&index_file))?;
    // --all rebuilds the listing from the tree: entries whose posts are gone
    // locally are pruned and their KV documents deleted (the manual
    // counterpart of the pipeline's delete-on-push).
    let removed: Vec<String> = if all {
        prev_index
            .iter()
            .filter(|entry| !changed.iter().any(|post| post.slug == entry.slug))
            .map(|entry| entry.slug.clone())
            .collect()
    } else {
        Vec::new()
    };

    let plan = publish::plan(prev_index, &changed, &removed)
        .map_err(|err| format!("serializing publish plan: {err}"))?;

    let out = Path::new(&out_dir);
    std::fs::create_dir_all(out).map_err(|err| format!("creating {out_dir}: {err}"))?;
    // wrangler `kv bulk put` takes [{"key","value"}]; `kv bulk delete` takes
    // plain key strings (workers-sdk KeyValue / deleteKVBulkKeyValue).
    let writes: Vec<serde_json::Value> = plan
        .writes
        .iter()
        .map(|w| serde_json::json!({ "key": w.key, "value": w.value }))
        .collect();
    write_json(out, "writes.json", &serde_json::Value::Array(writes))?;
    write_json(out, "deletes.json", &serde_json::json!(plan.deletes))?;
    // Purge-by-URL matches full URLs exactly; with --origin the file is
    // curl-ready, without it it holds bare paths (purge gets skipped anyway).
    let origin = flag_value(args, "--origin")?.unwrap_or_default();
    let purge: Vec<String> = plan
        .purge
        .iter()
        .map(|path| format!("{}{path}", origin.trim_end_matches('/')))
        .collect();
    write_json(out, "purge.json", &serde_json::json!(purge))?;

    let published: Vec<&str> = changed.iter().map(|post| post.slug.as_str()).collect();
    let removed_note = if removed.is_empty() {
        String::new()
    } else {
        format!(", removing {}", removed.join(", "))
    };
    Ok(format!(
        "planned {} (index: {} entries{removed_note}; {} purge paths) → {out_dir}",
        published.join(", "),
        plan.index.len(),
        plan.purge.len(),
    ))
}

fn ast(args: &[String]) -> Result<String, String> {
    let [path] = args else {
        return Err(USAGE.into());
    };
    let source = std::fs::read_to_string(path).map_err(|err| format!("{path}: {err}"))?;
    let doc = content::parse_named(&source, path).map_err(|diags| render_diags(&diags))?;
    serde_json::to_string_pretty(&doc).map_err(|err| format!("{path}: failed to serialize: {err}"))
}

/// Break-glass single-post publish: only the requested posts must be valid,
/// so one broken draft elsewhere in the tree cannot block an urgent fix.
fn selected_posts(
    content_dir: &Path,
    slugs: &[&String],
    manifest: &content::Manifest,
) -> Result<Vec<ParsedPost>, String> {
    let (sources, _) = xtask::discover(content_dir);
    let selected: Vec<_> = slugs
        .iter()
        .map(|slug| {
            sources
                .iter()
                .find(|source| source.slug == **slug)
                .cloned()
                .ok_or_else(|| format!("no post at {}/{slug}/index.mdx", content_dir.display()))
        })
        .collect::<Result<_, _>>()?;
    publish::check(&selected, manifest).map_err(|diags| render_diags(&diags))
}

/// Reads the current KV `index`. The sentinel logic lives in
/// [`xtask::parse_index`]: only empty output or wrangler's exact
/// `Value not found` mean "first publish" — an unreadable file or any other
/// non-JSON content fails closed instead of silently planning a fresh index.
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

/// Positional args: everything not a `--flag` and not a flag's value.
fn positional(args: &[String]) -> Vec<&String> {
    let mut out = Vec::new();
    let mut skip = false;
    for arg in args {
        if skip {
            skip = false;
            continue;
        }
        if arg == "--all" {
            continue;
        }
        if arg.starts_with("--") {
            skip = true;
            continue;
        }
        out.push(arg);
    }
    out
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
