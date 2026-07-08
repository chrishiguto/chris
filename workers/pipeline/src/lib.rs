//! Pure decision core: signature verification, push classification, status
//! building — natively testable. Transport lives behind the `worker` feature.

use std::collections::BTreeSet;

use content::{post_slug, source_path, Diagnostic};
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
/// The pre-merge check's context: branch pushes validate, never publish.
pub const CHECK_CONTEXT: &str = "blog/content-check";
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
    /// This worker's public origin, observed from the triggering request;
    /// statuses link their Details to `{status_origin}/status/{sha}`.
    /// Empty (or a pre-field persisted config) just drops the link.
    #[serde(default)]
    pub status_origin: String,
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
/// API write access is GitHub-App-only). `target_url` becomes the status's
/// "Details" link.
pub fn status_payload(
    context: &str,
    state: StatusState,
    description: &str,
    target_url: Option<&str>,
) -> String {
    let mut payload = serde_json::json!({
        "state": state,
        "context": context,
        "description": clamp(description),
    });
    if let Some(url) = target_url.filter(|url| !url.is_empty()) {
        payload["target_url"] = url.into();
    }
    payload.to_string()
}

/// Where a status's "Details" link lands: the pipeline's own record page.
pub fn status_target_url(origin: &str, sha: &str) -> Option<String> {
    let origin = origin.trim_end_matches('/');
    (!origin.is_empty()).then(|| format!("{origin}/status/{sha}"))
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
    /// Content-only branch push: validate the tree at this sha and post a
    /// `blog/content-check` status on it — nothing publishes.
    CheckBranch { sha: String },
}

/// The whole push decision tree, transport-free.
pub fn decide_push(event: &PushEvent) -> WebhookAction {
    if event.deleted {
        return WebhookAction::Ignore("ignored: ref deletion");
    }
    if !event.is_default_branch() {
        // A content-only branch push gets a dry-run check on its head sha —
        // the status shows in the PR before merge. Code-bearing branches
        // can't validate against a manifest that hasn't deployed, so their
        // content validates post-merge as always.
        return match classify(&event.commits) {
            PushClass::ContentOnly => WebhookAction::CheckBranch {
                sha: event.after.clone(),
            },
            _ => WebhookAction::Ignore("ignored: not a default-branch push"),
        };
    }
    match classify(&event.commits) {
        PushClass::Ignore => WebhookAction::Ignore("ignored: no content or code changes"),
        PushClass::ContentOnly => WebhookAction::Reconcile(ReconcileConfig {
            repository: event.repository.full_name.clone(),
            branch: event.repository.default_branch.clone(),
            // Pure decision; the transport layer fills in its own origin.
            status_origin: String::new(),
        }),
        PushClass::Code { touched_posts } => WebhookAction::DispatchCi {
            description: code_push_description(touched_posts),
        },
    }
}

/// The pre-merge check's status line: valid trees count their posts,
/// invalid ones lead with the first diagnostic.
pub fn branch_check_description(valid: usize, diags: &[Diagnostic]) -> (StatusState, String) {
    let posts = |n: usize| format!("{n} post{}", if n == 1 { "" } else { "s" });
    match diags.len() {
        0 => (
            StatusState::Success,
            format!("content valid — would publish {}", posts(valid)),
        ),
        _ => (StatusState::Failure, failure_description(diags)),
    }
}

/// Status text for a parked code push: the deploy must land first.
pub fn code_push_description(touched_posts: usize) -> String {
    match touched_posts {
        0 => "code push: publish reconciles after the CI deploy".to_string(),
        n => format!("code push: {n} content changes publish after the CI deploy"),
    }
}

/// One reconcile's status. `carried` is how many failures actually had a
/// previous version to keep — claiming "kept" for a dropped post would lie.
pub fn reconcile_description(
    published: usize,
    failed: usize,
    carried: usize,
    diags: &[Diagnostic],
) -> (StatusState, String) {
    let posts = |n: usize| format!("{n} post{}", if n == 1 { "" } else { "s" });
    match failed {
        0 => (
            StatusState::Success,
            format!("reconciled: {} published", posts(published)),
        ),
        n => {
            let kept = if carried == n {
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

/// What one reconcile did — stored per sha in the coordinator, served at
/// `/status/{sha}` (the commit status's Details link).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconcileRecord {
    pub sha: String,
    /// Slugs published from HEAD.
    pub published: Vec<String>,
    /// Slugs that failed validation and kept their previous version.
    pub carried: Vec<String>,
    /// Slugs that failed validation with nothing to carry — dropped.
    pub dropped: Vec<String>,
    /// Rendered `file:line:column: message` diagnostics.
    pub diagnostics: Vec<String>,
    pub purged: bool,
    /// Unix epoch millis when the reconcile finished.
    pub finished_at_ms: u64,
}

/// The record page: server-rendered, dependency-free HTML.
pub fn render_status_page(record: &ReconcileRecord) -> String {
    let list = |slugs: &[String]| match slugs {
        [] => "—".to_string(),
        _ => slugs
            .iter()
            .map(|slug| escape_html(slug))
            .collect::<Vec<_>>()
            .join(", "),
    };
    let diagnostics = match record.diagnostics.as_slice() {
        [] => String::new(),
        diags => format!(
            "<h2>diagnostics</h2><pre>{}</pre>",
            diags
                .iter()
                .map(|diag| escape_html(diag))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    };
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
         <title>publish {sha}</title>\
         <style>body{{font:16px/1.6 monospace;max-width:48rem;margin:3rem auto;\
         padding:0 1rem}}dt{{font-weight:bold;margin-top:1rem}}</style></head>\
         <body><h1>publish record</h1><dl>\
         <dt>commit</dt><dd>{sha}</dd>\
         <dt>published</dt><dd>{published}</dd>\
         <dt>kept previous version</dt><dd>{carried}</dd>\
         <dt>dropped</dt><dd>{dropped}</dd>\
         <dt>cache</dt><dd>{purged}</dd>\
         <dt>finished</dt><dd>{finished_at_ms} (unix ms)</dd>\
         </dl>{diagnostics}</body></html>",
        sha = escape_html(&record.sha),
        published = list(&record.published),
        carried = list(&record.carried),
        dropped = list(&record.dropped),
        purged = if record.purged {
            "purged"
        } else {
            "purge failed — 7-day TTL backstop"
        },
        finished_at_ms = record.finished_at_ms,
        diagnostics = diagnostics,
    )
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The sha out of a `/status/{sha}` path; rejects anything that is not a
/// plain hex-ish commit label so the DO never sees junk keys.
pub fn status_page_sha(path: &str) -> Option<&str> {
    let sha = path.strip_prefix("/status/")?;
    let valid = !sha.is_empty()
        && sha.len() <= 64
        && sha
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-');
    valid.then_some(sha)
}

pub fn dispatch_url(repo: &str) -> String {
    format!("https://api.github.com/repos/{repo}/actions/workflows/{WORKFLOW_FILE}/dispatches")
}

/// Runs on the pushed branch, carrying the SHA so CI can report back on it.
pub fn dispatch_payload(branch: &str, sha: &str) -> String {
    serde_json::json!({ "ref": branch, "inputs": { "sha": sha } }).to_string()
}
