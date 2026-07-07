use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    ParamSegment, StaticSegment,
};

use crate::components::{Header, NotFound};
use crate::listing::{HomePage, PostsPage, TagPage, TagsPage};
use crate::post::PostPage;

/// Critical faces (body, headings, code) preloaded so they reliably beat the
/// first paint — with `font-display: optional` (main.css) a face that misses
/// it is skipped for the page's whole life, never swapped in.
pub const PRELOADED_FONTS: [&str; 3] = [
    "/fonts/lora-latin-400-normal.woff2",
    "/fonts/libre-baskerville-latin-700-normal.woff2",
    "/fonts/ibm-plex-mono-latin-400-normal.woff2",
];

pub fn shell(options: LeptosOptions) -> impl IntoView {
    // id="leptos" on the link below is what cargo-leptos targets for CSS hot-reload.
    let css_href = format!("/pkg/{}.css", options.output_name);
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                {PRELOADED_FONTS
                    .map(|href| {
                        view! {
                            <link
                                rel="preload"
                                href=href
                                r#as="font"
                                type="font/woff2"
                                crossorigin="anonymous"
                            />
                        }
                    })
                    .collect_view()}
                <link rel="stylesheet" id="leptos" href=css_href />
                <link rel="alternate" type="application/atom+xml" title="chris" href="/rss.xml" />
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
            <Header />
            <main>
                <Routes fallback=|| view! { <NotFound /> }>
                    <Route path=StaticSegment("") view=HomePage />
                    <Route path=StaticSegment("posts") view=PostsPage />
                    <Route path=(StaticSegment("posts"), ParamSegment("slug")) view=PostPage />
                    <Route path=StaticSegment("tags") view=TagsPage />
                    <Route path=(StaticSegment("tags"), ParamSegment("tag")) view=TagPage />
                </Routes>
            </main>
        </Router>
    }
}
