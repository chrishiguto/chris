//! Site-wide UI components; post-embeddable ones live under [`blog`].

use leptos::prelude::*;
use leptos_meta::Title;

pub mod blog;
pub mod copy_button;
pub mod footer;
pub mod header;
pub mod konami;
pub mod not_found;
pub mod tag_filter;
pub mod theme_toggle;

pub use copy_button::CopyButton;
pub use footer::Footer;
pub use header::Header;
pub use konami::Konami;
pub use not_found::NotFound;
pub use tag_filter::TagFilter;
pub use theme_toggle::ThemeToggle;

/// Meta-row content (design Meta): formatted date, then `· N min` when the
/// read time is known — absent minutes render the date alone. The wrapper
/// (`p.post-meta` / `span.post-row-meta`) belongs to the caller.
pub(crate) fn meta_row(date: &str, minutes: Option<u32>) -> impl IntoView {
    let time = minutes.map(|minutes| {
        view! {
            <span class="meta-sep" aria-hidden="true">
                "·"
            </span>
            <span>{format!("{minutes} min")}</span>
        }
    });
    view! {
        <span>{content::format_date(date)}</span>
        {time}
    }
}

/// Mono section label (design SectionLabel); shared by the home and about pages.
pub(crate) fn section_label(text: &'static str) -> impl IntoView {
    view! { <p class="font-mono text-xs tracking-wide text-ink-3">{text}</p> }
}

/// Tag pill (design Tag): one `<li>` of a `.post-tags` row, shared by the
/// article bottom and the listing's filter row. Single-sourced because the
/// shape is a contract: the filter island reads the tag back out of the
/// href's fragment, and `a.tag` / `.tag-hash` key the CSS.
pub(crate) fn tag_pill(tag: String) -> impl IntoView {
    let href = content::tag_filter_path(&tag);
    view! {
        <li>
            <a class="tag" href=href>
                <span class="tag-hash" aria-hidden="true">
                    "#"
                </span>
                {tag}
            </a>
        </li>
    }
}

/// Shared page scaffold; every page except the post article renders through it.
pub(crate) fn page(
    title: Option<String>,
    heading: impl IntoView,
    body: impl IntoView,
) -> impl IntoView {
    view! {
        {title.map(|text| view! { <Title text=text /> })}
        <section class="mx-auto max-w-2xl px-6 py-16">
            <h1 class="text-3xl font-semibold tracking-tight">{heading}</h1>
            {body}
        </section>
    }
}
