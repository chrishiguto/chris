//! The pipeline worker's pure decision core: signature verification, push
//! classification, reconcile vocabulary, status building — all natively
//! testable. The wasm shim behind the `worker` feature owns transport only.

use std::collections::BTreeSet;

use content::Diagnostic;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

#[cfg(feature = "worker")]
mod coordinator;
#[cfg(feature = "worker")]
mod net;
#[cfg(feature = "worker")]
mod server;

/// The commit-status context both publish paths report under.
pub const STATUS_CONTEXT: &str = "blog/publish";
/// The workflow the code path dispatches; its last step calls `/publish`.
pub const WORKFLOW_FILE: &str = "publish.yml";
/// The Commit Status API rejects descriptions longer than 140 characters.
const DESCRIPTION_LIMIT: usize = 140;

#[derive(Debug, Deserialize)]
pub struct PushEvent {
    #[serde(rename = "ref")]
    pub git_ref: String,
    /// Head SHA of the push: statuses and content fetches pin to it.
    pub after: String,
    /// True when the push deletes the ref itself (branch deletion).
    #[serde(default)]
    pub deleted: bool,
    pub repository: Repository,
    #[serde(default)]
    pub commits: Vec<PushCommit>,
}

impl PushEvent {
    pub fn is_default_branch(&self) -> bool {
        self.git_ref == format!("refs/heads/{}", self.repository.default_branch)
    }
}

#[derive(Debug, Deserialize)]
pub struct Repository {
    /// `owner/repo`, as GitHub API paths want it.
    pub full_name: String,
    pub default_branch: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct PushCommit {
    #[serde(default)]
    pub added: Vec<String>,
    #[serde(default)]
    pub modified: Vec<String>,
    #[serde(default)]
    pub removed: Vec<String>,
}

/// GitHub signs the raw body with HMAC-SHA256 (`X-Hub-Signature-256:
/// sha256=<hex>`); comparison is constant-time.
pub fn verify_signature(secret: &str, body: &[u8], header: Option<&str>) -> bool {
    let Some(expected) = header
        .and_then(|value| value.strip_prefix("sha256="))
        .and_then(decode_hex)
    else {
        return false;
    };
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(body);
    mac.verify_slice(&expected).is_ok()
}

fn decode_hex(hex: &str) -> Option<Vec<u8>> {
    let digit = |byte: u8| char::from(byte).to_digit(16);
    (!hex.is_empty() && hex.len().is_multiple_of(2))
        .then(|| {
            hex.as_bytes()
                .chunks(2)
                .map(|pair| u8::try_from(digit(pair[0])? * 16 + digit(pair[1])?).ok())
                .collect()
        })
        .flatten()
}

#[derive(Debug, PartialEq, Eq)]
pub enum PushClass {
    /// Touches neither post sources nor code — acknowledge and stop.
    Ignore,
    /// Webhook fast path: reconcile immediately.
    ContentOnly,
    /// Deploy must precede publish: CI is dispatched, its `/publish`
    /// callback triggers the reconcile.
    Code { touched_posts: usize },
}

/// Classifies a push from its commits' paths. The reconcile is a full
/// rebuild, so only two facts matter: does the push touch code, and how
/// many post sources it touches (for the status message).
pub fn classify(commits: &[PushCommit]) -> PushClass {
    let paths = commits.iter().flat_map(|commit| {
        commit
            .added
            .iter()
            .chain(&commit.modified)
            .chain(&commit.removed)
    });
    let mut touched = BTreeSet::new();
    let mut code = false;
    for path in paths {
        touched.extend(post_slug(path));
        code = code || is_code_path(path);
    }
    match (code, touched.len()) {
        (true, touched_posts) => PushClass::Code { touched_posts },
        (false, 0) => PushClass::Ignore,
        (false, _) => PushClass::ContentOnly,
    }
}

/// A path that changes the deployed artifact or its build.
pub fn is_code_path(path: &str) -> bool {
    const CODE_ROOTS: [&str; 4] = ["app/", "crates/", "workers/", ".github/workflows/"];
    const CODE_FILES: [&str; 4] = ["Cargo.toml", "Cargo.lock", "justfile", "wrangler.toml"];
    path.ends_with(".rs")
        || CODE_ROOTS.iter().any(|root| path.starts_with(root))
        || CODE_FILES.contains(&path)
}

/// `content/blog/{slug}/index.mdx` → the slug; anything else is not a post
/// source.
pub fn post_slug(path: &str) -> Option<&str> {
    let (slug, file) = path.strip_prefix("content/blog/")?.split_once('/')?;
    (file == "index.mdx" && !slug.is_empty()).then_some(slug)
}

/// Repo path of a post source — the inverse of [`post_slug`] (not the
/// public URL; that is `content::post_path`).
pub fn source_path(slug: &str) -> String {
    format!("content/blog/{slug}/index.mdx")
}

/// Which repository and branch a reconcile converges to. The coordinator
/// persists it so alarm-driven reconciles can run without a request in hand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconcileConfig {
    /// `owner/repo`, as GitHub API paths want it.
    pub repository: String,
    pub branch: String,
}

