//! Component discovery: each `content/blog/{slug}/components.rs` becomes a
//! `#[path]` module under `app::components::blog`, so per-post code is real
//! workspace Rust (full rust-analyzer) that joins the registry inventory.

use std::path::Path;
use std::{env, fs};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("cargo sets this");
    // The `content/blog/{slug}` grammar is content::routes's; hardcoded
    // rather than pulling content in as a build-dependency for one literal.
    let blog = Path::new(&manifest_dir).join("../content/blog");
    // A directory path makes cargo scan it recursively, so new posts and
    // edited components both trigger a rebuild.
    println!("cargo:rerun-if-changed={}", blog.display());

    let posts = fs::read_dir(&blog)
        .unwrap_or_else(|err| panic!("cannot read the content tree {}: {err}", blog.display()));
    let mut declarations: Vec<String> = posts
        .flatten()
        .filter_map(|post| {
            let file = post.path().join("components.rs");
            let slug = post.file_name().into_string().ok()?;
            file.exists().then(|| declaration(&slug, &file))
        })
        .collect();
    declarations.sort();

    let out = Path::new(&env::var("OUT_DIR").expect("cargo sets this")).join("post_components.rs");
    fs::write(out, declarations.concat()).expect("OUT_DIR is writable");
}

/// `content/blog/orbit-demo/components.rs` → `pub mod post_orbit_demo`.
/// The slug grammar (content::valid_slug) is re-checked here because this
/// build runs on trees publish never gated, and a bad slug would otherwise
/// surface as an opaque rustc error in generated code.
fn declaration(slug: &str, file: &Path) -> String {
    let valid = slug.starts_with(|c: char| c.is_ascii_lowercase())
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !valid {
        panic!(
            "slug `{slug}` cannot name a component module: slugs are lowercase \
             (a-z, 0-9, -) and start with a letter"
        );
    }
    format!(
        "#[path = {:?}]\npub mod post_{};\n",
        file.display().to_string(),
        slug.replace('-', "_")
    )
}
