//! Thin worker shim per the PRD's testing decisions: routing, KV I/O, and
//! the Cache API front only. HTML rendering lives in `app`, feed/sitemap
//! rendering in [`feeds`], and the cache policy in [`cache`] — all testable
//! natively with `cargo test`.
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
            header::{CACHE_CONTROL, CONTENT_TYPE},
            HeaderValue, Method, Request, Response, StatusCode,
        },
        response::IntoResponse,
        routing::get,
        Router,
    };
    use content::{post_key, Document, IndexEntry, INDEX_KEY, LISTING_PAGES};
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list_with_exclusions, AxumRouteListing, LeptosRoutes};
    use tower_service::Service;
    use worker::{console_error, Cache, Env};

    /// KV namespace holding `post:{slug}` documents (see wrangler.toml).
    const KV_BINDING: &str = "BLOG";
    /// Handled by custom axum routes (they need the per-request `Env` for
    /// KV reads), so they are excluded from the leptos-generated route list.
    const POST_ROUTE: &str = "/posts/{slug}";
    /// Like [`POST_ROUTE`], but 404s on tags no published post carries.
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
        // Isolates are reused across requests, so cache the config and route
        // list (generate_route_list runs the App tree). Router assembly stays
        // per-request: cheap, and it captures the per-request `env`.
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

        // Cache front (ADR-0008): a hit returns before any KV read or render.
        let key = (req.method() == Method::GET)
            .then(|| cache::cache_key(&req.uri().to_string()))
            .flatten();
        let cache = Cache::default();
        if let Some(key) = &key {
            if let Some(hit) = cached(&cache, key).await {
                return Ok(hit);
            }
        }

        // Listing pages render from the KV `index`; one handler serves them
        // all — the leptos Router picks the page from the request URL.
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
            .with_state(state);

        let response = router.call(req).await?;
        let cache_control = response
            .headers()
            .get(CACHE_CONTROL)
            .and_then(|value| value.to_str().ok());
        match key {
            Some(key) if cache::should_cache(response.status().as_u16(), cache_control) => {
                Ok(store(&cache, key, response).await)
            }
            _ => Ok(response),
        }
    }

    /// Cache lookups are best-effort: an error is a loud miss, never a 500 —
    /// KV still holds the truth and the render path works without the cache.
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

    /// Buffers the rendered body so one copy goes to the cache and one to
    /// the client. A failed put logs loudly but never fails a good render
    /// (the next request just renders again).
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

    /// `x-blog-cache: hit|miss` — sent to the client only, never stored, so
    /// the hit-vs-miss path is verifiable from response headers alone.
    fn mark_cache_state(response: &mut Response<Body>, state: &'static str) {
        response
            .headers_mut()
            .insert("x-blog-cache", HeaderValue::from_static(state));
    }

    /// Opts a response into the cache front: the exact `Cache-Control` the
    /// shim's [`cache::should_cache`] gate looks for. Only handlers call
    /// this, and only for pages the publish purge set covers — never for
    /// drafts, 404s, or errors.
    fn mark_cacheable(response: &mut Response<Body>) {
        response.headers_mut().insert(
            CACHE_CONTROL,
            HeaderValue::from_static(cache::CACHE_CONTROL),
        );
    }

    #[worker::send]
    async fn post_page(
        State(state): State<AppState>,
        Path(slug): Path<String>,
        req: Request<Body>,
    ) -> Response<Body> {
        let post = match load_post(&state.env, &slug).await {
            Ok(post) => post,
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
        // Drafts render (shareable by URL, Slice 9) but must never be
        // cached: an unpublish would leave them served for the full TTL.
        let cacheable = post
            .as_ref()
            .is_some_and(|document| !document.frontmatter.draft);

        let mut response =
            render_page(&state, req, move || provide_context(PostData(post.clone()))).await;
        if not_found {
            *response.status_mut() = StatusCode::NOT_FOUND;
        } else if cacheable {
            mark_cacheable(&mut response);
        }
        response
    }

    #[worker::send]
    async fn listing_page(State(state): State<AppState>, req: Request<Body>) -> Response<Body> {
        let index = match load_index_or_500(&state.env).await {
            Ok(index) => index,
            Err(response) => return response,
        };

        let mut response = render_page(&state, req, move || {
            provide_context(IndexData(index.clone()))
        })
        .await;
        mark_cacheable(&mut response);
        response
    }

    /// Unknown tags 404 (no published post carries them); the page body is
    /// the app's readable empty state either way.
    #[worker::send]
    async fn tag_page(
        State(state): State<AppState>,
        Path(tag): Path<String>,
        req: Request<Body>,
    ) -> Response<Body> {
        let index = match load_index_or_500(&state.env).await {
            Ok(index) => index,
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
            mark_cacheable(&mut response);
        } else {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
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

    /// Renders one of the pure XML builders over the KV index, with absolute
    /// URLs rooted at the requesting origin.
    async fn index_xml(
        state: &AppState,
        req: &Request<Body>,
        build: fn(&str, &[IndexEntry]) -> String,
        content_type: &'static str,
    ) -> Response<Body> {
        let index = match load_index_or_500(&state.env).await {
            Ok(index) => index,
            Err(response) => return response,
        };
        (
            [
                (CONTENT_TYPE, content_type),
                (CACHE_CONTROL, cache::CACHE_CONTROL),
            ],
            build(&origin(req), &index),
        )
            .into_response()
    }

    /// Scheme + host of the request, no trailing slash. Workers hand axum an
    /// absolute request URI; the Host header is the dev-server fallback.
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

    /// SSRs the full shell for a page; handlers differ only by the per-request
    /// context they inject, so that is all this takes.
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

    /// The KV index or the shared 500 response — the three index-backed
    /// handlers (listings, tag pages, feeds) load it under one error contract.
    async fn load_index_or_500(env: &Env) -> Result<Vec<IndexEntry>, Response<Body>> {
        load_index(env).await.map_err(|err| {
            console_error!("failed to load index: {err}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "the post index could not be loaded",
            )
                .into_response()
        })
    }

    /// A missing `index` key just means nothing has been published yet —
    /// rendered as an empty listing. Corrupt payloads are errors so pipeline
    /// bugs surface loudly (ADR-0001).
    async fn load_index(env: &Env) -> Result<Vec<IndexEntry>, String> {
        let kv = env.kv(KV_BINDING).map_err(|err| err.to_string())?;
        let json = kv
            .get(INDEX_KEY)
            .text()
            .await
            .map_err(|err| err.to_string())?;
        json.map(|json| serde_json::from_str(&json).map_err(|err| err.to_string()))
            .transpose()
            .map(Option::unwrap_or_default)
    }

    /// A KV miss is `Ok(None)` — served as a plain 404, never a trigger to
    /// rebuild (ADR-0001; user story 33). Corrupt or wrong-schema payloads
    /// are errors so pipeline bugs surface loudly.
    async fn load_post(env: &Env, slug: &str) -> Result<Option<Document>, String> {
        let kv = env.kv(KV_BINDING).map_err(|err| err.to_string())?;
        let json = kv
            .get(&post_key(slug))
            .text()
            .await
            .map_err(|err| err.to_string())?;
        json.map(|json| Document::from_json(&json).map_err(|err| err.to_string()))
            .transpose()
    }
}
