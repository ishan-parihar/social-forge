// ─── JWT Auth ──────────────────────────────────────────────────
// Token creation and validation using jsonwebtoken crate.
// Also hosts password hashing helpers (argon2) used by auth.rs.

#[cfg(test)]
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT claims payload
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String, // user UUID
    pub exp: usize,
    pub iat: usize,
}

/// Hash a password with Argon2.
///
/// Only used by tests now — the single-user password gate uses
/// `constant_time_eq` against `APP_PASSWORD` directly (no DB hash).
/// Kept for backward compat + tests; gated behind `#[cfg(test)]`.
#[cfg(test)]
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

/// Verify a password against an Argon2 hash.
///
/// Only used by tests — see `hash_password` docs.
#[cfg(test)]
pub fn verify_password(password: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
    let parsed = PasswordHash::new(hash)?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

/// Create a JWT token for a user
pub fn create_token(user_id: Uuid, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        iat: now.timestamp() as usize,
        exp: (now + Duration::days(30)).timestamp() as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

/// Validate a JWT token and return the claims
pub fn validate_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password_round_trip() {
        let password = "my_secure_password_123!";
        let hash = hash_password(password).expect("hash should succeed");
        assert!(verify_password(password, &hash).expect("verify should succeed"));
    }

    #[test]
    fn test_wrong_password_rejected() {
        let hash = hash_password("correct_password").expect("hash should succeed");
        let result = verify_password("wrong_password", &hash).expect("verify should succeed");
        assert!(!result, "wrong password should not verify");
    }

    #[test]
    fn test_different_hashes_produced() {
        let pw = "same_password";
        let hash1 = hash_password(pw).expect("hash1");
        let hash2 = hash_password(pw).expect("hash2");
        // Argon2 generates random salt each time
        assert_ne!(hash1, hash2, "hashes should differ due to random salt");
    }

    #[test]
    fn test_invalid_hash_fails() {
        let result = verify_password("anything", "not-a-valid-hash");
        assert!(result.is_err(), "invalid hash should error");
    }

    #[test]
    fn test_create_and_validate_token() {
        let user_id = Uuid::new_v4();
        let secret = "test-secret-key-that-is-long-enough-for-hmac";
        
        let token = create_token(user_id, secret).expect("create token");
        let claims = validate_token(&token, secret).expect("validate token");
        
        assert_eq!(claims.sub, user_id.to_string(), "subject should match user_id");
        assert!(claims.exp > 0, "expiry should be set");
        assert!(claims.iat > 0, "issued-at should be set");
    }

    #[test]
    fn test_wrong_secret_rejected() {
        let token = create_token(Uuid::new_v4(), "correct-secret-for-signing-tokens")
            .expect("create token");
        let result = validate_token(&token, "wrong-secret-for-validation");
        assert!(result.is_err(), "wrong secret should fail validation");
    }

    #[test]
    fn test_tampered_token_rejected() {
        let secret = "test-secret-for-hmac";
        let token = create_token(Uuid::new_v4(), secret).expect("create token");
        
        // Tamper with the payload portion
        let mut parts: Vec<&str> = token.split('.').collect();
        if parts.len() == 3 {
            parts[1] = "eyJzdWIiOiJmYWtlIn0"; // base64 of {"sub":"fake"}
            let tampered = parts.join(".");
            let result = validate_token(&tampered, secret);
            assert!(result.is_err(), "tampered token should fail validation");
        }
    }

    #[test]
    fn test_token_contains_correct_user_id() {
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let secret = "another-test-key-for-validating-subject";
        
        let token = create_token(user_id, secret).expect("create token");
        let claims = validate_token(&token, secret).expect("validate token");
        
        assert_eq!(claims.sub, "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_token_expiry_is_about_30_days() {
        let user_id = Uuid::new_v4();
        let secret = "test-key-for-expiry-check";
        
        let token = create_token(user_id, secret).expect("create token");
        let claims = validate_token(&token, secret).expect("validate token");
        
        let now = Utc::now().timestamp() as usize;
        let thirty_days_secs = 30 * 24 * 60 * 60;
        
        // Allow 5 second tolerance for test execution time
        assert!(
            claims.exp >= now + thirty_days_secs - 5,
            "expiry should be ~30 days from now (exp={}, now={})",
            claims.exp, now
        );
        assert!(
            claims.exp <= now + thirty_days_secs + 5,
            "expiry should not exceed 30 days + 5s tolerance"
        );
    }

    #[test]
    fn test_iat_is_recent() {
        let user_id = Uuid::new_v4();
        let secret = "test-key-for-iat-check";
        
        let token = create_token(user_id, secret).expect("create token");
        let claims = validate_token(&token, secret).expect("validate token");
        
        let now = Utc::now().timestamp() as usize;
        assert!(
            claims.iat <= now && claims.iat >= now - 5,
            "iat should be within 5s of now"
        );
    }
}
