//! Constant-time HMAC verification shared across the workers: shared-secret
//! bearer tokens and GitHub webhook signatures. Pure and natively tested.

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

/// GitHub signs the raw body with HMAC-SHA256 (`X-Hub-Signature-256:
/// sha256=<hex>`); comparison is constant-time.
pub fn verify_signature(secret: &str, body: &[u8], header: Option<&str>) -> bool {
    let Some(expected) = header
        .and_then(|value| value.strip_prefix("sha256="))
        .and_then(decode_hex)
    else {
        return false;
    };
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(body);
    mac.verify_slice(&expected).is_ok()
}

fn decode_hex(hex: &str) -> Option<Vec<u8>> {
    if hex.is_empty() || !hex.len().is_multiple_of(2) {
        return None;
    }
    let digit = |byte: u8| char::from(byte).to_digit(16);
    hex.as_bytes()
        .chunks(2)
        .map(|pair| u8::try_from(digit(pair[0])? * 16 + digit(pair[1])?).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{verify_bearer, verify_signature};

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

    // GitHub's documented webhook validation example, so the implementation
    // is checked against the spec rather than against itself.
    const DOC_SECRET: &str = "It's a Secret to Everybody";
    const DOC_BODY: &[u8] = b"Hello, World!";
    const DOC_SIGNATURE: &str =
        "sha256=757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e17";

    #[test]
    fn valid_signature_verifies() {
        assert!(verify_signature(DOC_SECRET, DOC_BODY, Some(DOC_SIGNATURE)));
    }

    #[test]
    fn tampered_body_and_wrong_secret_fail() {
        assert!(!verify_signature(
            DOC_SECRET,
            b"Hello, World?",
            Some(DOC_SIGNATURE)
        ));
        assert!(!verify_signature(
            "wrong secret",
            DOC_BODY,
            Some(DOC_SIGNATURE)
        ));
    }

    #[test]
    fn missing_or_malformed_signature_headers_fail() {
        assert!(!verify_signature(DOC_SECRET, DOC_BODY, None));
        assert!(!verify_signature(DOC_SECRET, DOC_BODY, Some("")));
        assert!(!verify_signature(DOC_SECRET, DOC_BODY, Some("sha256=")));
        assert!(!verify_signature(DOC_SECRET, DOC_BODY, Some("sha256=zz")));
        assert!(!verify_signature(
            DOC_SECRET,
            DOC_BODY,
            Some("sha256=757107")
        ));
        // sha1= is the legacy header; only sha256 is accepted
        assert!(!verify_signature(
            DOC_SECRET,
            DOC_BODY,
            Some("sha1=deadbeef")
        ));
        // multibyte input must not panic the hex decoder
        assert!(!verify_signature(DOC_SECRET, DOC_BODY, Some("sha256=éé")));
    }
}
