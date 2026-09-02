use leptos::prelude::*;

/// A title-first row whose already-reserved date slides into view on hover
/// or keyboard focus. Keeping the date in flow prevents interaction reflow.
#[component]
pub fn HoverDateRow(
    date: String,
    #[prop(optional)] href: Option<String>,
    #[prop(default = false)] current: bool,
    #[prop(default = true)] focusable: bool,
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
        None => {
            let tabindex = if focusable { "0" } else { "-1" };
            view! {
                <span class="hover-date-row" tabindex=tabindex>
                    {content}
                </span>
            }
            .into_any()
        }
    }
}
