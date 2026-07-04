//! Thin worker shim per the PRD's testing decisions: routing and KV I/O only.
//! All rendering logic lives in `app`, testable natively with `cargo test`.
#[cfg(feature = "ssr")]
mod server {
    use app::{
        app::{shell, App},
        post::PostData,
    };
    use axum::{
        body::Body,
        extract::{FromRef, Path, State},
        http::{Request, Response, StatusCode},
        response::IntoResponse,
        routing::get,
        Router,
    };
    use content_ast::Document;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list_with_exclusions, AxumRouteListing, LeptosRoutes};
    use tower_service::Service;
    use worker::{console_error, Env};

    /// KV namespace holding `post:{slug}` documents (see wrangler.toml).
    const KV_BINDING: &str = "BLOG";
    /// Handled by a custom axum route (it needs the per-request `Env` for the
    /// KV read), so it is excluded from the leptos-generated route list.
    const POST_ROUTE: &str = "/posts/{slug}";

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
                    Some(vec![POST_ROUTE.to_string()]),
                );
                (conf.leptos_options, routes)
            };
        }
        let (options, routes) = CONFIG.with(Clone::clone);
        let state = AppState {
            options: options.clone(),
            env,
        };

        let mut router = Router::new()
            .route(POST_ROUTE, get(post_page))
            .leptos_routes(&state, routes, move || shell(options.clone()))
            .with_state(state);

        Ok(router.call(req).await?)
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

        let render = leptos_axum::render_app_async_with_context(
            move || provide_context(PostData(post.clone())),
            {
                let options = state.options.clone();
                move || shell(options.clone())
            },
        );
        let mut response = render(req).await;
        if not_found {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
        response
    }

    /// A KV miss is `Ok(None)` — served as a plain 404, never a trigger to
    /// rebuild (ADR-0001; user story 33). Corrupt or wrong-schema payloads
    /// are errors so pipeline bugs surface loudly.
    async fn load_post(env: &Env, slug: &str) -> Result<Option<Document>, String> {
        let kv = env.kv(KV_BINDING).map_err(|err| err.to_string())?;
        let json = kv
            .get(&format!("post:{slug}"))
            .text()
            .await
            .map_err(|err| err.to_string())?;
        json.map(|json| Document::from_json(&json).map_err(|err| err.to_string()))
            .transpose()
    }
}
