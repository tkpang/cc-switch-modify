//! PKCE (Proof Key for Public Clients) implementation per RFC 7636.
//!
//! Provides utilities for generating code verifiers and computing S256 challenges
//! used in OAuth 2.0 OIDC flows.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;
use sha2::{Digest, Sha256};

const UNRESERVED: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";

/// Generate a PKCE code verifier.
///
/// Returns a fixed 64-character random string containing only unreserved characters:
/// A-Z a-z 0-9 - . _ ~
/// (64 chars is within the RFC 7636 §4.1 allowed range of 43–128.)
pub fn generate_verifier() -> String {
    let mut rng = rand::thread_rng();
    (0..64)
        .map(|_| UNRESERVED[rng.gen_range(0..UNRESERVED.len())] as char)
        .collect()
}

/// Compute the S256 challenge from a verifier.
///
/// Takes the SHA-256 hash of the verifier and encodes it as base64url without padding.
pub fn challenge_s256(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifier_length_and_charset() {
        let v = generate_verifier();
        assert!((43..=128).contains(&v.len()));
        assert!(v
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-._~".contains(c)));
    }

    #[test]
    fn challenge_is_known_s256() {
        // RFC 7636 附录 B 向量
        let v = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(challenge_s256(v), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }
}
