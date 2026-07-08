//! Native tests for the purge endpoint's bearer check.

use site::auth::verify_bearer;

#[test]
fn verify_bearer_accepts_the_exact_token() {
    assert!(verify_bearer("s3cret", Some("Bearer s3cret")));
}

#[test]
fn verify_bearer_rejects_everything_else() {
    assert!(!verify_bearer("s3cret", None));
    assert!(!verify_bearer("s3cret", Some("s3cret")));
    assert!(!verify_bearer("s3cret", Some("Bearer wrong")));
    assert!(!verify_bearer("s3cret", Some("Bearer s3cret ")));
    assert!(!verify_bearer("s3cret", Some("bearer s3cret")));
    assert!(!verify_bearer("s3cret", Some("Bearer")));
    assert!(!verify_bearer("s3cret", Some("")));
}
