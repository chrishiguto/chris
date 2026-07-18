//! The retired `/posts` listing's redirect target, natively pinned: the
//! transport (301 + Location) is the shim's, the location decision is pure.

use site::redirects::posts_redirect_location;

#[test]
fn the_filter_query_rides_the_redirect() {
    assert_eq!(
        posts_redirect_location(Some("q=rust,wasm")),
        "/?q=rust,wasm"
    );
}

#[test]
fn unrelated_params_ride_verbatim() {
    assert_eq!(
        posts_redirect_location(Some("q=rust&utm_source=x")),
        "/?q=rust&utm_source=x"
    );
}

#[test]
fn bare_and_empty_queries_land_on_the_bare_home() {
    assert_eq!(posts_redirect_location(None), "/");
    assert_eq!(posts_redirect_location(Some("")), "/");
}
