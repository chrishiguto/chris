//! Derived metadata: read time from the AST. Pure and wasm-lean — the
//! publish plan and the live post page must compute the same number.

use crate::Node;

const WORDS_PER_MINUTE: usize = 200;

/// Estimated read time at ~200 wpm, code blocks excluded. Floors at one
/// minute — a wordless post must never display "0 min".
pub fn reading_minutes(ast: &[Node]) -> u32 {
    let minutes = words(ast).div_ceil(WORDS_PER_MINUTE).max(1);
    u32::try_from(minutes).unwrap_or(u32::MAX)
}

/// Prose words only: code blocks, image alt text, and attribute strings are
/// not read in the text flow.
fn words(nodes: &[Node]) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            Node::Text { value } | Node::InlineCode { value } => value.split_whitespace().count(),
            Node::Heading { children, .. }
            | Node::Paragraph { children }
            | Node::Emphasis { children }
            | Node::Strong { children }
            | Node::Link { children, .. }
            | Node::Blockquote { children }
            | Node::Html { children, .. }
            | Node::Component { children, .. } => words(children),
            Node::List { items, .. } => items.iter().map(|item| words(&item.children)).sum(),
            Node::Image { .. } | Node::CodeBlock { .. } | Node::ThematicBreak | Node::Break => 0,
        })
        .sum()
}
