//! Pure redirect decisions; the worker shim attaches only status and header.

use content::WRITING_PATH;

/// The retired `/posts` listing's landing spot: the writing archive, with
/// the whole query carried over verbatim so old `?q=` deep links land
/// filtered (and campaign params survive the hop).
pub fn posts_redirect_location(query: Option<&str>) -> String {
    match query {
        Some(query) if !query.is_empty() => format!("{WRITING_PATH}?{query}"),
        _ => WRITING_PATH.to_string(),
    }
}
