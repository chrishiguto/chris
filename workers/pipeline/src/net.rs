//! Outbound transport: GitHub API, commit statuses, cache purge. No
//! decisions live here.

use worker::{console_error, Env, Fetch, Headers, Method, Request, RequestInit, Response};

use crate::{purge_body, purge_url, status_payload, statuses_url, StatusState, STATUS_CONTEXT};

const GITHUB_TOKEN: &str = "GITHUB_TOKEN";
/// Zone of the site's custom domain; empty until one exists.
const ZONE_ID_VAR: &str = "CLOUDFLARE_ZONE_ID";
/// Absolute origin the site serves on — purge-by-URL needs full URLs.
const SITE_ORIGIN_VAR: &str = "SITE_ORIGIN";
/// Secret scoped to Zone → Cache Purge.
const PURGE_TOKEN: &str = "CLOUDFLARE_PURGE_TOKEN";
/// GitHub rejects API requests without a User-Agent.
const USER_AGENT: &str = "chris-blog-pipeline";

async fn send(
    method: Method,
    url: &str,
    headers: &[(&str, &str)],
    body: Option<String>,
) -> std::result::Result<Response, String> {
    let assembled = Headers::new();
    for (name, value) in headers {
        assembled.set(name, value).map_err(|err| err.to_string())?;
    }
    let mut init = RequestInit::new();
    init.with_method(method).with_headers(assembled);
    if let Some(body) = body {
        init.with_body(Some(body.into()));
    }
    let request = Request::new_with_init(url, &init).map_err(|err| err.to_string())?;
    Fetch::Request(request)
        .send()
        .await
        .map_err(|err| err.to_string())
}

fn expect_status(response: &Response, want: u16, url: &str) -> std::result::Result<(), String> {
    let status = response.status_code();
    (status == want)
        .then_some(())
        .ok_or_else(|| format!("{url} returned {status}"))
}

pub(crate) async fn github(
    env: &Env,
    method: Method,
    url: &str,
    accept: &str,
    body: Option<String>,
) -> std::result::Result<Response, String> {
    let token = env
        .secret(GITHUB_TOKEN)
        .map_err(|err| err.to_string())?
        .to_string();
    let auth = format!("Bearer {token}");
    let mut headers = vec![
        ("authorization", auth.as_str()),
        ("user-agent", USER_AGENT),
        ("accept", accept),
        ("x-github-api-version", "2022-11-28"),
    ];
    if body.is_some() {
        headers.push(("content-type", "application/json"));
    }
    send(method, url, &headers, body).await
}

pub(crate) async fn github_json(
    env: &Env,
    url: &str,
) -> std::result::Result<serde_json::Value, String> {
    let mut response = github(env, Method::Get, url, "application/vnd.github+json", None).await?;
    expect_status(&response, 200, url)?;
    let text = response.text().await.map_err(|err| err.to_string())?;
    serde_json::from_str(&text).map_err(|err| format!("{url} returned non-JSON: {err}"))
}

/// Raw post source via the contents API; 404 is `Ok(None)`.
pub(crate) async fn fetch_content(
    env: &Env,
    url: &str,
) -> std::result::Result<Option<String>, String> {
    let mut response = github(
        env,
        Method::Get,
        url,
        "application/vnd.github.raw+json",
        None,
    )
    .await?;
    match response.status_code() {
        200 => response
            .text()
            .await
            .map(Some)
            .map_err(|err| err.to_string()),
        404 => Ok(None),
        status => Err(format!("{url} returned {status}")),
    }
}

/// Best-effort: a failed status post must not fail an applied publish.
/// Returns whether the status landed so dedup callers can retry a lost one.
pub(crate) async fn post_status(
    env: &Env,
    repo: &str,
    sha: &str,
    state: StatusState,
    description: &str,
) -> bool {
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
        Ok(response) if response.status_code() == 201 => true,
        Ok(response) => {
            console_error!(
                "{STATUS_CONTEXT} status on {sha} rejected: {}",
                response.status_code()
            );
            false
        }
        Err(err) => {
            console_error!("{STATUS_CONTEXT} status on {sha} failed: {err}");
            false
        }
    }
}

/// Best-effort purge-by-URL: KV is the truth and the site's 7-day TTL
/// backstops a miss. Skips until a custom domain's zone and origin exist.
pub(crate) async fn purge(env: &Env, plan: &publish::SnapshotPlan) {
    let var = |name: &str| {
        env.var(name)
            .ok()
            .map(|value| value.to_string())
            .filter(|value| !value.is_empty())
    };
    let (Some(zone), Some(origin)) = (var(ZONE_ID_VAR), var(SITE_ORIGIN_VAR)) else {
        worker::console_log!(
            "cache purge skipped: {ZONE_ID_VAR}/{SITE_ORIGIN_VAR} not configured (no custom domain yet)"
        );
        return;
    };
    let Ok(token) = env.secret(PURGE_TOKEN) else {
        console_error!(
            "cache purge skipped: {PURGE_TOKEN} secret missing while a zone is configured"
        );
        return;
    };
    let url = purge_url(&zone);
    let token = token.to_string();
    let requests = plan.purge_chunks(&origin).into_iter().map(|chunk| {
        let (url, token) = (&url, &token);
        async move {
            if let Err(err) = purge_request(url, token, purge_body(&chunk)).await {
                console_error!("cache purge failed (TTL backstop applies): {err}");
            }
        }
    });
    futures_util::future::join_all(requests).await;
}

async fn purge_request(url: &str, token: &str, body: String) -> std::result::Result<(), String> {
    let auth = format!("Bearer {token}");
    let headers = [
        ("authorization", auth.as_str()),
        ("content-type", "application/json"),
    ];
    let response = send(Method::Post, url, &headers, Some(body)).await?;
    expect_status(&response, 200, url)
}
