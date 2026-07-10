use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    ParamSegment, StaticSegment,
};

use crate::about::AboutPage;
use crate::components::{Footer, Header, NotFound};
use crate::listing::{HomePage, PostsPage};
use crate::post::PostPage;

/// Geist + Geist Mono from Google Fonts; `swap` deliberately reverses the v1
/// self-hosted/`optional` strategy (PRD: design-system migration).
pub const GOOGLE_FONTS_URL: &str = "https://fonts.googleapis.com/css2?family=Geist:wght@400;500;600;700&family=Geist+Mono:wght@400;500;600&display=swap";

/// Re-applies a stored explicit theme before any stylesheet loads, so the
/// first paint can't flash the wrong theme (ADR-0011). A constant: the served
/// HTML is byte-identical for every visitor, keeping the edge cache one
/// response per URL. Unknown stored values are ignored — `color-scheme`
/// then keeps following the system preference.
pub const THEME_SCRIPT: &str = r#"try{var t=localStorage.getItem("chris-theme");if(t==="light"||t==="dark")document.documentElement.dataset.theme=t}catch(e){}"#;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    // cargo-leptos targets id="leptos" on the link below for CSS hot-reload.
    let css_href = format!("/pkg/{}.css", options.output_name);
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <script inner_html=THEME_SCRIPT></script>
                <link rel="preconnect" href="https://fonts.googleapis.com" />
                <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous" />
                <link rel="stylesheet" href=GOOGLE_FONTS_URL />
                <link rel="stylesheet" id="leptos" href=css_href />
                <link
                    rel="alternate"
                    type="application/atom+xml"
                    title="chris"
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
        <Title text="~/chris" />

        <Router>
            <div class="flex min-h-screen flex-col">
                <Header />
                <main class="flex-1">
                    <Routes fallback=|| view! { <NotFound /> }>
                        <Route path=StaticSegment("") view=HomePage />
                        <Route path=StaticSegment("posts") view=PostsPage />
                        <Route path=(StaticSegment("posts"), ParamSegment("slug")) view=PostPage />
                        <Route path=StaticSegment("about") view=AboutPage />
                    </Routes>
                </main>
                <Footer />
            </div>
        </Router>
    }
}
