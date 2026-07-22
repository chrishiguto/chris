//! Pure redirect decisions; the worker shim attaches only status and header.

use content::HOME_PATH;

/// The retired `/posts` listing's landing spot: the home front door, with
/// the whole query carried over verbatim so old `?q=` deep links land
/// filtered (and campaign params survive the hop).
pub fn posts_redirect_location(query: Option<&str>) -> String {
    match query {
        Some(query) if !query.is_empty() => format!("{HOME_PATH}?{query}"),
        _ => HOME_PATH.to_string(),
    }
}
