//! Pure cache policy for the shim's Cache API front, testable natively.

/// Edge keeps pages 7 days; browsers revalidate every view (no purge can
/// reach a client cache). Doubles as the may-store marker: [`should_cache`].
pub const CACHE_CONTROL: &str = "max-age=0, s-maxage=604800";

/// The quoted snapshot sha — one site-wide validator, so any publish
/// invalidates every page.
pub fn etag(sha: &str) -> String {
    format!("\"{sha}\"")
}

/// Does an `If-None-Match` value (list, `W/` prefixes, `*`) match `etag`?
pub fn not_modified(if_none_match: &str, etag: &str) -> bool {
    if_none_match.split(',').map(str::trim).any(|candidate| {
        candidate == "*" || candidate.strip_prefix("W/").unwrap_or(candidate) == etag
    })
}

/// Lookup key: the absolute URL minus query and fragment, matching the bare
/// URLs the purge set names. `None` for relative URIs.
pub fn cache_key(uri: &str) -> Option<String> {
    let (scheme, rest) = uri.split_once("://")?;
    if scheme.is_empty() || rest.is_empty() {
        return None;
    }
    let end = rest
        .find(['?', '#'])
        .map(|i| scheme.len() + 3 + i)
        .unwrap_or(uri.len());
    Some(uri[..end].to_string())
}

/// Store only what a handler explicitly marked: 200 with exactly
/// [`CACHE_CONTROL`].
pub fn should_cache(status: u16, cache_control: Option<&str>) -> bool {
    status == 200 && cache_control == Some(CACHE_CONTROL)
}
