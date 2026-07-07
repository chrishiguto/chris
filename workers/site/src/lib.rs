//! Thin worker shim: routing, KV I/O, and the Cache API front. Rendering
//! lives in `app`, feeds in [`feeds`], cache policy in [`cache`].
pub mod cache;
pub mod feeds;

#[cfg(feature = "ssr")]
mod server {
    use crate::{cache, feeds};
    use app::{
        app::{shell, App},
        listing::IndexData,
        post::PostData,
    };
    use axum::{
        body::Body,
        extract::{FromRef, Path, State},
        http::{
            header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH},
            HeaderValue, Method, Request, Response, StatusCode,
        },
        response::IntoResponse,
        routing::get,
        Router,
    };
    use content::{
        index_key_at, post_key_at, CurrentPointer, Document, IndexEntry, CURRENT_KEY, LISTING_PAGES,
    };
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list_with_exclusions, AxumRouteListing, LeptosRoutes};
    use tower_service::Service;
    use worker::{console_error, Cache, Env};

    const KV_BINDING: &str = "BLOG";
    /// Custom axum routes (they need the per-request `Env` for KV), so
    /// excluded from the leptos-generated route list.
    const POST_ROUTE: &str = "/posts/{slug}";
    const TAG_ROUTE: &str = "/tags/{tag}";

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
        // Isolates outlive requests: cache the config and route list, but
        // assemble the router per-request (it captures `env`).
        thread_local! {
            static CONFIG: (LeptosOptions, Vec<AxumRouteListing>) = {
                let conf = get_configuration(None).unwrap();
                let routes = generate_route_list_with_exclusions(
                    App,
                    Some(
                        [POST_ROUTE, TAG_ROUTE]
                            .into_iter()
                            .chain(LISTING_PAGES)
                            .map(String::from)
                            .collect(),
                    ),
                );
                (conf.leptos_options, routes)
            };
        }
        let (options, routes) = CONFIG.with(Clone::clone);
        let state = AppState {
            options: options.clone(),
            env,
        };

        // Captured before the router consumes the request.
        let if_none_match = req
            .headers()
            .get(IF_NONE_MATCH)
            .and_then(|value| value.to_str().ok())
            .map(String::from);

        // Cache front: a hit returns before any KV read or render.
        let key = (req.method() == Method::GET)
            .then(|| cache::cache_key(&req.uri().to_string()))
            .flatten();
        let cache = Cache::default();
        let response = 'response: {
            if let Some(key) = &key {
                if let Some(hit) = cached(&cache, key).await {
                    break 'response hit;
                }
            }

            // One handler serves all listing pages; the leptos Router picks
            // the page from the URL.
            let mut router = LISTING_PAGES
                .iter()
                .fold(
                    Router::new()
                        .route(POST_ROUTE, get(post_page))
                        .route(TAG_ROUTE, get(tag_page))
                        .route("/rss.xml", get(feed_xml))
                        .route("/sitemap.xml", get(sitemap_xml)),
                    |r, path| r.route(path, get(listing_page)),
                )
                .leptos_routes(&state, routes, move || shell(options.clone()))
                .fallback(not_found_page)
                .with_state(state);

            let response = router.call(req).await?;
            let cache_control = response
                .headers()
                .get(CACHE_CONTROL)
                .and_then(|value| value.to_str().ok());
            match key {
                Some(key) if cache::should_cache(response.status().as_u16(), cache_control) => {
                    // The cache keeps the full body; only the client copy
                    // may thin to a 304 below.
                    store(&cache, key, response).await
                }
                _ => response,
            }
        };
        Ok(revalidated(response, if_none_match.as_deref()))
    }

    /// Best-effort: a lookup error is a loud miss, never a 500.
    async fn cached(cache: &Cache, key: &str) -> Option<Response<Body>> {
        let hit = match cache.get(key, false).await {
            Ok(hit) => hit?,
            Err(err) => {
                console_error!("cache lookup for {key} failed: {err}");
                return None;
            }
        };
        match worker::HttpResponse::try_from(hit) {
            Ok(response) => {
                let mut response = response.map(Body::new);
                mark_cache_state(&mut response, "hit");
                Some(response)
            }
            Err(err) => {
                console_error!("cached response for {key} unusable: {err}");
                None
            }
        }
    }

    /// Buffers the body so one copy goes to the cache, one to the client.
    /// A failed put logs but never fails a good render.
    async fn store(cache: &Cache, key: String, response: Response<Body>) -> Response<Body> {
        let (parts, body) = response.into_parts();
        let bytes = match axum::body::to_bytes(body, usize::MAX).await {
            Ok(bytes) => bytes,
            Err(err) => {
                console_error!("buffering {key} for the cache failed: {err}");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "the rendered page could not be buffered",
                )
                    .into_response();
            }
        };
        let copy = Response::from_parts(parts.clone(), Body::from(bytes.clone()));
        match worker::Response::try_from(copy) {
            Ok(copy) => {
                if let Err(err) = cache.put(&key, copy).await {
                    console_error!("cache put for {key} failed: {err}");
                }
            }
            Err(err) => console_error!("cache copy of {key} unusable: {err}"),
        }
        let mut response = Response::from_parts(parts, Body::from(bytes));
        mark_cache_state(&mut response, "miss");
        response
    }

    /// `x-blog-cache: hit|miss` — sent to the client, never stored.
    fn mark_cache_state(response: &mut Response<Body>, state: &'static str) {
        response
            .headers_mut()
            .insert("x-blog-cache", HeaderValue::from_static(state));
    }

    /// Opts a response into the cache front (the exact `Cache-Control` that
    /// [`cache::should_cache`] gates on) plus the snapshot-sha ETag. Never
    /// called for drafts, 404s, or errors.
    fn mark_cacheable(response: &mut Response<Body>, sha: Option<&str>) {
        response.headers_mut().insert(
            CACHE_CONTROL,
            HeaderValue::from_static(cache::CACHE_CONTROL),
        );
        if let Some(value) = sha.and_then(|sha| HeaderValue::from_str(&cache::etag(sha)).ok()) {
            response.headers_mut().insert(ETAG, value);
        }
    }

    /// Downgrades a 200 whose ETag matches the client's `If-None-Match` to a
    /// bodyless 304; anything without an ETag passes through untouched.
    fn revalidated(response: Response<Body>, if_none_match: Option<&str>) -> Response<Body> {
        let etag = response
            .headers()
            .get(ETAG)
            .and_then(|value| value.to_str().ok());
        let (Some(validators), Some(etag)) = (if_none_match, etag) else {
            return response;
        };
        if response.status() != StatusCode::OK || !cache::not_modified(validators, etag) {
            return response;
        }
        let (mut parts, _) = response.into_parts();
        parts.status = StatusCode::NOT_MODIFIED;
        let entity: Vec<_> = parts
            .headers
            .keys()
            .filter(|name| name.as_str().starts_with("content-"))
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
        let (post, sha) = match load_post(&state.env, &slug).await {
            Ok(loaded) => loaded,
            Err(err) => {
                console_error!("failed to load post:{slug}: {err}");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "the stored post could not be loaded",
                )
                    .into_response();
            }
        };
        let not_found = post.is_none();
        // Drafts render (shareable by URL) but are never cached: an
        // unpublish would leave them served for the full TTL.
        let cacheable = post
            .as_ref()
            .is_some_and(|document| !document.frontmatter.draft);

        let mut response =
            render_page(&state, req, move || provide_context(PostData(post.clone()))).await;
        if not_found {
            *response.status_mut() = StatusCode::NOT_FOUND;
        } else if cacheable {
            mark_cacheable(&mut response, sha.as_deref());
        }
        response
    }

    #[worker::send]
    async fn listing_page(State(state): State<AppState>, req: Request<Body>) -> Response<Body> {
        let (index, sha) = match load_index_or_500(&state.env).await {
            Ok(loaded) => loaded,
            Err(response) => return response,
        };

        let mut response = render_page(&state, req, move || {
            provide_context(IndexData(index.clone()))
        })
        .await;
        mark_cacheable(&mut response, sha.as_deref());
        response
    }

    /// Unknown tags 404; the body is the app's empty state either way.
    #[worker::send]
    async fn tag_page(
        State(state): State<AppState>,
        Path(tag): Path<String>,
        req: Request<Body>,
    ) -> Response<Body> {
        let (index, sha) = match load_index_or_500(&state.env).await {
            Ok(loaded) => loaded,
            Err(response) => return response,
        };
        let known = index
            .iter()
            .any(|entry| entry.is_listed() && entry.tags.contains(&tag));

        let mut response = render_page(&state, req, move || {
            provide_context(IndexData(index.clone()))
        })
        .await;
        if known {
            mark_cacheable(&mut response, sha.as_deref());
        } else {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
        response
    }

    /// Unmatched paths: SSR the shell so the app's router fallback renders
    /// the 404 page, with a real 404 status. Never marked cacheable.
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

    /// Renders a pure XML builder over the KV index.
    async fn index_xml(
        state: &AppState,
        req: &Request<Body>,
        build: fn(&str, &[IndexEntry]) -> String,
        content_type: &'static str,
    ) -> Response<Body> {
        let (index, sha) = match load_index_or_500(&state.env).await {
            Ok(loaded) => loaded,
            Err(response) => return response,
        };
        let mut response =
            ([(CONTENT_TYPE, content_type)], build(&origin(req), &index)).into_response();
        mark_cacheable(&mut response, sha.as_deref());
        response
    }

    /// Scheme + host, no trailing slash. Workers hand axum an absolute URI;
    /// the Host header is the dev-server fallback.
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

    /// The KV index (with its snapshot sha) or the shared 500 response.
    async fn load_index_or_500(
        env: &Env,
    ) -> Result<(Vec<IndexEntry>, Option<String>), Response<Body>> {
        load_index(env).await.map_err(|err| {
            console_error!("failed to load index: {err}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "the post index could not be loaded",
            )
                .into_response()
        })
    }

    /// The published snapshot's sha; `None` until the first flip. A corrupt
    /// pointer is a loud error, never a silent fallback.
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

    /// One snapshot-pinned KV read; the sha rides along because it doubles
    /// as the page's ETag.
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

    /// A missing index means nothing published yet (an empty listing);
    /// corrupt payloads are loud errors.
    async fn load_index(env: &Env) -> Result<(Vec<IndexEntry>, Option<String>), String> {
        let (index, sha) = snapshot_read(env, index_key_at, |json| {
            serde_json::from_str(json).map_err(|err| err.to_string())
        })
        .await?;
        Ok((index.unwrap_or_default(), sha))
    }

    /// A KV miss is `Ok(None)` — a plain 404, never a trigger to rebuild;
    /// corrupt payloads are loud errors.
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
