//! `blog` — authoring tools over the shared content crates (PRD `blog-cli`):
//! `check` gates a tree locally, `publish --local` is the break-glass
//! publish path (ADR-0007).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use blog_cli::kv::KvClient;
use clap::{Parser, Subcommand};
use content_parser::Diagnostic;
use publish_core::ParsedPost;

#[derive(Parser)]
#[command(name = "blog", about = "Authoring tools for the blog content tree")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse and validate every post against the deployed component vocabulary
    Check {
        /// Root of the content tree
        #[arg(long, default_value = "content/blog")]
        content_dir: PathBuf,
    },
    /// Parse posts and write `post:*` + `index` to KV
    Publish {
        /// Publish from this machine via the Cloudflare API (v1's only path)
        #[arg(long)]
        local: bool,
        /// Publish the whole tree, pruning index entries it no longer contains
        #[arg(long, conflicts_with = "slugs")]
        all: bool,
        /// Slugs to publish
        #[arg(required_unless_present = "all")]
        slugs: Vec<String>,
        /// Root of the content tree
        #[arg(long, default_value = "content/blog")]
        content_dir: PathBuf,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
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

fn run(cli: Cli) -> Result<String, String> {
    match cli.command {
        Command::Check { content_dir } => {
            let posts = blog_cli::check_tree(&content_dir, &blog_cli::manifest())
                .map_err(|diags| render_diags(&diags))?;
            Ok(format!("checked {} posts — all valid", posts.len()))
        }
        Command::Publish {
            local,
            all,
            slugs,
            content_dir,
        } => {
            if !local {
                return Err(
                    "only `blog publish --local` exists in v1; the webhook path lands with the \
                     pipeline worker (Slice 6)"
                        .into(),
                );
            }
            publish_local(&content_dir, all, &slugs)
        }
    }
}

fn publish_local(content_dir: &Path, all: bool, slugs: &[String]) -> Result<String, String> {
    let manifest = blog_cli::manifest();
    let changed = if all {
        blog_cli::check_tree(content_dir, &manifest).map_err(|diags| render_diags(&diags))?
    } else {
        selected_posts(content_dir, slugs, &manifest)?
    };

    let kv = KvClient::from_env()?;
    let prev_index = kv.read_index()?;
    // --all rebuilds the listing from the tree: entries whose posts are gone
    // locally are pruned and their KV documents deleted (user story 5's
    // manual counterpart).
    let removed: Vec<String> = if all {
        prev_index
            .iter()
            .filter(|entry| !changed.iter().any(|post| post.slug == entry.slug))
            .map(|entry| entry.slug.clone())
            .collect()
    } else {
        Vec::new()
    };

    let plan = publish_core::plan(prev_index, &changed, &removed)
        .map_err(|err| format!("serializing publish plan: {err}"))?;
    kv.apply(&plan)?;

    let published: Vec<_> = changed.iter().map(|post| post.slug.as_str()).collect();
    let removed_note = if removed.is_empty() {
        String::new()
    } else {
        format!(", removed {}", removed.join(", "))
    };
    Ok(format!(
        "published {} (index: {} entries{removed_note})",
        published.join(", "),
        plan.index.len(),
    ))
}

/// Break-glass single-post publish: only the requested posts must be valid,
/// so one broken draft elsewhere in the tree cannot block an urgent fix.
fn selected_posts(
    content_dir: &Path,
    slugs: &[String],
    manifest: &registry::Manifest,
) -> Result<Vec<ParsedPost>, String> {
    let (sources, _) = blog_cli::discover(content_dir);
    let selected: Vec<_> = slugs
        .iter()
        .map(|slug| {
            sources
                .iter()
                .find(|source| source.slug == *slug)
                .cloned()
                .ok_or_else(|| format!("no post at {}/{slug}/index.mdx", content_dir.display()))
        })
        .collect::<Result<_, _>>()?;
    publish_core::check(&selected, manifest).map_err(|diags| render_diags(&diags))
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
