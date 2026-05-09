// ─── Common OAuth2 Helpers ────────────────────────────────────
// PKCE code generation, state generation, and token exchange.

use rand::{Rng, RngCore};
use sha2::{Digest, Sha256};

const CODE_VERIFIER_CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";

/// Generate a PKCE code verifier (128 chars)
pub fn generate_code_verifier() -> String {
    let mut rng = rand::thread_rng();
    let mut chars = Vec::with_capacity(128);
    for _ in 0..128 {
        let idx = rng.gen_range(0..CODE_VERIFIER_CHARSET.len());
        chars.push(CODE_VERIFIER_CHARSET[idx] as char);
    }
    chars.into_iter().collect()
}

/// Generate SHA256 code challenge from verifier (PKCE S256)
pub fn generate_code_challenge(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash)
}

/// Generate a random OAuth state string (hex-encoded 32 bytes)
pub fn generate_state() -> String {
    let mut rng = rand::thread_rng();
    let mut bytes = vec![0u8; 32];
    RngCore::fill_bytes(&mut rng, &mut bytes);
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Perform OAuth2 token exchange (authorization_code grant)
pub async fn exchange_code_for_token(
    client: &reqwest::Client,
    token_url: &str,
    client_id: &str,
    client_secret: &str,
    code: &str,
    code_verifier: &str,
    redirect_uri: &str,
) -> Result<serde_json::Value, reqwest::Error> {
    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("code_verifier", code_verifier),
        ("redirect_uri", redirect_uri),
        ("client_id", client_id),
        ("client_secret", client_secret),
    ];

    let resp = client.post(token_url).form(&params).send().await?;
    let json: serde_json::Value = resp.json().await?;
    Ok(json)
}

/// Refresh an OAuth2 access token
pub async fn refresh_access_token(
    client: &reqwest::Client,
    token_url: &str,
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<serde_json::Value, reqwest::Error> {
    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", client_id),
        ("client_secret", client_secret),
    ];

    let resp = client.post(token_url).form(&params).send().await?;
    let json: serde_json::Value = resp.json().await?;
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_code_verifier_length() {
        let verifier = generate_code_verifier();
        assert_eq!(verifier.len(), 128, "verifier should be 128 chars");
    }

    #[test]
    fn test_generate_code_verifier_charset() {
        let verifier = generate_code_verifier();
        // All chars should be from the allowed set
        for c in verifier.chars() {
            assert!(
                (c.is_ascii_alphanumeric()) || c == '-' || c == '.' || c == '_' || c == '~',
                "illegal char '{}' in verifier",
                c
            );
        }
    }

    #[test]
    fn test_generate_code_verifier_unique() {
        let v1 = generate_code_verifier();
        let v2 = generate_code_verifier();
        assert_ne!(
            v1, v2,
            "two consecutive verifiers should differ (random salt)"
        );
    }

    #[test]
    fn test_generate_code_challenge_deterministic() {
        let verifier = "test-verifier-string-1234567890abcdefghijklmnopqrstuvwxyz";
        let c1 = generate_code_challenge(verifier);
        let c2 = generate_code_challenge(verifier);
        assert_eq!(
            c1, c2,
            "same verifier should produce same challenge"
        );
    }

    #[test]
    fn test_generate_code_challenge_length() {
        let verifier = "short";
        let challenge = generate_code_challenge(verifier);
        // SHA256 base64url = 43 chars (no padding)
        assert_eq!(challenge.len(), 43, "SHA256 base64url challenge should be 43 chars");
    }

    #[test]
    fn test_generate_state_length() {
        let state = generate_state();
        // 32 bytes hex = 64 chars
        assert_eq!(state.len(), 64, "state should be 64 hex chars");
    }

    #[test]
    fn test_generate_state_hex_chars() {
        let state = generate_state();
        for c in state.chars() {
            assert!(c.is_ascii_hexdigit(), "state char '{}' is not hex", c);
        }
    }

    #[test]
    fn test_generate_state_unique() {
        let s1 = generate_state();
        let s2 = generate_state();
        assert_ne!(s1, s2, "states should be unique");
    }
}
