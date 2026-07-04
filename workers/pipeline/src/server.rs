//! The wasm shim: transport only. Every decision on this path is a pure
//! function in `lib.rs` with native tests; this file just moves bytes
//! between the webhook, the GitHub API, and KV.

use content_ast::IndexEntry;
use content_parser::Diagnostic;
use publish_core::{ParsedPost, PostSource, INDEX_KEY};
use worker::{console_error, Env, Fetch, Headers, Method, Request, RequestInit, Response, Result};

use crate::{
    classify, contents_url, dispatch_payload, dispatch_url, failure_description, manifest,
    merge_pending, pending_description, post_path, status_payload, statuses_url,
    success_description, verify_publish_auth, verify_signature, DrainEntryOutcome, DrainReport,
    PendingEntry, PublishRequest, PublishSet, PushClass, PushEvent, StatusState, PENDING_KEY,
    STATUS_CONTEXT,
};

/// Same namespace the site worker reads (wrangler.toml in this directory).
const KV_BINDING: &str = "BLOG";
const WEBHOOK_SECRET: &str = "GITHUB_WEBHOOK_SECRET";
const GITHUB_TOKEN: &str = "GITHUB_TOKEN";
/// Shared with CI (an Actions secret): authenticates `/publish` callbacks.
const PUBLISH_SECRET: &str = "PUBLISH_SHARED_SECRET";
/// GitHub rejects API requests without a User-Agent.
const USER_AGENT: &str = "chris-blog-pipeline";

#[worker::event(fetch)]
async fn fetch(mut req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    match (req.method(), req.path().as_str()) {
        (Method::Post, "/webhook") => webhook(&mut req, &env).await,
        (Method::Post, "/publish") => publish_callback(&mut req, &env).await,
        _ => Response::error("not found", 404),
    }
}

async fn webhook(req: &mut Request, env: &Env) -> Result<Response> {
    let body = req.bytes().await?;
    let secret = env.secret(WEBHOOK_SECRET)?.to_string();
    let signature = req.headers().get("x-hub-signature-256")?;
    if !verify_signature(&secret, &body, signature.as_deref()) {
        return Response::error("invalid webhook signature", 401);
    }
    match req.headers().get("x-github-event")?.as_deref() {
        Some("push") => {}
        Some("ping") => return Response::ok("pong"),
        _ => return Response::ok("ignored: not a push event"),
    }
    let Ok(event) = serde_json::from_slice::<PushEvent>(&body) else {
        return Response::error("unrecognized push payload", 400);
    };
    if event.deleted || !event.is_default_branch() {
        return Response::ok("ignored: not a default-branch push");
    }
    match classify(&event.commits) {
        PushClass::Ignore => Response::ok("ignored: no content or code changes"),
        PushClass::Code(set) => park_pending(env, &event, &set).await,
        PushClass::ContentOnly(set) => publish(env, &event, &set).await,
    }
}

/// Code path (ADR-0007): deploy must precede publish, so the set is parked
/// under `pending` and CI is dispatched; its `/publish` callback drains.
async fn park_pending(env: &Env, event: &PushEvent, set: &PublishSet) -> Result<Response> {
    let kv = env.kv(KV_BINDING)?;
    let prev: Vec<PendingEntry> = kv.get(PENDING_KEY).json().await?.unwrap_or_default();
    let pending = merge_pending(prev, set, &event.after);
    kv.put(PENDING_KEY, serde_json::to_string(&pending)?)?
        .execute()
        .await?;
    let repo = &event.repository.full_name;
    if let Err(err) = dispatch_workflow(env, event).await {
        // The park is idempotent (merge supersedes), so the 500 makes GitHub
        // webhook redelivery the retry path for a failed dispatch.
        console_error!("workflow dispatch for {} failed: {err}", event.after);
        post_status(
            env,
            repo,
            &event.after,
            StatusState::Error,
            "could not start the CI publish — see worker logs",
        )
        .await;
        return Response::error("workflow dispatch failed", 500);
    }
    let description = pending_description(set);
    post_status(env, repo, &event.after, StatusState::Pending, &description).await;
    Response::ok(description)
}

/// Fires the publish workflow on the pushed branch with the commit context.
async fn dispatch_workflow(env: &Env, event: &PushEvent) -> std::result::Result<(), String> {
    let url = dispatch_url(&event.repository.full_name);
    let body = dispatch_payload(&event.repository.default_branch, &event.after);
    let response = github(
        env,
        Method::Post,
        &url,
        "application/vnd.github+json",
        Some(body),
    )
    .await
    .map_err(|err| err.to_string())?;
    match response.status_code() {
        204 => Ok(()),
        status => Err(format!("workflow dispatch returned {status}")),
    }
}

