use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    ParamSegment, StaticSegment,
};

use crate::components::{fold::FOLD_SCRIPT, Footer, NotFound};
use crate::home::HomePage;
use crate::post::PostPage;
use crate::writing::WritingPage;

/// Newsreader carries every reading voice; Geist Mono is reserved for code.
/// The variable axes ship in the URL so browsers receive italic, optical-size,
/// and the complete 300–700 weight range without local axis overrides.
pub const GOOGLE_FONTS_URL: &str = "https://fonts.googleapis.com/css2?family=Geist+Mono:wght@400&family=Newsreader:ital,opsz,wght@0,6..72,300..700;1,6..72,300..700&display=swap";

pub fn shell(options: LeptosOptions) -> impl IntoView {
    // cargo-leptos targets id="leptos" on the link below for CSS hot-reload.
    let css_href = format!("/pkg/{}.css", options.output_name);
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <link rel="preconnect" href="https://fonts.googleapis.com" />
                <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous" />
                <link rel="stylesheet" href=GOOGLE_FONTS_URL />
                <link rel="stylesheet" id="leptos" href=css_href />
                <link
                    rel="alternate"
                    type="application/atom+xml"
                    title=content::SITE_TITLE
                    href=content::RSS_PATH
                />
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
        <Title text=content::SITE_TITLE />

        <Router>
            <div class="flex min-h-dvh flex-col">
                // The top edge is a blur-and-paper veil, deliberately inert: it
                // dissolves scrolling text without becoming a navigation surface.
                <div class="site-veil" aria-hidden="true"></div>
                <main class="flex-1">
                    <Routes fallback=|| view! { <NotFound /> }>
                        <Route path=StaticSegment("") view=HomePage />
                        <Route path=StaticSegment("writing") view=WritingPage />
                        <Route path=(StaticSegment("posts"), ParamSegment("slug")) view=PostPage />
                    </Routes>
                </main>
                <Footer />
                <script>{FOLD_SCRIPT}</script>
            </div>
        </Router>
    }
}
