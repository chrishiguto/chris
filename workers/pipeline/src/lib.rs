//! The pipeline worker's pure decision core (ADR-0006/0007): webhook
//! signature verification, push classification, publish-set computation,
//! pending handling, and commit-status building — all natively testable.
//! The wasm shim (`server.rs`, behind the `worker` feature) owns transport
//! only: webhook HTTP in, GitHub API + KV out.

use std::collections::BTreeMap;

use content::Diagnostic;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

#[cfg(feature = "worker")]
mod server;

/// The commit-status context both publish paths report under (ADR-0007).
pub const STATUS_CONTEXT: &str = "blog/publish";
/// KV key of the parked publish list awaiting a CI callback (PRD KV schema).
pub const PENDING_KEY: &str = "pending";
/// The workflow the code path dispatches; its last step calls `/publish`.
pub const WORKFLOW_FILE: &str = "publish.yml";
/// The Commit Status API rejects descriptions longer than 140 characters.
const DESCRIPTION_LIMIT: usize = 140;

// --- webhook payload (only the fields the decision needs) ---

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

// --- signature verification (user story 34) ---

/// GitHub signs the raw request body with HMAC-SHA256 and sends
/// `X-Hub-Signature-256: sha256=<hex>`; comparison is constant-time
/// (`Mac::verify_slice`), so no timing oracle on the secret.
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

// --- classification (user stories 3, 5, 30, 31) ---

/// The posts a push wants published or retired, as slugs.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PublishSet {
    pub changed: Vec<String>,
    pub removed: Vec<String>,
}

impl PublishSet {
    pub fn len(&self) -> usize {
        self.changed.len() + self.removed.len()
    }