enum PublishError {
    /// Content failed validation — the author's problem, reported on the commit.
    Invalid(Vec<Diagnostic>),
    /// KV/GitHub transport failed — the system's problem, a 500.
    Infra(String),
}

/// Webhook fast path: fetch → check → plan → KV, then report on the commit.
async fn publish(env: &Env, event: &PushEvent, set: &PublishSet) -> Result<Response> {
    let repo = &event.repository.full_name;
    match run_publish(env, event, set).await {
        Ok(()) => {
            let description = success_description(set);
            post_status(env, repo, &event.after, StatusState::Success, &description).await;
            Response::ok(description)
        }
        Err(PublishError::Invalid(diags)) => {
            let description = failure_description(&diags);
            post_status(env, repo, &event.after, StatusState::Failure, &description).await;
            Response::ok(format!("publish failed: {description}"))
        }
        Err(PublishError::Infra(err)) => {
            console_error!("publish of {} failed: {err}", event.after);
            post_status(
                env,
                repo,
                &event.after,
                StatusState::Error,
                "publish error — see worker logs",
            )
            .await;
            Response::error("publish error", 500)
        }
    }
}

async fn run_publish(
    env: &Env,
    event: &PushEvent,
    set: &PublishSet,
) -> std::result::Result<(), PublishError> {
    let infra = |err: &dyn std::fmt::Display| PublishError::Infra(err.to_string());
    let repo = &event.repository.full_name;

    let mut sources = Vec::new();
    for slug in &set.changed {
        let source = fetch_content(env, repo, slug, &event.after)
            .await
            .map_err(PublishError::Infra)?
            .ok_or_else(|| {
                PublishError::Infra(format!("contents fetch for {slug} returned 404"))
            })?;
        sources.push(PostSource {
            slug: slug.clone(),
            file: post_path(slug),
            source,
        });
    }
    let parsed = publish_core::check(&sources, &manifest()).map_err(PublishError::Invalid)?;

    let kv = env.kv(KV_BINDING).map_err(|err| infra(&err))?;
    apply_plan(&kv, &parsed, &set.removed)
        .await
        .map_err(PublishError::Infra)?;
    Ok(())
}

/// Merge one publish plan into the stored index and apply it to KV: post +
/// index writes, then removed-post deletes, then purge. Shared by the
/// webhook fast path and the `/publish` drain so the two can't drift on
/// index-merge or KV ordering.
async fn apply_plan(
    kv: &worker::kv::KvStore,
    published: &[ParsedPost],
    removed: &[String],
) -> std::result::Result<(), String> {
    let prev: Vec<IndexEntry> = kv
        .get(INDEX_KEY)
        .json()
        .await
        .map_err(|err| err.to_string())?
        .unwrap_or_default();
    let plan = publish_core::plan(prev, published, removed).map_err(|err| err.to_string())?;
    for write in &plan.writes {
        kv.put(&write.key, write.value.as_str())
            .map_err(|err| err.to_string())?
            .execute()
            .await
            .map_err(|err| err.to_string())?;
    }
    for key in &plan.deletes {
        kv.delete(key).await.map_err(|err| err.to_string())?;
    }
    purge(&plan.purge);
    Ok(())
}

/// Cache purge lands in Slice 8 (ADR-0008); publish correctness does not
/// depend on it — the 7-day TTL backstop bounds staleness meanwhile. The
/// URL paths to invalidate are already computed (`PublishPlan::purge`).
fn purge(_paths: &[String]) {}

/// CI's post-deploy callback (ADR-0007): authenticate, drain `pending`,
/// park validation failures for the next callback, report per pushed SHA.
async fn publish_callback(req: &mut Request, env: &Env) -> Result<Response> {
    let secret = env.secret(PUBLISH_SECRET)?.to_string();
    let auth = req.headers().get("authorization")?;
    if !verify_publish_auth(&secret, auth.as_deref()) {
        return Response::error("unauthorized", 401);
    }
    let Ok(request) = serde_json::from_slice::<PublishRequest>(&req.bytes().await?) else {
        return Response::error("unrecognized publish request", 400);
    };
    let kv = env.kv(KV_BINDING)?;
    let pending: Vec<PendingEntry> = kv.get(PENDING_KEY).json().await?.unwrap_or_default();
    if pending.is_empty() {
        return Response::ok("nothing pending");
    }
    match drain(env, &request.repository, pending).await {
        Ok(report) => {
            for (sha, state, description) in report.statuses() {
                post_status(env, &request.repository, &sha, state, &description).await;
            }
            Response::ok(report.summary())
        }
        Err(err) => {
            // `pending` is untouched on infra failure — the next callback
            // (or a manual re-run of the workflow) retries the whole set.
            console_error!("publish callback for {} failed: {err}", request.sha);
            Response::error("publish error", 500)
        }
    }
}

