use leptos::prelude::*;
use leptos_meta::Title;

/// `{page} — ~/chris`: every non-home tab title hangs off the site title.
pub(crate) fn page_title(page: &str) -> String {
    format!("{page} — {}", content::SITE_TITLE)
}

/// The display-face page heading: Fraunces, the display size, tight tracking.
/// Shared by [`Page`] and the home masthead so the two front-page headings
/// can't drift in face, size, or tracking.
pub(crate) const DISPLAY_HEADING_CLASS: &str =
    "font-display text-display font-semibold tracking-[-0.01em]";

/// The page frame every route mounts into: the centered column carrying the
/// `page-enter` transition every page's mount is pinned to. The home listing
/// and the post article compose their own masthead into it directly; [`Page`]
/// adds the display heading on top for the about and 404 pages.
#[component]
pub(crate) fn PageShell(children: Children) -> impl IntoView {
    view! { <section class="page-enter mx-auto max-w-2xl px-6 py-16">{children()}</section> }
}

/// [`PageShell`] plus a title and the display heading: the about and 404
/// pages render through it. The home listing and post article open with their
/// own masthead and use the bare shell instead.
#[component]
pub(crate) fn Page(title: String, heading: &'static str, children: Children) -> impl IntoView {
    view! {
        <Title text=title />
        <PageShell>
            <h1 class=DISPLAY_HEADING_CLASS>{heading}</h1>
            {children()}
        </PageShell>
    }
}

#[cfg(test)]
mod tests {
    use super::page_title;

    // The literal pins the suffix shape; agreement with the tab and feed is
    // structural through `content::SITE_TITLE`.
    #[test]
    fn page_titles_hang_off_the_site_title() {
        assert_eq!(page_title("posts"), "posts — ~/chris");
    }
}