    pub fn is_empty(&self) -> bool {
        self.changed.is_empty() && self.removed.is_empty()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PushClass {
    /// Touches neither post sources nor code — acknowledge and stop.
    Ignore,
    /// Webhook fast path: publish immediately.
    ContentOnly(PublishSet),
    /// Deploy must precede publish: park as pending, CI drains (Slice 7).
    Code(PublishSet),
}

#[derive(Clone, Copy)]
enum SlugState {
    Changed,
    Removed,
}

/// Classifies a push from its commits' `added/modified/removed` paths.
/// Folded in commit order so a slug's *final* state within the push wins
/// (added-then-removed publishes as a removal, not both).
pub fn classify(commits: &[PushCommit]) -> PushClass {
    let events = commits.iter().flat_map(|commit| {
        commit
            .added
            .iter()
            .chain(&commit.modified)
            .map(|path| (path, SlugState::Changed))
            .chain(commit.removed.iter().map(|path| (path, SlugState::Removed)))
    });
    let (states, code) = events.fold(
        (BTreeMap::new(), false),
        |(mut states, code), (path, state)| {
            if let Some(slug) = post_slug(path) {
                states.insert(slug.to_string(), state);
            }
            (states, code || is_code_path(path))
        },
    );

    let of_state = |wanted: fn(&SlugState) -> bool| {
        states
            .iter()
            .filter(|(_, state)| wanted(state))
            .map(|(slug, _)| slug.clone())
            .collect()
    };
    let set = PublishSet {
        changed: of_state(|state| matches!(state, SlugState::Changed)),
        removed: of_state(|state| matches!(state, SlugState::Removed)),
    };

    if code {
        PushClass::Code(set)
    } else if set.is_empty() {
        PushClass::Ignore
    } else {
        PushClass::ContentOnly(set)
    }
}

/// A path that changes the deployed artifact or its build: any Rust source
/// (including co-located per-post `components.rs`, ADR-0004), the workspace
/// crates and app, worker configs, or the CI workflows that deploy them.
pub fn is_code_path(path: &str) -> bool {
    const CODE_ROOTS: [&str; 4] = ["app/", "crates/", "workers/", ".github/workflows/"];
    const CODE_FILES: [&str; 4] = ["Cargo.toml", "Cargo.lock", "justfile", "wrangler.toml"];
    path.ends_with(".rs")
        || CODE_ROOTS.iter().any(|root| path.starts_with(root))
        || CODE_FILES.contains(&path)
}

/// `content/blog/{slug}/index.mdx` → the slug; anything else is not a post
/// source (CONTENT.md's authoring layout).
pub fn post_slug(path: &str) -> Option<&str> {
    let (slug, file) = path.strip_prefix("content/blog/")?.split_once('/')?;
    (file == "index.mdx" && !slug.is_empty()).then_some(slug)
}

/// Repo path of a post source, the inverse of [`post_slug`].
pub fn post_path(slug: &str) -> String {
    format!("content/blog/{slug}/index.mdx")
}

// --- pending stash (user story 31; PRD KV schema `pending`) ---

/// One parked publish awaiting the CI callback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingEntry {
    pub slug: String,
    pub sha: String,
    /// The push removed this post; draining deletes instead of fetching.
    #[serde(default)]
    pub removed: bool,
}

/// Parks `set` at `sha`, superseding any older entry for the same slug —
/// the newest push is the state that should eventually publish.
pub fn merge_pending(prev: Vec<PendingEntry>, set: &PublishSet, sha: &str) -> Vec<PendingEntry> {
    let superseded =
        |slug: &str| set.changed.iter().any(|s| s == slug) || set.removed.iter().any(|s| s == slug);
    let park = |slugs: &[String], removed: bool| {
        slugs
            .iter()
            .map(|slug| PendingEntry {
                slug: slug.clone(),
                sha: sha.to_string(),
                removed,
            })
            .collect::<Vec<_>>()
    };
    prev.into_iter()
        .filter(|entry| !superseded(&entry.slug))
        .chain(park(&set.changed, false))
        .chain(park(&set.removed, true))
        .collect()
}

// --- /publish auth (user story 35) ---

/// CI's callback body: which commit triggered the workflow, on which repo
/// (pending entries carry slugs and SHAs, but not the repo).
#[derive(Debug, Deserialize)]
pub struct PublishRequest {
    pub sha: String,
    /// `owner/repo`, as GitHub API paths want it.
    pub repository: String,
}

/// Only CI may drain `pending`: exact `Bearer <secret>` match. Both sides go
/// through HMAC under a fixed key so `verify_slice`'s constant-time equality
/// compares length-normalized tags — no timing oracle on content or length.
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

// --- drain report (user story 32: the cross-commit retry) ---

/// What draining did with one parked entry.
#[derive(Debug, PartialEq)]
pub enum DrainEntryOutcome {
    Published,
    Removed,
    /// Validation failed — the entry stays parked and retries on the next
    /// CI callback (the cross-commit race, ADR-0007).
    Failed(Vec<Diagnostic>),
}

/// Per-entry outcomes of one `/publish` drain, in pending order.
#[derive(Debug)]
pub struct DrainReport {
    pub outcomes: Vec<(PendingEntry, DrainEntryOutcome)>,
}

impl DrainReport {
    /// Entries to park again — validation failures only (infra failures
    /// abort the drain before any report exists, leaving `pending` intact).
    pub fn retries(&self) -> Vec<PendingEntry> {
        self.outcomes
            .iter()
            .filter(|(_, outcome)| matches!(outcome, DrainEntryOutcome::Failed(_)))
            .map(|(entry, _)| entry.clone())
            .collect()
    }

    /// One `blog/publish` status per pushed SHA, so every parked commit's
    /// page reflects its own content's fate — not just the CI trigger's.
    pub fn statuses(&self) -> Vec<(String, StatusState, String)> {
        let by_sha = self.outcomes.iter().fold(
            BTreeMap::<&str, (PublishSet, Vec<Diagnostic>)>::new(),
            |mut by_sha, (entry, outcome)| {
                let (set, diags) = by_sha.entry(&entry.sha).or_default();
                match outcome {
                    DrainEntryOutcome::Published => set.changed.push(entry.slug.clone()),
                    DrainEntryOutcome::Removed => set.removed.push(entry.slug.clone()),
                    DrainEntryOutcome::Failed(errs) => diags.extend(errs.iter().cloned()),
                }
                by_sha
            },
        );
        by_sha
            .into_iter()
            .map(|(sha, (set, diags))| match diags.is_empty() {
                true => (
                    sha.to_string(),
                    StatusState::Success,
                    success_description(&set),
                ),
                false => (
                    sha.to_string(),
                    StatusState::Failure,
                    format!("{}; parked for retry", failure_description(&diags)),
                ),
            })
            .collect()
    }

