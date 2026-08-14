//! Pure redirect decisions; the worker shim attaches only status and header.

use content::{TAG_FILTER_PARAM, WRITING_PATH};

/// The retired `/posts` listing's landing spot: the writing page, with the
/// whole query carried over verbatim so old `?q=` deep links land filtered
/// (and campaign params survive the hop).
pub fn posts_redirect_location(query: Option<&str>) -> String {
    match query {
        Some(query) if !query.is_empty() => format!("{WRITING_PATH}?{query}"),
        _ => WRITING_PATH.to_string(),
    }
}

/// The filter briefly rooted at the home (ADR-0012's 07-16 shape), so `/?q=`
/// links exist in the wild: a home query carrying the filter param moves to
/// the writing page whole — campaign params ride along — and any other query
/// stays put, since the static home ignores it.
pub fn home_redirect_location(query: Option<&str>) -> Option<String> {
    let query = query.filter(|query| {
        query
            .split('&')
            .any(|pair| pair.split('=').next() == Some(TAG_FILTER_PARAM))
    })?;
    Some(format!("{WRITING_PATH}?{query}"))
}
