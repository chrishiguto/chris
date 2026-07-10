//! Worker shim: routing and KV reads. Workers Cache fronts the fetch
//! handler at the platform layer, so a hit never reaches this code.
pub mod cache;
pub mod feeds;
#[cfg(feature = "ssr")]
mod purge;

#[cfg(feature = "ssr")]
mod server {
    use crate::{cache, feeds, purge};
    use app::{app::shell, listing::IndexData, post::PostData};
    use authn::verify_bearer;
    use axum::{
        body::Body,
        extract::{FromRef, Path, State},
        http::{
            header::{AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH},
            HeaderName, HeaderValue, Request, Response, StatusCode,
        },
        response::IntoResponse,
        routing::{get, post},
        Router,
    };
    use content::{
        index_key_at, post_key_at, CurrentPointer, Document, IndexEntry, CURRENT_KEY,
        LISTING_PAGES, RSS_PATH, SITEMAP_PATH, SITE_TAG, STATIC_PAGES,
    };
    use leptos::prelude::*;
    use tower_service::Service;
    use worker::{console_error, Env};

    const KV_BINDING: &str = "BLOG";
    /// Axum `{param}` form of `content::post_path`.
    const POST_ROUTE: &str = "/posts/{slug}";
    /// The pipeline's purge hook; Workers Cache is private to this worker.
    const PURGE_ROUTE: &str = "/__purge";
    // TODO: evaluate Cloudflare Secrets Store for this cross-worker shared secret.
    /// Shared with the pipeline worker: authenticates purge calls.
    const PURGE_SECRET: &str = "PURGE_SHARED_SECRET";
    /// A purge body is a short tag list; anything bigger is malformed.
    const PURGE_BODY_LIMIT: usize = 16 * 1024;

    #[derive(Clone)]
    struct AppState {
        options: LeptosOptions,
        env: Env,
    }

    impl FromRef<AppState> for LeptosOptions {
        fn from_ref(state: &AppState) -> Self {
            state.options.clone()
        }
    }

    #[worker::event(fetch)]
    async fn fetch(
        req: worker::HttpRequest,
        env: Env,
        _ctx: worker::Context,
    ) -> worker::Result<Response<Body>> {
        // The async renderer spawns through the global executor; registering
        // it is once-per-isolate, so later requests hit `AlreadySet`.
        _ = any_spawner::Executor::init_wasm_bindgen();
        // Isolates outlive requests: cache the config; the router captures
        // `env`, so build it per-request.
        thread_local! {
            static OPTIONS: LeptosOptions = get_configuration(None).unwrap().leptos_options;
        }
        let options = OPTIONS.with(Clone::clone);
        let state = AppState { options, env };

        // Captured before the router consumes the request.
        let if_none_match = req
            .headers()
            .get(IF_NONE_MATCH)
            .and_then(|value| value.to_str().ok())
            .map(String::from);

        // One handler serves all listing pages and another all static
        // pages, so a page list and the routes can't diverge; the leptos
        // router picks the page from the URL.
        let router = Router::new()
            .route(POST_ROUTE, get(post_page))
            .route(RSS_PATH, get(feed_xml))
            .route(SITEMAP_PATH, get(sitemap_xml))
            .route(PURGE_ROUTE, post(purge_route));
        let router = LISTING_PAGES
            .iter()
            .fold(router, |r, path| r.route(path, get(listing_page)));
        let mut router = STATIC_PAGES
            .iter()
            .fold(router, |r, path| r.route(path, get(static_page)))
            .fallback(not_found_page)
            .with_state(state);

        let mut response = router.call(req).await?;
        // No explicit Cache-Control means no-store: drafts, 404s, and errors
        // must never gain Workers Cache's heuristic freshness.
        if !response.headers().contains_key(CACHE_CONTROL) {
            response
                .headers_mut()
                .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
        }
        Ok(revalidated(response, if_none_match.as_deref()))
    }

