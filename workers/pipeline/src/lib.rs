//! Pure decision core: push classification, status building, request/response
//! shaping — natively testable. Transport lives behind the `worker` feature;
//! HMAC verification lives in the shared `authn` crate.

use std::collections::BTreeSet;

use content::{post_slug, source_path, Diagnostic, SITE_TAG};
use serde::{Deserialize, Serialize};

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
    /// The push deletes the ref itself (branch deletion).
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

/// The reconcile is a full rebuild, so only two facts matter: does the push
/// touch code, and how many post sources (for the status message).
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

/// Reconcile target; persisted so alarm-driven reconciles can run without
/// a request in hand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconcileConfig {
    /// `owner/repo`, as GitHub API paths want it.
    pub repository: String,
    pub branch: String,
}

/// CI's callback body. CI also sends the triggering `sha`; serde ignores
/// it — a reconcile converges to HEAD.
#[derive(Debug, Deserialize)]
pub struct PublishRequest {
    /// `owner/repo`, as GitHub API paths want it.
    pub repository: String,
    /// Empty only from a caller predating the field; the handler rejects it.
    #[serde(default)]
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

/// One concise line for the status; the API clamps descriptions to ~140 chars.
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

/// What the webhook does with a verified, parsed push event.
#[derive(Debug, PartialEq, Eq)]
pub enum WebhookAction {
    /// Acknowledge and stop.
    Ignore(&'static str),
    /// Content-only push: poke the coordinator now.
    Reconcile(ReconcileConfig),
    /// Code push: dispatch CI (deploy precedes publish), posting `pending`
    /// with this description; CI's `/publish` callback reconciles.
    DispatchCi { description: String },
}

/// The whole push decision tree, transport-free.
pub fn decide_push(event: &PushEvent) -> WebhookAction {
    if event.deleted || !event.is_default_branch() {
        return WebhookAction::Ignore("ignored: not a default-branch push");
    }
    match classify(&event.commits) {
        PushClass::Ignore => WebhookAction::Ignore("ignored: no content or code changes"),
        PushClass::ContentOnly => WebhookAction::Reconcile(ReconcileConfig {
            repository: event.repository.full_name.clone(),
            branch: event.repository.default_branch.clone(),
        }),
        PushClass::Code { touched_posts } => WebhookAction::DispatchCi {
            description: code_push_description(touched_posts),
        },
    }
}

/// Status text for a parked code push: the deploy must land first.
pub fn code_push_description(touched_posts: usize) -> String {
    match touched_posts {
        0 => "code push: publish reconciles after the CI deploy".to_string(),
        n => format!("code push: {n} content changes publish after the CI deploy"),
    }
}

/// What one reconcile did — the inputs its commit status is built from.
#[derive(Debug, Clone, Copy)]
pub struct ReconcileOutcome {
    pub published: usize,
    pub failed: usize,
    /// How many failures actually had a previous version to keep —
    /// claiming "kept" for a dropped post would lie.
    pub carried: usize,
    /// Whether the purge covered everything the flip (plus debt) made stale.
    pub purged: bool,
}

/// One reconcile's status. A failed purge fails it even when every post
/// published: readers still see stale pages, and a green check must never
/// hide that.
pub fn reconcile_description(
    outcome: ReconcileOutcome,
    diags: &[Diagnostic],
) -> (StatusState, String) {
    let posts = |n: usize| format!("{n} post{}", if n == 1 { "" } else { "s" });
    let (state, description) = match outcome.failed {
        0 => (
            StatusState::Success,
            format!("reconciled: {} published", posts(outcome.published)),
        ),
        n => {
            let kept = if outcome.carried == n {
                "previous versions kept"
            } else {
                "previous versions kept where available"
            };
            (
                StatusState::Failure,
                format!(
                    "{} failed validation ({kept}); {}",
                    posts(n),
                    failure_description(diags)
                ),
            )
        }
    };
    if outcome.purged {
        (state, description)
    } else {
        (
            StatusState::Failure,
            format!("{description}; cache purge failed — pages may be stale"),
        )
    }
}

/// One reconcile's purge scope: the tags this flip made stale plus the debt
/// a previously failed purge left behind, so a same-HEAD reconcile still
/// retries it. An unreadable debt ledger (`None`) escalates to a full site
/// purge — over-purge, never staleness.
pub fn purge_scope(stale: Vec<String>, debt: Option<Vec<String>>) -> Vec<String> {
    let debt = debt.unwrap_or_else(|| vec![SITE_TAG.to_string()]);
    stale
        .into_iter()
        .chain(debt)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
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

pub fn statuses_url(repo: &str, sha: &str) -> String {
    format!("https://api.github.com/repos/{repo}/statuses/{sha}")
}

/// The environment content publishes deploy to — GitHub surfaces it on the
/// merged PR's timeline and the repo's Environments panel.
pub const DEPLOY_ENVIRONMENT: &str = "content";

pub fn deployments_url(repo: &str) -> String {
    format!("https://api.github.com/repos/{repo}/deployments")
}

pub fn deployment_statuses_url(repo: &str, id: u64) -> String {
    format!("https://api.github.com/repos/{repo}/deployments/{id}/statuses")
}

/// `required_contexts: []` keeps our own commit status from gating the
/// record; `auto_merge: false` keeps GitHub from touching the ref.
pub fn deployment_payload(sha: &str, description: &str) -> String {
    serde_json::json!({
        "ref": sha,
        "environment": DEPLOY_ENVIRONMENT,
        "description": clamp(description),
        "auto_merge": false,
        "required_contexts": [],
        "production_environment": true,
    })
    .to_string()
}

/// `auto_inactive` retires the previous deployment, so the Environments
/// panel always shows exactly what is live.
pub fn deployment_status_payload(environment_url: &str, description: &str) -> String {
    let mut payload = serde_json::json!({
        "state": "success",
        "description": clamp(description),
        "auto_inactive": true,
    });
    if !environment_url.is_empty() {
        payload["environment_url"] = environment_url.into();
    }
    payload.to_string()
}

/// The new deployment's id out of a creation response.
pub fn parse_deployment_id(json: &serde_json::Value) -> Result<u64, String> {
    json["id"]
        .as_u64()
        .ok_or_else(|| "deployment response carries no id".to_string())
}

pub fn dispatch_url(repo: &str) -> String {
    format!("https://api.github.com/repos/{repo}/actions/workflows/{WORKFLOW_FILE}/dispatches")
}

/// Runs on the pushed branch, carrying the SHA so CI can report back on it.
pub fn dispatch_payload(branch: &str, sha: &str) -> String {
    serde_json::json!({ "ref": branch, "inputs": { "sha": sha } }).to_string()
}
