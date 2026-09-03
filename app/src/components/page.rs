use leptos::prelude::*;
use leptos_meta::Title;

use super::Heading;

/// `{page} · ~/chris`: every non-home tab title hangs off the site title.
pub(crate) fn page_title(page: &str) -> String {
    format!("{page} · {}", content::SITE_TITLE)
}

/// The page frame every route mounts into: a flexible gutter on each side of
/// the 44rem reading column. The home, the writing archive, and the post
/// article compose their own opening into it directly; [`Page`] adds the
/// display heading on top for the 404.
#[component]
pub(crate) fn PageShell(children: Children) -> impl IntoView {
    view! {
        <section class="page-grid">
            <div class="page-column page-enter">{children()}</div>
        </section>
    }
}

/// [`PageShell`] plus a title and the display heading: the 404 renders
/// through it. The home, the writing archive, and the post article open
/// with their own masthead and use the bare shell instead.
#[component]
pub(crate) fn Page(title: String, heading: &'static str, children: Children) -> impl IntoView {
    view! {
        <Title text=title />
        <PageShell>
            <Heading>{heading}</Heading>
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
        assert_eq!(page_title("posts"), "posts · ~/chris");
    }
}
