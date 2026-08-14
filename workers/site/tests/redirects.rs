//! The legacy listing redirects' targets, natively pinned: the transport
//! (301 + Location) is the shim's, the location decisions are pure.

use site::redirects::{home_redirect_location, posts_redirect_location};

#[test]
fn the_filter_query_rides_the_redirect() {
    assert_eq!(
        posts_redirect_location(Some("q=rust,wasm")),
        "/writing?q=rust,wasm"
    );
}

#[test]
fn unrelated_params_ride_verbatim() {
    assert_eq!(
        posts_redirect_location(Some("q=rust&utm_source=x")),
        "/writing?q=rust&utm_source=x"
    );
}

#[test]
fn bare_and_empty_queries_land_on_the_bare_writing_page() {
    assert_eq!(posts_redirect_location(None), "/writing");
    assert_eq!(posts_redirect_location(Some("")), "/writing");
}

// The filter briefly rooted at the home; only a query actually carrying the
// filter param moves — whole, so campaign params ride along.
#[test]
fn home_moves_filter_queries_to_the_writing_page() {
    assert_eq!(
        home_redirect_location(Some("q=rust,wasm")).as_deref(),
        Some("/writing?q=rust,wasm")
    );
    assert_eq!(
        home_redirect_location(Some("utm_source=x&q=rust")).as_deref(),
        Some("/writing?utm_source=x&q=rust")
    );
    // A bare `q` restores the empty selection — still the writing page's.
    assert_eq!(
        home_redirect_location(Some("q")).as_deref(),
        Some("/writing?q")
    );
}

#[test]
fn home_keeps_filterless_queries_where_they_are() {
    assert_eq!(home_redirect_location(None), None);
    assert_eq!(home_redirect_location(Some("")), None);
    assert_eq!(home_redirect_location(Some("utm_source=x")), None);
    // Lookalike params never trip the filter shape.
    assert_eq!(home_redirect_location(Some("query=x")), None);
    assert_eq!(home_redirect_location(Some("iq=140")), None);
}
