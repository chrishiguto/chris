//! Pure cache policy for the shim's Cache API front (ADR-0008): the key a
//! request caches under and whether a rendered response may be stored.
//! Native-testable; the wasm shim owns the actual `cache.match`/`put`.

/// The 7-day TTL — a staleness backstop for missed purges (ADR-0008).
/// Handlers set it only on pages the publish purge set covers; its exact
/// value doubles as the shim's may-store marker (see [`should_cache`]).
pub const CACHE_CONTROL: &str = "max-age=604800";

/// Cache-API lookup key for a request URL: the absolute URL with query and
/// fragment stripped, so every variant of a page shares the one entry the
/// purge set's bare `origin + path` URLs (the `publish` crate) name exactly.
/// `None` for relative URIs — the Cache API keys on absolute URLs only.
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

/// Only responses a handler explicitly marked cacheable are stored: 200 with
/// exactly [`CACHE_CONTROL`]. Everything else — drafts, 404s, errors, pages
/// no handler opted in — passes through uncached.
pub fn should_cache(status: u16, cache_control: Option<&str>) -> bool {
    status == 200 && cache_control == Some(CACHE_CONTROL)
}
