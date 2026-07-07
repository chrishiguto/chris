//! Transport-only routes; decisions are pure functions in `lib.rs`.

use worker::{console_error, Env, Method, Request, RequestInit, Response, Result};

use crate::net::post_status;
use crate::{
    coordinator, decide_push, dispatch_payload, dispatch_url, verify_publish_auth,
    verify_signature, PublishRequest, PushEvent, ReconcileConfig, StatusState, WebhookAction,
};

const WEBHOOK_SECRET: &str = "GITHUB_WEBHOOK_SECRET";
/// Shared with CI (an Actions secret): authenticates `/publish` callbacks.
const PUBLISH_SECRET: &str = "PUBLISH_SHARED_SECRET";

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
    match decide_push(&event) {
        WebhookAction::Ignore(reason) => Response::ok(reason),
        // Fast path: the coordinator posts the status itself. A failed
        // trigger 500s so webhook redelivery retries.
        WebhookAction::Reconcile(config) => {
            trigger_reconcile(env, &config).await?;
            Response::ok("content push: reconcile scheduled")
        }
        WebhookAction::DispatchCi { description } => {
            let repo = &event.repository.full_name;
            if let Err(err) = dispatch_workflow(env, &event).await {
                // Dispatch is idempotent; the 500 makes redelivery the retry.
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
            post_status(env, repo, &event.after, StatusState::Pending, &description).await;
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
    };
    trigger_reconcile(env, &config).await?;
    Response::ok("reconcile scheduled")
}

/// Pokes the coordinator; returns as soon as the trigger is stored — nobody
/// waits on GitHub inside a webhook delivery window.
async fn trigger_reconcile(env: &Env, config: &ReconcileConfig) -> Result<()> {
    let namespace = env.durable_object(coordinator::COORDINATOR_BINDING)?;
    let stub = namespace
        .id_from_name(coordinator::COORDINATOR_NAME)?
        .get_stub()?;
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
