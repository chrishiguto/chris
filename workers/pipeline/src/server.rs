//! Transport-only routes; decisions are pure functions in `lib.rs`.

use worker::{console_error, Env, Method, Request, RequestInit, Response, Result};

use crate::net::post_status;
use crate::{
    coordinator, decide_push, dispatch_payload, dispatch_url, render_status_page, status_page_sha,
    verify_publish_auth, verify_signature, PublishRequest, PushEvent, ReconcileConfig,
    ReconcileRecord, StatusState, WebhookAction, STATUS_CONTEXT,
};

const WEBHOOK_SECRET: &str = "GITHUB_WEBHOOK_SECRET";
/// Shared with CI (an Actions secret): authenticates `/publish` callbacks.
const PUBLISH_SECRET: &str = "PUBLISH_SHARED_SECRET";

#[worker::event(fetch)]
async fn fetch(mut req: Request, env: Env, ctx: worker::Context) -> Result<Response> {
    match (req.method(), req.path().as_str()) {
        (Method::Post, "/webhook") => webhook(&mut req, &env, &ctx).await,
        (Method::Post, "/publish") => publish_callback(&mut req, &env).await,
        (Method::Get, path) => match status_page_sha(path) {
            Some(sha) => status_page(&env, sha).await,
            None => Response::error("not found", 404),
        },
        _ => Response::error("not found", 404),
    }
}

/// The commit status's Details page: the coordinator's stored record for
/// one reconciled sha, rendered as plain HTML.
async fn status_page(env: &Env, sha: &str) -> Result<Response> {
    let stub = coordinator_stub(env)?;
    let mut response = stub
        .fetch_with_str(&format!("https://coordinator/record/{sha}"))
        .await?;
    if response.status_code() != 200 {
        return Response::error("no publish record for this commit", 404);
    }
    let record: ReconcileRecord = response.json().await?;
    Response::from_html(render_status_page(&record))
}

/// This worker's public origin, so statuses can link back to `/status/{sha}`.
fn request_origin(req: &Request) -> String {
    req.url()
        .map(|url| url.origin().ascii_serialization())
        .unwrap_or_default()
}

async fn webhook(req: &mut Request, env: &Env, ctx: &worker::Context) -> Result<Response> {
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
    match decide_push(&event) {
        WebhookAction::Ignore(reason) => Response::ok(reason),
        // Fast path: the coordinator posts the status itself. A failed
        // trigger 500s so webhook redelivery retries.
        WebhookAction::Reconcile(mut config) => {
            config.status_origin = request_origin(req);
            trigger_reconcile(env, &config).await?;
            Response::ok("content push: reconcile scheduled")
        }
        // Dry run off the delivery window; the status is the only output.
        WebhookAction::CheckBranch { sha } => {
            let env = env.clone();
            let repo = event.repository.full_name.clone();
            ctx.wait_until(async move {
                coordinator::check_branch_head(&env, &repo, &sha).await;
            });
            Response::ok("branch content push: check scheduled")
        }
        WebhookAction::DispatchCi { description } => {
            let repo = &event.repository.full_name;
            if let Err(err) = dispatch_workflow(env, &event).await {
                // Dispatch is idempotent; the 500 makes redelivery the retry.
                console_error!("workflow dispatch for {} failed: {err}", event.after);
                post_status(
                    env,
                    STATUS_CONTEXT,
                    repo,
                    &event.after,
                    StatusState::Error,
                    "could not start the CI publish — see worker logs",
                    None,
                )
                .await;
                return Response::error("workflow dispatch failed", 500);
            }
            post_status(
                env,
                STATUS_CONTEXT,
                repo,
                &event.after,
                StatusState::Pending,
                &description,
                None,
            )
            .await;
            Response::ok(description)
        }
    }
}

/// CI's post-deploy callback: the deploy landed, so HEAD now validates
/// against the freshly deployed manifest.
async fn publish_callback(req: &mut Request, env: &Env) -> Result<Response> {
    let secret = env.secret(PUBLISH_SECRET)?.to_string();
    let auth = req.headers().get("authorization")?;
    if !verify_publish_auth(&secret, auth.as_deref()) {
        return Response::error("unauthorized", 401);
    }
    let Ok(request) = serde_json::from_slice::<PublishRequest>(&req.bytes().await?) else {
        return Response::error("unrecognized publish request", 400);
    };
    if request.branch.is_empty() {
        return Response::error("publish request carries no branch", 400);
    }
    let config = ReconcileConfig {
        repository: request.repository,
        branch: request.branch,
        status_origin: request_origin(req),
    };
    trigger_reconcile(env, &config).await?;
    Response::ok("reconcile scheduled")
}

fn coordinator_stub(env: &Env) -> Result<worker::Stub> {
    env.durable_object(coordinator::COORDINATOR_BINDING)?
        .id_from_name(coordinator::COORDINATOR_NAME)?
        .get_stub()
}

/// Pokes the coordinator; returns as soon as the trigger is stored — nobody
/// waits on GitHub inside a webhook delivery window.
async fn trigger_reconcile(env: &Env, config: &ReconcileConfig) -> Result<()> {
    let stub = coordinator_stub(env)?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_body(Some(serde_json::to_string(config)?.into()));
    let request = Request::new_with_init("https://coordinator/trigger", &init)?;
    let response = stub.fetch_with_request(request).await?;
    match response.status_code() {
        200 => Ok(()),
        status => Err(worker::Error::RustError(format!(
            "coordinator trigger returned {status}"
        ))),
    }
}

async fn dispatch_workflow(env: &Env, event: &PushEvent) -> std::result::Result<(), String> {
    let url = dispatch_url(&event.repository.full_name);
    let body = dispatch_payload(&event.repository.default_branch, &event.after);
    let response = crate::net::github(
        env,
        Method::Post,
        &url,
        "application/vnd.github+json",
        Some(body),
    )
    .await?;
    match response.status_code() {
        204 => Ok(()),
        status => Err(format!("workflow dispatch returned {status}")),
    }
}