/// CI's callback body: which repo and branch to reconcile. CI also sends
/// the triggering `sha`; serde ignores it — a reconcile converges to HEAD.
#[derive(Debug, Deserialize)]
pub struct PublishRequest {
    /// `owner/repo`, as GitHub API paths want it.
    pub repository: String,
    /// Empty only from a caller predating the field; the handler rejects it.
    #[serde(default)]
    pub branch: String,
}

/// Only CI may call `/publish`: exact `Bearer <secret>` match. Both sides
/// go through HMAC so the comparison is constant-time with no length oracle.
pub fn verify_publish_auth(secret: &str, header: Option<&str>) -> bool {
    let Some(token) = header.and_then(|value| value.strip_prefix("Bearer ")) else {
        return false;
    };
    let Ok(mac) = Hmac::<Sha256>::new_from_slice(b"publish-auth") else {
        return false;
    };
    let expected = mac.clone().chain_update(secret.as_bytes()).finalize();
    mac.chain_update(token.as_bytes())
        .verify_slice(&expected.into_bytes())
        .is_ok()
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

/// [`tree_post_slugs`] over a tree listing response's blob entries. A
/// truncated listing is an error — it would silently retire omitted posts.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StatusState {
    Pending,
    Success,
    Failure,
    Error,
}

/// Body for `POST /repos/{repo}/statuses/{sha}` (Commit Status API — Checks
/// API write access is GitHub-App-only).
pub fn status_payload(state: StatusState, description: &str) -> String {
    serde_json::json!({
        "state": state,
        "context": STATUS_CONTEXT,
        "description": clamp(description),
    })
    .to_string()
}

fn clamp(text: &str) -> String {
    match text.chars().count() {
        n if n <= DESCRIPTION_LIMIT => text.to_string(),
        _ => text
            .chars()
            .take(DESCRIPTION_LIMIT - 1)
            .chain(std::iter::once('…'))
            .collect(),
    }
}

/// One concise line for the status; the API clamps descriptions to ~140
/// chars, full detail stays `check`'s job.
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

/// Status text for a parked code push: the deploy must land first.
pub fn code_push_description(touched_posts: usize) -> String {
    match touched_posts {
        0 => "code push: publish reconciles after the CI deploy".to_string(),
        n => format!("code push: {n} content changes publish after the CI deploy"),
    }
}

/// One reconcile's status: success when every post validated, failure
/// naming the count that rode in as previous versions.
pub fn reconcile_description(
    published: usize,
    failed: usize,
    diags: &[Diagnostic],
) -> (StatusState, String) {
    let posts = |n: usize| format!("{n} post{}", if n == 1 { "" } else { "s" });
    match failed {
        0 => (
            StatusState::Success,
            format!("reconciled: {} published", posts(published)),
        ),
        n => (
            StatusState::Failure,
            format!(
                "{} failed validation (previous versions kept); {}",
                posts(n),
                failure_description(diags)
            ),
        ),
    }
}

/// Raw-content fetch for one post, pinned to the observed HEAD — every
/// source in one snapshot comes from one commit.
pub fn contents_url(repo: &str, slug: &str, sha: &str) -> String {
    format!(
        "https://api.github.com/repos/{repo}/contents/{}?ref={sha}",
        source_path(slug)
    )
}

/// Resolves the branch HEAD.
pub fn head_ref_url(repo: &str, branch: &str) -> String {
    format!("https://api.github.com/repos/{repo}/git/ref/heads/{branch}")
}

/// Recursive tree listing at a commit — the post inventory for a reconcile.
pub fn tree_url(repo: &str, sha: &str) -> String {
    format!("https://api.github.com/repos/{repo}/git/trees/{sha}?recursive=1")
}

pub fn statuses_url(repo: &str, sha: &str) -> String {
    format!("https://api.github.com/repos/{repo}/statuses/{sha}")
}

pub fn dispatch_url(repo: &str) -> String {
    format!("https://api.github.com/repos/{repo}/actions/workflows/{WORKFLOW_FILE}/dispatches")
}

/// `workflow_dispatch` body: run on the pushed branch, carrying the commit
/// SHA so CI can report back on it.
pub fn dispatch_payload(branch: &str, sha: &str) -> String {
    serde_json::json!({ "ref": branch, "inputs": { "sha": sha } }).to_string()
}

pub fn purge_url(zone: &str) -> String {
    format!("https://api.cloudflare.com/client/v4/zones/{zone}/purge_cache")
}

/// Purge-by-URL request body for one chunk of absolute URLs; the plan owns
/// origin-prefixing and chunking (`SnapshotPlan::purge_chunks`).
pub fn purge_body(files: &[String]) -> String {
    serde_json::json!({ "files": files }).to_string()
}
