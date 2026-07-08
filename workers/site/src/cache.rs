//! Pure HTTP caching policy — headers, revalidation, purge scopes — testable
//! natively. Storage itself is Workers Cache, in front of the worker.

use content::{post_tag, SITE_TAG, VIEWS_TAG};

/// Workers Cache keeps pages 7 days (`s-maxage`, a backstop for missed
/// purges); browsers revalidate every view (no purge reaches a client cache).
pub const CACHE_CONTROL: &str = "max-age=0, s-maxage=604800";

/// `Cache-Tag` for a post page: evicted site-wide or on its own.
pub fn post_cache_tags(slug: &str) -> String {
    format!("{SITE_TAG},{}", post_tag(slug))
}

/// `Cache-Tag` for the index-backed views (listings, tag pages, feeds).
pub fn view_cache_tags() -> String {
    format!("{SITE_TAG},{VIEWS_TAG}")
}

/// Tags out of a `/__purge` body. No body means `[SITE_TAG]` — the
/// break-glass full purge; anything else must be `{"tags":[...]}` with
/// non-blank entries.
pub fn parse_purge_body(body: &[u8]) -> Result<Vec<String>, String> {
    if body.iter().all(u8::is_ascii_whitespace) {
        return Ok(vec![SITE_TAG.to_string()]);
    }
    #[derive(serde::Deserialize)]
    struct PurgeBody {
        tags: Vec<String>,
    }
    let parsed: PurgeBody =
        serde_json::from_slice(body).map_err(|err| format!("purge body: {err}"))?;
    if parsed.tags.is_empty() || parsed.tags.iter().any(|tag| tag.trim().is_empty()) {
        return Err("tags must be a non-empty list of non-blank strings".to_string());
    }
    Ok(parsed.tags)
}

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