    /// The HTTP response body: what landed and what stayed parked.
    pub fn summary(&self) -> String {
        let landed = PublishSet {
            changed: self.slugs_with(|o| matches!(o, DrainEntryOutcome::Published)),
            removed: self.slugs_with(|o| matches!(o, DrainEntryOutcome::Removed)),
        };
        let parked = self.retries().len();
        [
            (!landed.is_empty()).then(|| success_description(&landed)),
            (parked > 0).then(|| format!("{parked} parked for retry")),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("; ")
    }

    fn slugs_with(&self, wanted: fn(&DrainEntryOutcome) -> bool) -> Vec<String> {
        self.outcomes
            .iter()
            .filter(|(_, outcome)| wanted(outcome))
            .map(|(entry, _)| entry.slug.clone())
            .collect()
    }
}

// --- commit status building (user story 12; ADR-0007 amendment) ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StatusState {
    Pending,
    Success,
    Failure,
    Error,
}

/// Body for `POST /repos/{repo}/statuses/{sha}` (Commit Status API — Checks
/// API write access is GitHub-App-only, see ADR-0007's amendment).
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

pub fn success_description(set: &PublishSet) -> String {
    let clause = |verb: &str, slugs: &[String]| {
        (!slugs.is_empty()).then(|| format!("{verb} {}", slugs.join(", ")))
    };
    [
        clause("published", &set.changed),
        clause("removed", &set.removed),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("; ")
}

/// One concise line for the status; full file/line detail stays `blog
/// check`'s job (the API clamps descriptions to ~140 chars).
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

pub fn pending_description(set: &PublishSet) -> String {
    format!(
        "code push: {} content changes parked for CI publish",
        set.len()
    )
}

// --- GitHub API request shapes ---

/// Raw-content fetch for one changed post, pinned to the pushed SHA.
pub fn contents_url(repo: &str, slug: &str, sha: &str) -> String {
    format!(
        "https://api.github.com/repos/{repo}/contents/{}?ref={sha}",
        post_path(slug)
    )
}

pub fn statuses_url(repo: &str, sha: &str) -> String {
    format!("https://api.github.com/repos/{repo}/statuses/{sha}")
}

pub fn dispatch_url(repo: &str) -> String {
    format!("https://api.github.com/repos/{repo}/actions/workflows/{WORKFLOW_FILE}/dispatches")
}

/// `workflow_dispatch` body: run on the pushed branch, carrying the commit
/// SHA so CI can report back on it (`/publish` callback + statuses).
pub fn dispatch_payload(branch: &str, sha: &str) -> String {
    serde_json::json!({ "ref": branch, "inputs": { "sha": sha } }).to_string()
}

// --- cache purge requests (ADR-0008, Slice 8) ---

/// The purge-by-URL API caps each request at 30 files (non-Enterprise).
pub const PURGE_FILES_LIMIT: usize = 30;

pub fn purge_url(zone: &str) -> String {
    format!("https://api.cloudflare.com/client/v4/zones/{zone}/purge_cache")
}

/// Purge-by-URL request bodies for one publish: the plan's URL paths made
/// absolute under the site's origin (purge matches URLs exactly, so this
/// must mirror how the site keys its cache entries), chunked to the API's
/// per-request file cap.
pub fn purge_payloads(origin: &str, paths: &[String]) -> Vec<String> {
    let origin = origin.trim_end_matches('/');
    paths
        .chunks(PURGE_FILES_LIMIT)
        .map(|chunk| {
            let files: Vec<String> = chunk.iter().map(|path| format!("{origin}{path}")).collect();
            serde_json::json!({ "files": files }).to_string()
        })
        .collect()
}
