//! Component discovery: each `content/blog/{slug}/components.rs` becomes a
//! `#[path]` module under `app::components`, so per-post code is real
//! workspace Rust (full rust-analyzer) that joins the registry inventory.

use std::path::Path;
use std::{env, fs};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("cargo sets this");
    let blog = Path::new(&manifest_dir).join("../content/blog");
    // A directory path makes cargo scan it recursively, so new posts and
    // edited components both trigger a rebuild.
    println!("cargo:rerun-if-changed={}", blog.display());

    let mut declarations: Vec<String> = fs::read_dir(&blog)
        .map(|posts| {
            posts
                .flatten()
                .filter_map(|post| {
                    let file = post.path().join("components.rs");
                    let slug = post.file_name().into_string().ok()?;
                    file.exists().then(|| declaration(&slug, &file))
                })
                .collect()
        })
        .unwrap_or_default();
    declarations.sort();

    let out = Path::new(&env::var("OUT_DIR").expect("cargo sets this")).join("post_components.rs");
    fs::write(out, declarations.concat()).expect("OUT_DIR is writable");
}

/// `content/blog/orbit-demo/components.rs` → `pub mod post_orbit_demo`. A
/// slug that isn't a valid identifier fails the build loudly — publish
/// validation enforces lowercase slugs anyway.
fn declaration(slug: &str, file: &Path) -> String {
    format!(
        "#[path = {:?}]\npub mod post_{};\n",
        file.display().to_string(),
        slug.replace('-', "_")
    )
}
