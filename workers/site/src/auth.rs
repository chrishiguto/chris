//! Pure bearer-token check for the purge endpoint, testable natively.

use hmac::{Hmac, Mac};
use sha2::Sha256;

/// Only the pipeline may purge: exact `Bearer <secret>` match. Both sides
/// go through HMAC so the comparison is constant-time with no length oracle.
pub fn verify_bearer(secret: &str, header: Option<&str>) -> bool {
    let Some(token) = header.and_then(|value| value.strip_prefix("Bearer ")) else {
        return false;
    };
    let Ok(mac) = Hmac::<Sha256>::new_from_slice(b"purge-auth") else {
        return false;
    };
    let expected = mac.clone().chain_update(secret.as_bytes()).finalize();
    mac.chain_update(token.as_bytes())
        .verify_slice(&expected.into_bytes())
        .is_ok()
}
