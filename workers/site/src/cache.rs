//! Pure HTTP caching policy — headers and revalidation — testable natively.
//! Storage itself is Workers Cache, in front of the worker.

/// Workers Cache keeps pages 7 days (`s-maxage`, a backstop for missed
/// purges); browsers revalidate every view (no purge reaches a client cache).
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

/// A 200 whose ETag matches the client's `If-None-Match` may thin to a
/// bodyless 304.
pub fn revalidates(status: u16, if_none_match: Option<&str>, etag: Option<&str>) -> bool {
    status == 200
        && match (if_none_match, etag) {
            (Some(validators), Some(etag)) => not_modified(validators, etag),
            _ => false,
        }
}

/// Headers describing the entity body, dropped when a 304 drops the body.
pub fn is_entity_header(name: &str) -> bool {
    name.starts_with("content-")
}
