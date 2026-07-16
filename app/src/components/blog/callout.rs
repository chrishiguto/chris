use leptos::prelude::*;
use registry::post_component;

#[post_component]
#[component]
pub fn Callout(kind: String, title: Option<String>, children: Children) -> impl IntoView {
    let class = format!("callout callout-{kind}");
    view! {
        <aside class=class>
            <span class="callout-label">{kind}</span>
            {title.map(|title| view! { <p class="callout-title">{title}</p> })}
            <div class="callout-body">{children()}</div>
        </aside>
    }
}
