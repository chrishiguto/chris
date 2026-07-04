//! The `blog` CLI's testable core: content-tree discovery and check.
//! The binary (`main.rs`) and the Cloudflare KV transport (`kv.rs`) stay
//! thin shims per the PRD testing decisions.

use std::path::Path;

use content_parser::Diagnostic;
use publish_core::{ParsedPost, PostSource};
use registry::Manifest;

pub mod kv;

/// The deployed component vocabulary, collected from `app`'s inventory
/// registrations — `blog check` validates against exactly what the site
/// renders with.
pub fn manifest() -> Manifest {
    // The registrations only link if at least one `app` symbol is referenced
    // from this binary — with zero references the linker drops the rlib and
    // the vocabulary comes back empty (pinned by the manifest test).
    std::hint::black_box(app::app::PRELOADED_FONTS);
    registry::manifest()
}

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

/// `blog check`: discover + parse + validate the whole tree, collecting
/// every problem (user story 14). Ok means the tree is publishable.
pub fn check_tree(
    content_dir: &Path,
    manifest: &Manifest,
) -> Result<Vec<ParsedPost>, Vec<Diagnostic>> {
    let (posts, mut diags) = discover(content_dir);
    match publish_core::check(&posts, manifest) {
        Ok(parsed) if diags.is_empty() => Ok(parsed),
        Ok(_) => Err(diags),
        Err(check_diags) => {
            diags.extend(check_diags);
            Err(diags)
        }
    }
}