    /// Purges the requested cache tags (`{"tags":[...]}`; no body means the
    /// site-wide tag). Failures answer loudly with a 502 so callers can
    /// report them.
    #[worker::send]
    async fn purge_route(State(state): State<AppState>, req: Request<Body>) -> Response<Body> {
        // A missing secret is a server misconfiguration (500), not a rejected
        // caller (401) — the same split the pipeline's authed routes make.
        let Ok(secret) = state.env.secret(PURGE_SECRET) else {
            console_error!("{PURGE_SECRET} secret missing — cannot authenticate purge");
            return (StatusCode::INTERNAL_SERVER_ERROR, "purge misconfigured").into_response();
        };
        let (parts, body) = req.into_parts();
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok());
        if !verify_bearer(&secret.to_string(), header) {
            return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
        }
        let Ok(body) = axum::body::to_bytes(body, PURGE_BODY_LIMIT).await else {
            return (StatusCode::BAD_REQUEST, "unreadable purge body").into_response();
        };
        let tags = match cache::parse_purge_body(&body) {
            Ok(tags) => tags,
            Err(err) => return (StatusCode::BAD_REQUEST, err).into_response(),
        };
        match purge::purge_tags(&tags).await {
            Ok(()) => (StatusCode::OK, "purged").into_response(),
            Err(err) => {
                console_error!("cache purge failed: {err}");
                (StatusCode::BAD_GATEWAY, "purge failed").into_response()
            }
        }
    }

    /// Opts a response into Workers Cache — `s-maxage` stores at the edge,
    /// `max-age=0` keeps browsers revalidating — plus the snapshot-sha ETag
    /// and the `Cache-Tag` scopes purges select on. Fail-closed: tags are a
    /// purge's only handle on a cached entry, so a tag set that can't be a
    /// header leaves the response uncached rather than cached unpurgeable.
    fn mark_cacheable(response: &mut Response<Body>, sha: Option<&str>, tags: &str) {
        let Ok(tag_header) = HeaderValue::from_str(tags) else {
            console_error!("cache tags {tags:?} form no valid header — response left uncached");
            return;
        };
        let headers = response.headers_mut();
        headers.insert(HeaderName::from_static("cache-tag"), tag_header);
        headers.insert(
            CACHE_CONTROL,
            HeaderValue::from_static(cache::CACHE_CONTROL),
        );
        if let Some(value) = sha.and_then(|sha| HeaderValue::from_str(&cache::etag(sha)).ok()) {
            headers.insert(ETAG, value);
        }
    }

    /// Downgrades a 200 matching the client's `If-None-Match` to a bodyless 304.
    fn revalidated(response: Response<Body>, if_none_match: Option<&str>) -> Response<Body> {
        let etag = response
            .headers()
            .get(ETAG)
            .and_then(|value| value.to_str().ok());
        if !cache::revalidates(response.status().as_u16(), if_none_match, etag) {
            return response;
        }
        let (mut parts, _) = response.into_parts();
        parts.status = StatusCode::NOT_MODIFIED;
        let entity: Vec<_> = parts
            .headers
            .keys()
            .filter(|name| cache::is_entity_header(name.as_str()))
            .cloned()
            .collect();
        for name in entity {
            parts.headers.remove(&name);
        }
        Response::from_parts(parts, Body::empty())
    }

    #[worker::send]
    async fn post_page(
        State(state): State<AppState>,
        Path(slug): Path<String>,
        req: Request<Body>,
    ) -> Response<Body> {
        let loaded = load_or_500(load_post(&state.env, &slug), &format!("post {slug}")).await;
        let (post, sha) = match loaded {
            Ok(loaded) => loaded,
            Err(response) => return response,
        };
        let not_found = post.is_none();
        // Drafts render (shareable by URL) but never cache: an unpublish
        // would leave them served for the full TTL.
        let cacheable = post
            .as_ref()
            .is_some_and(|document| !document.frontmatter.draft);

        let mut response =
            render_page(&state, req, move || provide_context(PostData(post.clone()))).await;
        if not_found {
            *response.status_mut() = StatusCode::NOT_FOUND;
        } else if cacheable {
            mark_cacheable(
                &mut response,
                sha.as_deref(),
                &cache::post_cache_tags(&slug),
            );
        }
        response
    }

    #[worker::send]
    async fn listing_page(State(state): State<AppState>, req: Request<Body>) -> Response<Body> {
        let (index, sha) = match load_or_500(load_index(&state.env), "the post index").await {
            Ok(loaded) => loaded,
            Err(response) => return response,
        };

        let mut response = render_page(&state, req, move || {
            provide_context(IndexData(index.clone()))
        })
        .await;
        mark_cacheable(&mut response, sha.as_deref(), &cache::view_cache_tags());
        response
    }

    /// Hardcoded pages, no KV read: nothing to inject and no snapshot sha to
    /// serve as an ETag; cached under the site tag alone — they change on
    /// deploy (which purges `site`), never on publish.
    #[worker::send]
    async fn static_page(State(state): State<AppState>, req: Request<Body>) -> Response<Body> {
        let mut response = render_page(&state, req, || ()).await;
        mark_cacheable(&mut response, None, SITE_TAG);
        response
    }

    /// SSRs the shell so the app's router fallback renders the 404 page
    /// with a real 404 status. Never cacheable.
    #[worker::send]
    async fn not_found_page(State(state): State<AppState>, req: Request<Body>) -> Response<Body> {
        let mut response = render_page(&state, req, || ()).await;
        *response.status_mut() = StatusCode::NOT_FOUND;
        response
    }

    #[worker::send]
    async fn feed_xml(State(state): State<AppState>, req: Request<Body>) -> Response<Body> {
        index_xml(
            &state,
            &req,
            feeds::atom,
            "application/atom+xml; charset=utf-8",
        )
        .await
    }

    #[worker::send]
    async fn sitemap_xml(State(state): State<AppState>, req: Request<Body>) -> Response<Body> {
        index_xml(
            &state,
            &req,
            feeds::sitemap,
            "application/xml; charset=utf-8",
        )
        .await
    }

    async fn index_xml(
        state: &AppState,
        req: &Request<Body>,
        build: fn(&str, &[IndexEntry]) -> String,
        content_type: &'static str,
    ) -> Response<Body> {
        let (index, sha) = match load_or_500(load_index(&state.env), "the post index").await {
            Ok(loaded) => loaded,
            Err(response) => return response,
        };
        let mut response =
            ([(CONTENT_TYPE, content_type)], build(&origin(req), &index)).into_response();
        mark_cacheable(&mut response, sha.as_deref(), &cache::view_cache_tags());
        response
    }

    /// Scheme + host, no trailing slash; the Host header is the dev-server
    /// fallback (workers hand axum an absolute URI).
    fn origin(req: &Request<Body>) -> String {
        let uri = req.uri();
        match (uri.scheme_str(), uri.authority()) {
            (Some(scheme), Some(authority)) => format!("{scheme}://{authority}"),
            _ => {
                let host = req
                    .headers()
                    .get("host")
                    .and_then(|host| host.to_str().ok())
                    .unwrap_or("localhost");
                format!("https://{host}")
            }
        }
    }

    /// SSRs the shell; handlers differ only by the context they inject.
    async fn render_page(
        state: &AppState,
        req: Request<Body>,
        provide: impl Fn() + Clone + Send + Sync + 'static,
    ) -> Response<Body> {
        let options = state.options.clone();
        let render =
            leptos_axum::render_app_async_with_context(provide, move || shell(options.clone()));
        render(req).await
    }

    async fn load_or_500<T>(
        load: impl std::future::Future<Output = Result<T, String>>,
        what: &str,
    ) -> Result<T, Response<Body>> {
        load.await.map_err(|err| {
            console_error!("failed to load {what}: {err}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("{what} could not be loaded"),
            )
                .into_response()
        })
    }

    /// `None` until the first pointer flip; a corrupt pointer is a loud
    /// error, never a silent fallback.
    async fn current_sha(kv: &worker::kv::KvStore) -> Result<Option<String>, String> {
        let json = kv
            .get(CURRENT_KEY)
            .text()
            .await
            .map_err(|err| err.to_string())?;
        json.map(|json| {
            CurrentPointer::from_json(&json)
                .map(|pointer| pointer.sha)
                .map_err(|err| err.to_string())
        })
        .transpose()
    }

    /// Snapshot-pinned KV read; the sha rides along because it doubles as
    /// the page's ETag.
    async fn snapshot_read<T>(
        env: &Env,
        key: impl FnOnce(Option<&str>) -> String,
        parse: impl FnOnce(&str) -> Result<T, String>,
    ) -> Result<(Option<T>, Option<String>), String> {
        let kv = env.kv(KV_BINDING).map_err(|err| err.to_string())?;
        let sha = current_sha(&kv).await?;
        let json = kv
            .get(&key(sha.as_deref()))
            .text()
            .await
            .map_err(|err| err.to_string())?;
        let value = json.as_deref().map(parse).transpose()?;
        Ok((value, sha))
    }

    /// A missing index means nothing published yet; corrupt payloads are
    /// loud errors.
    async fn load_index(env: &Env) -> Result<(Vec<IndexEntry>, Option<String>), String> {
        let (index, sha) = snapshot_read(env, index_key_at, |json| {
            serde_json::from_str(json).map_err(|err| err.to_string())
        })
        .await?;
        Ok((index.unwrap_or_default(), sha))
    }

    /// A KV miss is `Ok(None)` — a plain 404, never a trigger to rebuild.
    async fn load_post(
        env: &Env,
        slug: &str,
    ) -> Result<(Option<Document>, Option<String>), String> {
        snapshot_read(
            env,
            |sha| post_key_at(sha, slug),
            |json| Document::from_json(json).map_err(|err| err.to_string()),
        )
        .await
    }
}