/// Drains every parked entry: fetch + validate each changed post at its own
/// pushed SHA, apply one merged KV plan, and re-park what failed validation.
async fn drain(
    env: &Env,
    repo: &str,
    pending: Vec<PendingEntry>,
) -> std::result::Result<DrainReport, String> {
    let manifest = manifest();
    let mut outcomes = Vec::new();
    let mut published: Vec<ParsedPost> = Vec::new();
    let mut removed: Vec<String> = Vec::new();
    for entry in pending {
        let outcome = if entry.removed {
            removed.push(entry.slug.clone());
            DrainEntryOutcome::Removed
        } else {
            match fetch_content(env, repo, &entry.slug, &entry.sha).await? {
                Some(source) => {
                    let post = PostSource {
                        slug: entry.slug.clone(),
                        file: post_path(&entry.slug),
                        source,
                    };
                    match publish_core::check(std::slice::from_ref(&post), &manifest) {
                        Ok(mut posts) => {
                            published.extend(posts.pop());
                            DrainEntryOutcome::Published
                        }
                        Err(diags) => DrainEntryOutcome::Failed(diags),
                    }
                }
                // A force-push can orphan a parked entry; parking it again
                // keeps it from wedging the rest until a later push
                // supersedes it (merge_pending).
                None => DrainEntryOutcome::Failed(vec![Diagnostic {
                    message: format!("source not found at {}", entry.sha),
                    file: Some(post_path(&entry.slug)),
                    line: None,
                    column: None,
                }]),
            }
        };
        outcomes.push((entry, outcome));
    }

    let kv = env.kv(KV_BINDING).map_err(|err| err.to_string())?;
    apply_plan(&kv, &published, &removed).await?;

    let report = DrainReport { outcomes };
    let retries = report.retries();
    if retries.is_empty() {
        kv.delete(PENDING_KEY)
            .await
            .map_err(|err| err.to_string())?;
    } else {
        kv.put(
            PENDING_KEY,
            serde_json::to_string(&retries).map_err(|err| err.to_string())?,
        )
        .map_err(|err| err.to_string())?
        .execute()
        .await
        .map_err(|err| err.to_string())?;
    }
    Ok(report)
}

/// Raw post source at the pushed SHA via the contents API. A 404 is its own
/// case: the webhook path treats it as infra, the drain as a validation-like
/// park (a force-push can orphan a pending entry; it must not wedge others).
async fn fetch_content(
    env: &Env,
    repo: &str,
    slug: &str,
    sha: &str,
) -> std::result::Result<Option<String>, String> {
    let url = contents_url(repo, slug, sha);
    let mut response = github(
        env,
        Method::Get,
        &url,
        "application/vnd.github.raw+json",
        None,
    )
    .await
    .map_err(|err| err.to_string())?;
    match response.status_code() {
        200 => response
            .text()
            .await
            .map(Some)
            .map_err(|err| err.to_string()),
        404 => Ok(None),
        status => Err(format!("contents fetch for {slug} returned {status}")),
    }
}

/// Best-effort: a failed status post must not fail an already-applied
/// publish, so it logs loudly instead of propagating (user story 12 still
/// holds — GitHub outages aside, the status lands on every path).
async fn post_status(env: &Env, repo: &str, sha: &str, state: StatusState, description: &str) {
    let url = statuses_url(repo, sha);
    let body = status_payload(state, description);
    let posted = github(
        env,
        Method::Post,
        &url,
        "application/vnd.github+json",
        Some(body),
    )
    .await;
    match posted {
        Ok(response) if response.status_code() == 201 => {}
        Ok(response) => console_error!(
            "{STATUS_CONTEXT} status on {sha} rejected: {}",
            response.status_code()
        ),
        Err(err) => console_error!("{STATUS_CONTEXT} status on {sha} failed: {err}"),
    }
}

async fn github(
    env: &Env,
    method: Method,
    url: &str,
    accept: &str,
    body: Option<String>,
) -> Result<Response> {
    let token = env.secret(GITHUB_TOKEN)?.to_string();
    let headers = Headers::new();
    headers.set("authorization", &format!("Bearer {token}"))?;
    headers.set("user-agent", USER_AGENT)?;
    headers.set("accept", accept)?;
    headers.set("x-github-api-version", "2022-11-28")?;
    if body.is_some() {
        headers.set("content-type", "application/json")?;
    }
    let mut init = RequestInit::new();
    init.with_method(method).with_headers(headers);
    if let Some(body) = body {
        init.with_body(Some(body.into()));
    }
    Fetch::Request(Request::new_with_init(url, &init)?)
        .send()
        .await
}
