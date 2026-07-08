//! Constant-time shared-secret bearer-token verification, shared across the
//! workers (the site's `/__purge` and the pipeline's `/publish`). Pure and
//! natively tested.

use hmac::{Hmac, Mac};
use sha2::Sha256;

/// Exact `Bearer <secret>` match. Both sides go through a fixed-key HMAC so
/// the comparison is constant-time with no length oracle.
pub fn verify_bearer(secret: &str, header: Option<&str>) -> bool {
    let Some(token) = header.and_then(|value| value.strip_prefix("Bearer ")) else {
        return false;
    };
    let Ok(mac) = Hmac::<Sha256>::new_from_slice(b"bearer-auth") else {
        return false;
    };
    let expected = mac.clone().chain_update(secret.as_bytes()).finalize();
    mac.chain_update(token.as_bytes())
        .verify_slice(&expected.into_bytes())
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::verify_bearer;

    #[test]
    fn bearer_accepts_the_exact_token() {
        assert!(verify_bearer("s3cret", Some("Bearer s3cret")));
    }

    #[test]
    fn bearer_rejects_everything_else() {
        assert!(!verify_bearer("s3cret", None));
        assert!(!verify_bearer("s3cret", Some("s3cret")));
        assert!(!verify_bearer("s3cret", Some("Bearer wrong")));
        assert!(!verify_bearer("s3cret", Some("Bearer s3cret ")));
        assert!(!verify_bearer("s3cret", Some("Bearer s3cret extra")));
        assert!(!verify_bearer("s3cret", Some("bearer s3cret")));
        assert!(!verify_bearer("s3cret", Some("Bearer")));
        assert!(!verify_bearer("s3cret", Some("")));
    }
}
