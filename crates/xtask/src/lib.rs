//! The workspace's scripts crate (the cargo-xtask pattern): content-tree
//! discovery and check, testable natively. The binary (`main.rs`) stays a
//! thin shim, and transport belongs to `just` recipes piping into wrangler —
//! this crate never talks to the network.

use std::path::Path;

use content::{Diagnostic, IndexEntry, Manifest};
use publish::{ParsedPost, PostSource};

/// What wrangler's `kv key get` prints (to stdout, exit 0) when the key does
/// not exist — the only non-JSON output [`parse_index`] accepts.
const WRANGLER_VALUE_NOT_FOUND: &str = "Value not found";

/// Walks a `content/blog` tree (`{slug}/index.mdx`, per CONTENT.md) into
/// post sources, sorted by slug. Structural problems — a post directory
/// without `index.mdx`, unreadable files — come back as diagnostics.
pub fn discover(content_dir: &Path) -> (Vec<PostSource>, Vec<Diagnostic>) {
    let mut posts = Vec::new();
    let mut diags = Vec::new();
    let mut structural = |path: &Path, message: String| {
        diags.push(Diagnostic {
            message,
            file: Some(path.display().to_string()),
            line: None,
            column: None,
        });
    };

    let entries = match std::fs::read_dir(content_dir) {
        Ok(entries) => entries,
        Err(err) => {
            structural(content_dir, format!("cannot read content tree: {err}"));
            return (posts, diags);
        }
    };
    for entry in entries.flatten().filter(|e| e.path().is_dir()) {
        let slug = entry.file_name().to_string_lossy().into_owned();
        let file = entry.path().join("index.mdx");
        if !file.is_file() {
            structural(&entry.path(), "post directory has no index.mdx".into());
            continue;
        }
        match std::fs::read_to_string(&file) {
            Ok(source) => posts.push(PostSource {
                slug,
                file: file.display().to_string(),
                source,
            }),
            Err(err) => structural(&file, format!("cannot read post: {err}")),
        }
    }
    posts.sort_by(|a, b| a.slug.cmp(&b.slug));
    (posts, diags)
}

/// `xtask check`: discover + parse + validate the whole tree, collecting
/// every problem. Ok means the tree is publishable.
pub fn check_tree(
    content_dir: &Path,
    manifest: &Manifest,
) -> Result<Vec<ParsedPost>, Vec<Diagnostic>> {
    let (posts, mut diags) = discover(content_dir);
    match publish::check(&posts, manifest) {
        Ok(parsed) if diags.is_empty() => Ok(parsed),
        Ok(_) => Err(diags),
        Err(check_diags) => {
            diags.extend(check_diags);
            Err(diags)
        }
    }
}

/// Parses the KV `index` as captured by `just publish`. Exactly two inputs
/// mean "no index yet" (the first-ever publish): empty output, or wrangler's
/// `Value not found` message (`kv key get` prints it and exits 0). Anything
/// else must parse as the index's JSON array — never a silent fallback,
/// because a swallowed index would make `plan` rewrite the listing from
/// scratch, unlisting every post it wasn't given.
pub fn parse_index(raw: &str) -> Result<Vec<IndexEntry>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == WRANGLER_VALUE_NOT_FOUND {
        return Ok(Vec::new());
    }
    serde_json::from_str(trimmed).map_err(|err| format!("index is not valid JSON: {err}"))
}
