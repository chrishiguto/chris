//! Pure decision core: reconcile vocabulary, GitHub URL shaping, and the
//! publish outcome — natively testable. Transport lives behind the `worker`
//! feature; the `/publish` bearer check lives in the shared `authn` crate.

use content::{post_slug, source_path, Diagnostic};
use serde::{Deserialize, Serialize};

#[cfg(feature = "worker")]
mod coordinator;
#[cfg(feature = "worker")]
mod net;
#[cfg(feature = "worker")]
mod server;

/// Reconcile target, and the `/publish` request body itself: the coordinator
/// reconciles KV to this branch's HEAD. Unknown fields are tolerated (no
/// `deny_unknown_fields`) so wire drift can't reject the call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconcileConfig {
    /// `owner/repo`, as GitHub API paths want it.
    pub repository: String,
    pub branch: String,
}

/// Sorted slugs of every post source in one recursive tree listing.
pub fn tree_post_slugs<'a>(paths: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut slugs: Vec<String> = paths
        .into_iter()
        .filter_map(post_slug)
        .map(String::from)
        .collect();
    slugs.sort();
    slugs.dedup();
    slugs
}

/// The branch HEAD out of a git ref response (`{"object": {"sha": …}}`).
pub fn parse_head_ref(json: &serde_json::Value) -> Result<String, String> {
    json["object"]["sha"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| "ref response carries no object.sha".to_string())
}

/// Post slugs from a tree listing's blob entries. A truncated listing is an
/// error — it would silently retire omitted posts.
pub fn parse_tree_listing(json: &serde_json::Value) -> Result<Vec<String>, String> {
    if json["truncated"].as_bool().unwrap_or(false) {
        return Err("tree listing is truncated".to_string());
    }
    let entries = json["tree"]
        .as_array()
        .ok_or_else(|| "tree response carries no tree array".to_string())?;
    let paths = entries
        .iter()
        .filter(|entry| entry["type"] == "blob")
        .filter_map(|entry| entry["path"].as_str());
    Ok(tree_post_slugs(paths))
}

/// One concise line naming the first validation failure (and a count when
/// there are more), for the outcome summary.
pub fn failure_description(diags: &[Diagnostic]) -> String {
    let first = diags
        .first()
        .map(|diag| match &diag.file {
            Some(file) => format!("{file}: {}", diag.message),
            None => diag.message.clone(),
        })
        .unwrap_or_default();
    match diags.len() {
        0 | 1 => first,
        n => format!("{n} errors; first: {first}"),
    }
}

/// The result of one reconcile, returned to the CI caller so the Actions run
/// — and the `content` deployment it surfaces on the merged PR — reflect what
/// happened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PublishOutcome {
    pub published: usize,
    pub failed: usize,
    /// How many failures actually had a previous version to keep — claiming
    /// "kept" for a dropped post would lie.
    pub carried: usize,
    /// Cache tags this flip made stale (changed/added/removed posts plus
    /// `views`), empty when nothing changed. CI purges them from the site's
    /// public `/__purge` — the only entrypoint that can evict its Workers
    /// Cache; the coordinator no longer purges.
    pub tags: Vec<String>,
    /// False when a post failed validation: readers still see stale or missing
    /// pages, so the run (and its deployment) must go red rather than hide it.
    pub ok: bool,
    /// One human line for the run log and the deployment description.
    pub summary: String,
}

impl PublishOutcome {
    pub fn new(
        published: usize,
        failed: usize,
        carried: usize,
        tags: Vec<String>,
        diags: &[Diagnostic],
    ) -> Self {
        let posts = |n: usize| format!("{n} post{}", if n == 1 { "" } else { "s" });
        let summary = match failed {
            0 => format!("reconciled: {} published", posts(published)),
            n => {
                let kept = if carried == n {
                    "previous versions kept"
                } else {
                    "previous versions kept where available"
                };
                format!(
                    "{} failed validation ({kept}); {}",
                    posts(n),
                    failure_description(diags)
                )
            }
        };
        Self {
            published,
            failed,
            carried,
            tags,
            ok: failed == 0,
            summary,
        }
    }
}

/// Raw-content fetch pinned to the observed HEAD — every source in one
/// snapshot comes from one commit.
pub fn contents_url(repo: &str, slug: &str, sha: &str) -> String {
    format!(
        "https://api.github.com/repos/{repo}/contents/{}?ref={sha}",
        source_path(slug)
    )
}

pub fn head_ref_url(repo: &str, branch: &str) -> String {
    format!("https://api.github.com/repos/{repo}/git/ref/heads/{branch}")
}

pub fn tree_url(repo: &str, sha: &str) -> String {
    format!("https://api.github.com/repos/{repo}/git/trees/{sha}?recursive=1")
}
