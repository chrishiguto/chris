#[cfg(feature = "ssr")]
#[worker::event(fetch)]
async fn fetch(
    req: worker::HttpRequest,
    _env: worker::Env,
    _ctx: worker::Context,
) -> worker::Result<axum::http::Response<axum::body::Body>> {
    use axum::Router;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, AxumRouteListing, LeptosRoutes};
    use tower_service::Service;

    use app::app::{shell, App};

    // Isolates are reused across requests, so cache the config and route list
    // (generate_route_list runs the App tree). Router assembly stays per-request:
    // cheap, and it will capture the per-request `env` once KV bindings land.
    thread_local! {
        static CONFIG: (LeptosOptions, Vec<AxumRouteListing>) = {
            let conf = get_configuration(None).unwrap();
            (conf.leptos_options, generate_route_list(App))
        };
    }
    let (leptos_options, routes) = CONFIG.with(Clone::clone);

    let mut router = Router::new()
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .with_state(leptos_options);

    Ok(router.call(req).await?)
}
