//! The wasm shim: transport only. Every decision on this path is a pure
//! function in `lib.rs` with native tests; this file just moves bytes
//! between the webhook, the GitHub API, and KV.

use content_ast::IndexEntry;
use content_parser::Diagnostic;
use publish_core::{PostSource, INDEX_KEY};
use worker::{console_error, Env, Fetch, Headers, Method, Request, RequestInit, Response, Result};

use crate::{
    classify, contents_url, failure_description, manifest, merge_pending, pending_description,
    post_path, status_payload, statuses_url, success_description, verify_signature, PendingEntry,
    PublishSet, PushClass, PushEvent, StatusState, PENDING_KEY, STATUS_CONTEXT,
};

/// Same namespace the site worker reads (wrangler.toml in this directory).
const KV_BINDING: &str = "BLOG";
const WEBHOOK_SECRET: &str = "GITHUB_WEBHOOK_SECRET";
const GITHUB_TOKEN: &str = "GITHUB_TOKEN";
/// GitHub rejects API requests without a User-Agent.
const USER_AGENT: &str = "chris-blog-pipeline";

#[worker::event(fetch)]
async fn fetch(mut req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    match (req.method(), req.path().as_str()) {
        (Method::Post, "/webhook") => webhook(&mut req, &env).await,
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
/// under `pending` for the CI callback (Slice 7) and nothing is published.
async fn park_pending(env: &Env, event: &PushEvent, set: &PublishSet) -> Result<Response> {
    let kv = env.kv(KV_BINDING)?;
    let prev: Vec<PendingEntry> = kv.get(PENDING_KEY).json().await?.unwrap_or_default();
    let pending = merge_pending(prev, set, &event.after);
    kv.put(PENDING_KEY, serde_json::to_string(&pending)?)?
        .execute()
        .await?;
    let description = pending_description(set);
    post_status(env, event, StatusState::Pending, &description).await;
    Response::ok(description)
}

enum PublishError {
    /// Content failed validation — the author's problem, reported on the commit.
    Invalid(Vec<Diagnostic>),
    /// KV/GitHub transport failed — the system's problem, a 500.
    Infra(String),
}

/// Webhook fast path: fetch → check → plan → KV, then report on the commit.
async fn publish(env: &Env, event: &PushEvent, set: &PublishSet) -> Result<Response> {
    match run_publish(env, event, set).await {
        Ok(()) => {
            let description = success_description(set);
            post_status(env, event, StatusState::Success, &description).await;
            Response::ok(description)
        }
        Err(PublishError::Invalid(diags)) => {
            let description = failure_description(&diags);
            post_status(env, event, StatusState::Failure, &description).await;
            Response::ok(format!("publish failed: {description}"))
        }
        Err(PublishError::Infra(err)) => {
            console_error!("publish of {} failed: {err}", event.after);
            post_status(
                env,
                event,
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
            .map_err(PublishError::Infra)?;
        sources.push(PostSource {
            slug: slug.clone(),
            file: post_path(slug),
            source,
        });
    }
    let parsed = publish_core::check(&sources, &manifest()).map_err(PublishError::Invalid)?;

    let kv = env.kv(KV_BINDING).map_err(|err| infra(&err))?;
    let prev: Vec<IndexEntry> = kv
        .get(INDEX_KEY)
        .json()
        .await
        .map_err(|err| infra(&err))?
        .unwrap_or_default();
    let plan = publish_core::plan(prev, &parsed, &set.removed).map_err(|err| infra(&err))?;
    for write in &plan.writes {
        kv.put(&write.key, write.value.as_str())
            .map_err(|err| infra(&err))?
            .execute()
            .await
            .map_err(|err| infra(&err))?;
    }
    for key in &plan.deletes {
        kv.delete(key).await.map_err(|err| infra(&err))?;
    }
    purge(set);
    Ok(())
}

/// Cache purge lands in Slice 8 (ADR-0008); publish correctness does not
/// depend on it — the 7-day TTL backstop bounds staleness meanwhile.
fn purge(_set: &PublishSet) {}

/// Raw post source at the pushed SHA via the contents API.
async fn fetch_content(
    env: &Env,
    repo: &str,
    slug: &str,
    sha: &str,
) -> std::result::Result<String, String> {
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
        200 => response.text().await.map_err(|err| err.to_string()),
        status => Err(format!("contents fetch for {slug} returned {status}")),
    }
}

/// Best-effort: a failed status post must not fail an already-applied
/// publish, so it logs loudly instead of propagating (user story 12 still
/// holds — GitHub outages aside, the status lands on every path).
async fn post_status(env: &Env, event: &PushEvent, state: StatusState, description: &str) {
    let url = statuses_url(&event.repository.full_name, &event.after);
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
            "{STATUS_CONTEXT} status on {} rejected: {}",
            event.after,
            response.status_code()
        ),
        Err(err) => console_error!("{STATUS_CONTEXT} status on {} failed: {err}", event.after),
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
