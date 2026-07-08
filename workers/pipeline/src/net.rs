//! Outbound transport: GitHub API (content reads) and cache purge. No
//! decisions live here.

use worker::{
    console_error, console_log, Env, Fetch, Headers, Method, Request, RequestInit, Response,
};

const GITHUB_TOKEN: &str = "GITHUB_TOKEN";
/// Service binding to the site worker — only it can reach its own cache.
const SITE_BINDING: &str = "SITE";
/// Shared with the site worker: authenticates `/__purge` calls.
const PURGE_SECRET: &str = "PURGE_SHARED_SECRET";
/// The binding ignores the host; the path selects the site's purge route.
const PURGE_ENDPOINT: &str = "https://site.internal/__purge";
/// GitHub rejects API requests without a User-Agent.
const USER_AGENT: &str = "chris-blog-pipeline";

fn expect_status(response: &Response, want: u16, url: &str) -> std::result::Result<(), String> {
    let status = response.status_code();
    (status == want)
        .then_some(())
        .ok_or_else(|| format!("{url} returned {status}"))
}

/// Authenticated GitHub API GET. Every call the reconcile makes is a read —
/// HEAD ref, tree listing, raw content — so there is never a body to send.
async fn github_get(env: &Env, url: &str, accept: &str) -> std::result::Result<Response, String> {
    let token = env
        .secret(GITHUB_TOKEN)
        .map_err(|err| err.to_string())?
        .to_string();
    let auth = format!("Bearer {token}");
    let headers = Headers::new();
    for (name, value) in [
        ("authorization", auth.as_str()),
        ("user-agent", USER_AGENT),
        ("accept", accept),
        ("x-github-api-version", "2022-11-28"),
    ] {
        headers.set(name, value).map_err(|err| err.to_string())?;
    }
    let mut init = RequestInit::new();
    init.with_method(Method::Get).with_headers(headers);
    let request = Request::new_with_init(url, &init).map_err(|err| err.to_string())?;
    Fetch::Request(request)
        .send()
        .await
        .map_err(|err| err.to_string())
}

pub(crate) async fn github_json(
    env: &Env,
    url: &str,
) -> std::result::Result<serde_json::Value, String> {
    let mut response = github_get(env, url, "application/vnd.github+json").await?;
    expect_status(&response, 200, url)?;
    let text = response.text().await.map_err(|err| err.to_string())?;
    serde_json::from_str(&text).map_err(|err| format!("{url} returned non-JSON: {err}"))
}

/// Raw post source via the contents API; 404 is `Ok(None)`.
pub(crate) async fn fetch_content(
    env: &Env,
    url: &str,
) -> std::result::Result<Option<String>, String> {
    let mut response = github_get(env, url, "application/vnd.github.raw+json").await?;
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

/// Purges exactly `tags` from the site's cache; returns whether it landed —
/// KV already flipped, so the caller reports a failure instead of unwinding
/// the publish. Workers Cache is private to the site worker, so the purge is
/// a call into it over the service binding, not a Cloudflare API request.
pub(crate) async fn purge_site(env: &Env, tags: &[String]) -> bool {
    match purge_request(env, tags).await {
        Ok(()) => true,
        Err(err) => {
            console_error!("cache purge failed (TTL backstop applies): {err}");
            false
        }
    }
}

async fn purge_request(env: &Env, tags: &[String]) -> std::result::Result<(), String> {
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
    headers
        .set("content-type", "application/json")
        .map_err(|err| err.to_string())?;
    let body = serde_json::json!({ "tags": tags }).to_string();
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(body.into()));
    let request = Request::new_with_init(PURGE_ENDPOINT, &init).map_err(|err| err.to_string())?;
    let response = site
        .fetch_request(request)
        .await
        .map_err(|err| err.to_string())?;
    let status = response.status().as_u16();
    if status != 200 {
        return Err(format!("{PURGE_ENDPOINT} returned {status}"));
    }
    console_log!("site cache purged: {}", tags.join(", "));
    Ok(())
}
