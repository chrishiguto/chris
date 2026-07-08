//! Content-tree discovery and checks, natively testable. Transport belongs
//! to wrangler via `just` — this crate never talks to the network.

use std::path::Path;

use content::{Diagnostic, Manifest};
use publish::{ParsedPost, PostSource};

/// Walks a `content/blog` tree (`{slug}/index.mdx`) into post sources, sorted by slug.
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

/// Discover + parse + validate the whole tree; Ok means publishable.
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
