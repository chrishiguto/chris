//! The AST renderer: turns a content-IR [`Document`] (read from KV by the
//! site worker) into Leptos views. The stored AST is semantic; every
//! presentational decision lives here.

use content::{Document, Node};
use leptos::attr::custom::custom_attribute;
use leptos::prelude::*;

/// Renders a full post document: header (title, date, tags) plus body.
pub fn render_document(doc: &Document) -> impl IntoView {
    let tags = (!doc.frontmatter.tags.is_empty()).then(|| {
        let tags: Vec<_> = doc
            .frontmatter
            .tags
            .iter()
            .map(|tag| view! { <li class="tag">{tag.clone()}</li> })
            .collect();
        view! { <ul class="post-tags">{tags}</ul> }
    });
    // Prose lives in `.post-body`, so `.post-body <element>` selectors can
    // never bleed into the header chrome above it.
    view! {
        <article class="post mx-auto max-w-2xl px-6">
            <header>
                <h1>{doc.frontmatter.title.clone()}</h1>
                <p class="post-date">{doc.frontmatter.date.clone()}</p>
                {tags}
            </header>
            <div class="post-body">{render_nodes(&doc.ast)}</div>
        </article>
    }
}

/// Renders a slice of AST nodes; the recursion point for all children.
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
        // `class=Option::None` still emits `class=""`, so branch instead.
        Node::CodeBlock { lang, text } => match lang {
            Some(lang) => view! {
                <pre>
                    <code class=format!("language-{lang}")>{text.clone()}</code>
                </pre>
            }
            .into_any(),
            None => view! {
                <pre>
                    <code>{text.clone()}</code>
                </pre>
            }
            .into_any(),
        },
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
        // Publish-time validation makes both error arms unreachable for
        // content that went through the pipeline; if bad data lands in KV
        // anyway it must fail visibly, never silently.
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
