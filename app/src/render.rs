//! AST renderer: [`Document`] → Leptos views. The stored AST is semantic;
//! every presentational decision lives here.

use content::{reading_minutes, Document, Node};
use leptos::attr::custom::custom_attribute;
use leptos::prelude::*;

use crate::components::{meta_row, tag_pill, CopyButton};

pub fn render_document(doc: &Document) -> impl IntoView {
    // Pills close the article and land on the pre-filtered listing (ADR-0012).
    let tags = (!doc.frontmatter.tags.is_empty()).then(|| {
        let tags: Vec<_> = doc.frontmatter.tags.iter().cloned().map(tag_pill).collect();
        view! { <ul class="post-tags">{tags}</ul> }
    });
    // Prose sits in `.post-body` so its element selectors never hit the chrome.
    view! {
        <article class="post mx-auto max-w-2xl px-6">
            <a class="back-link" href="/posts">
                <span class="back-link-arrow" aria-hidden="true">
                    "←"
                </span>
                "back to all posts"
            </a>
            <header>
                <h1>{doc.frontmatter.title.clone()}</h1>
                // The same ~200 wpm number the publish plan stamps into the
                // index, computed live from the AST this page already holds.
                <p class="post-meta">
                    {meta_row(&doc.frontmatter.date, Some(reading_minutes(&doc.ast)))}
                </p>
            </header>
            <div class="post-body">{render_nodes(&doc.ast)}</div>
            {tags}
        </article>
    }
}

pub fn render_nodes(nodes: &[Node]) -> Vec<AnyView> {
    nodes.iter().map(render_node).collect()
}

fn component_error(name: &str, message: String) -> AnyView {
    view! {
        <span class="component-error" data-component=name.to_string()>
            {message}
        </span>
    }
    .into_any()
}

fn render_node(node: &Node) -> AnyView {
    match node {
        Node::Heading { level, children } => {
            let children = render_nodes(children);
            match level {
                1 => view! { <h1>{children}</h1> }.into_any(),
                2 => view! { <h2>{children}</h2> }.into_any(),
                3 => view! { <h3>{children}</h3> }.into_any(),
                4 => view! { <h4>{children}</h4> }.into_any(),
                5 => view! { <h5>{children}</h5> }.into_any(),
                _ => view! { <h6>{children}</h6> }.into_any(),
            }
        }
        Node::Paragraph { children } => view! { <p>{render_nodes(children)}</p> }.into_any(),
        Node::Text { value } => value.clone().into_any(),
        Node::Emphasis { children } => view! { <em>{render_nodes(children)}</em> }.into_any(),
        Node::Strong { children } => view! { <strong>{render_nodes(children)}</strong> }.into_any(),
        Node::InlineCode { value } => view! { <code>{value.clone()}</code> }.into_any(),
        Node::Link {
            url,
            title,
            children,
        } => view! {
            <a href=url.clone() title=title.clone()>
                {render_nodes(children)}
            </a>
        }
        .into_any(),
        Node::Image { url, alt, title } => {
            view! { <img src=url.clone() alt=alt.clone() title=title.clone() /> }.into_any()
        }
        Node::List {
            ordered,
            start,
            items,
        } => {
            let items: Vec<AnyView> = items
                .iter()
                .map(|item| view! { <li>{render_nodes(&item.children)}</li> }.into_any())
                .collect();
            if *ordered {
                view! { <ol start=start.map(|s| s.to_string())>{items}</ol> }.into_any()
            } else {
                view! { <ul>{items}</ul> }.into_any()
            }
        }
        // The chrome bar names the fence language (design CodeBlock); the
        // copy button finds this wrapper by class, so they move together.
        // `class=Option::None` still emits `class=""`, so branch instead.
        Node::CodeBlock { lang, text } => {
            let label = lang.clone().unwrap_or_else(|| "code".into());
            let code = match lang {
                Some(lang) => {
                    view! { <code class=format!("language-{lang}")>{text.clone()}</code> }
                        .into_any()
                }
                None => view! { <code>{text.clone()}</code> }.into_any(),
            };
            view! {
                <div class="code-block">
                    <div class="code-bar">
                        <span class="code-lang">{label}</span>
                        <CopyButton />
                    </div>
                    <pre>{code}</pre>
                </div>
            }
            .into_any()
        }
        Node::Blockquote { children } => {
            view! { <blockquote>{render_nodes(children)}</blockquote> }.into_any()
        }
        Node::ThematicBreak => view! { <hr /> }.into_any(),
        Node::Break => view! { <br /> }.into_any(),
        Node::Html {
            tag,
            attrs,
            children,
        } => {
            let el = leptos::html::custom(tag.clone())
                .child(render_nodes(children))
                .into_any();
            attrs.iter().fold(el, |el, (key, value)| {
                el.add_any_attr(custom_attribute(key.clone(), value.clone()))
                    .into_any()
            })
        }
        // Publish-time validation makes both error arms unreachable, but
        // bad KV data must fail visibly, never silently.
        Node::Component {
            name,
            props,
            children,
        } => match registry::lookup(name) {
            None => component_error(name, format!("unknown component <{name}>")),
            Some(component) => {
                let children = render_nodes(children).into_any();
                match (component.render)(props, children) {
                    Ok(view) => view,
                    Err(err) => component_error(name, format!("<{name}>: {err}")),
                }
            }
        },
    }
}
