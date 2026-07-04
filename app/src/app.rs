use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    ParamSegment, StaticSegment,
};

use crate::components::counter::Counter;
use crate::post::PostPage;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    // id="leptos" on the link below is what cargo-leptos targets for CSS hot-reload.
    let css_href = format!("/pkg/{}.css", options.output_name);
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <link rel="stylesheet" id="leptos" href=css_href />
                <AutoReload options=options.clone() />
                <HydrationScripts options islands=true />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Title text="chris" />

        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=HomePage />
                    <Route path=(StaticSegment("posts"), ParamSegment("slug")) view=PostPage />
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    view! {
        <div class="mx-auto flex min-h-screen max-w-xl flex-col items-center justify-center gap-6 p-8">
            <h1 class="text-3xl font-bold">"chris"</h1>
            <p class="text-neutral-600">
                "leptos ssr on cloudflare workers. the counter below is an island — "
                "the only thing on this page that hydrates."
            </p>
            <Counter initial=0 />
        </div>
    }
}
