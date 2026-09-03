use leptos::prelude::*;

/// A title-first row whose already-reserved date slides into view on hover
/// or keyboard focus. Keeping the date in flow prevents interaction reflow.
/// A row without an `href` is a focusable span so the date is reachable by
/// keyboard; a fold that clips rows out of sight takes them out of the tab
/// order itself.
#[component]
pub fn HoverDateRow(
    date: String,
    #[prop(optional)] href: Option<String>,
    #[prop(default = false)] current: bool,
    children: Children,
) -> impl IntoView {
    let words = children().into_any();
    let date_class = if current {
        "hover-date-row-date hover-date-row-date-current"
    } else {
        "hover-date-row-date"
    };
    let content = view! {
        <span class="hover-date-row-words">{words}</span>
        <span class=date_class>{date}</span>
    };
    match href {
        Some(href) => view! {
            <a class="hover-date-row" href=href>
                {content}
            </a>
        }
        .into_any(),
        None => view! {
            <span class="hover-date-row" tabindex="0">
                {content}
            </span>
        }
        .into_any(),
    }
}
