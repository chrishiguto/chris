//! Transport-only routes; decisions are pure functions in `lib.rs`.

use worker::{Env, Method, Request, RequestInit, Response, Result};

use crate::{coordinator, ReconcileConfig};
use authn::verify_bearer;

// TODO: evaluate Cloudflare Access service tokens for this caller, and
// Secrets Store for the shared secret.
/// Shared with CI (an Actions secret): authenticates `/publish` calls.
const PUBLISH_SECRET: &str = "PUBLISH_SHARED_SECRET";

#[worker::event(fetch)]
async fn fetch(mut req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    match (req.method(), req.path().as_str()) {
        (Method::Post, "/publish") => publish(&mut req, &env).await,
        _ => Response::error("not found", 404),
    }
}

/// CI calls this after any deploy: run one reconcile-to-HEAD and return its
/// outcome so the Actions run reflects what happened. The reconcile is
/// synchronous — an Actions step has no delivery window to protect.
async fn publish(req: &mut Request, env: &Env) -> Result<Response> {
    let secret = env.secret(PUBLISH_SECRET)?.to_string();
    let auth = req.headers().get("authorization")?;
    if !verify_bearer(&secret, auth.as_deref()) {
        return Response::error("unauthorized", 401);
    }
    let Ok(config) = serde_json::from_slice::<ReconcileConfig>(&req.bytes().await?) else {
        return Response::error("unrecognized publish request", 400);
    };
    if config.branch.is_empty() {
        return Response::error("publish request carries no branch", 400);
    }

    // Hand the reconcile to the one coordinator instance and pass its response
    // straight back — the outcome JSON on success, the error on 500. The
    // single-instance DO serializes overlapping calls.
    let stub = env
        .durable_object(coordinator::COORDINATOR_BINDING)?
        .id_from_name(coordinator::COORDINATOR_NAME)?
        .get_stub()?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_body(Some(serde_json::to_string(&config)?.into()));
    let request = Request::new_with_init("https://coordinator/reconcile", &init)?;
    stub.fetch_with_request(request).await
}
