//! Derived display metadata: read time from the AST, formatted dates from
//! ISO. Pure and wasm-lean — the publish plan and the live post page must
//! compute the same numbers.

use crate::Node;

const WORDS_PER_MINUTE: usize = 200;

const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];

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

/// `YYYY-MM-DD` → `jul 04, 2026`. Anything off-shape passes through
/// unchanged — display formatting must never panic on stored data.
pub fn format_date(iso: &str) -> String {
    let parts: Vec<&str> = iso.split('-').collect();
    let [year, month, day] = parts[..] else {
        return iso.to_string();
    };
    if !(digits(year, 4) && digits(month, 2) && digits(day, 2)) {
        return iso.to_string();
    }
    month
        .parse::<usize>()
        .ok()
        .and_then(|m| m.checked_sub(1))
        .and_then(|m| MONTHS.get(m))
        .map_or_else(|| iso.to_string(), |name| format!("{name} {day}, {year}"))
}

fn digits(part: &str, len: usize) -> bool {
    part.len() == len && part.bytes().all(|byte| byte.is_ascii_digit())
}
