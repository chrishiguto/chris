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

/// Walks a `content/blog` tree (`{slug}/index.mdx`) into post sources,
/// sorted by slug. Structural problems come back as diagnostics.
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
        let file = entry.path().join(content::POST_FILE);
        if !file.is_file() {
            structural(
                &entry.path(),
                format!("post directory has no {}", content::POST_FILE),
            );
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

/// Parses a captured snapshot index. Exactly two inputs mean "no index yet":
/// empty output, or wrangler's `Value not found` (printed with exit 0).
/// Anything else must parse — a swallowed index would compute an empty
/// purge set.
pub fn parse_index(raw: &str) -> Result<Vec<IndexEntry>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == WRANGLER_VALUE_NOT_FOUND {
        return Ok(Vec::new());
    }
    serde_json::from_str(trimmed).map_err(|err| format!("index is not valid JSON: {err}"))
}

/// Parses a captured `current` pointer into its sha. Same sentinel contract
/// as [`parse_index`] — anything else must parse, never fall back.
pub fn parse_pointer(raw: &str) -> Result<Option<String>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == WRANGLER_VALUE_NOT_FOUND {
        return Ok(None);
    }
    content::CurrentPointer::from_json(trimmed)
        .map(|pointer| Some(pointer.sha))
        .map_err(|err| format!("current pointer is not valid JSON: {err}"))
}
