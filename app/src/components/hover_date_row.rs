use leptos::prelude::*;

/// A title-first row whose already-reserved date slides into view on hover
/// or keyboard focus. Keeping the date in flow prevents interaction reflow.
/// Styled by `.hover-date-row` in `app/style/home.css`.
///
/// A row without an `href` is a focusable span, since focus is the only way a
/// keyboard reader reveals its date. Inside a [`Fold`](crate::components::Fold)
/// that trade stops paying: a clipped row would be an invisible tab stop, and
/// the island cannot reach into its opaque children to fix the tab order. Such
/// rows opt out with `focusable=false` and the fold shows their dates outright
/// once open.
#[component]
pub fn HoverDateRow(
    date: String,
    #[prop(optional)] href: Option<String>,
    #[prop(default = false)] current: bool,
    #[prop(default = true)] focusable: bool,
    children: Children,
) -> impl IntoView {
    let words = children().into_any();
    let content = view! {
        <span class="hover-date-row-words">{words}</span>
        <span class="hover-date-row-date" class:hover-date-row-date-current=current>
            {date}
        </span>
    };
    match href {
        Some(href) => view! {
            <a class="hover-date-row" href=href>
                {content}
            </a>
        }
        .into_any(),
        None => view! {
            <span class="hover-date-row" tabindex=focusable.then_some("0")>
                {content}
            </span>
        }
        .into_any(),
    }
}
