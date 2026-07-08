//! Outbound transport: GitHub API, commit statuses, cache purge. No
//! decisions live here.

use worker::{
    console_error, console_log, Env, Fetch, Headers, Method, Request, RequestInit, Response,
};

use crate::{status_payload, statuses_url, StatusState, STATUS_CONTEXT};

const GITHUB_TOKEN: &str = "GITHUB_TOKEN";
/// Service binding to the site worker — only it can reach its own cache.
const SITE_BINDING: &str = "SITE";
/// Shared with the site worker: authenticates `/__purge` calls.
const PURGE_SECRET: &str = "PURGE_SHARED_SECRET";
/// The binding ignores the host; the path selects the site's purge route.
const PURGE_ENDPOINT: &str = "https://site.internal/__purge";
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

/// Best-effort: KV is the truth and the site's 7-day TTL backstops a miss.
/// Workers Cache is private to the site worker, so the purge is a call into
/// it over the service binding, not a Cloudflare API request.
pub(crate) async fn purge_site(env: &Env) {
    if let Err(err) = purge_request(env).await {
        console_error!("cache purge failed (TTL backstop applies): {err}");
    }
}

async fn purge_request(env: &Env) -> std::result::Result<(), String> {
    let site = env
        .service(SITE_BINDING)
        .map_err(|err| format!("{SITE_BINDING} binding: {err}"))?;
    let secret = env
        .secret(PURGE_SECRET)
        .map_err(|err| format!("{PURGE_SECRET} secret: {err}"))?;
    let headers = Headers::new();
    headers
        .set("authorization", &format!("Bearer {secret}"))
        .map_err(|err| err.to_string())?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post).with_headers(headers);
    let request = Request::new_with_init(PURGE_ENDPOINT, &init).map_err(|err| err.to_string())?;
    let response = site
        .fetch_request(request)
        .await
        .map_err(|err| err.to_string())?;
    let status = response.status().as_u16();
    if status != 200 {
        return Err(format!("{PURGE_ENDPOINT} returned {status}"));
    }
    console_log!("site cache purged");
    Ok(())
}
