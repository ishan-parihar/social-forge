// ─── Auth Middleware (single-user password gate) ──────────────
//
// Social Forge is a local-deployment tool: one user, one password.
// There is no user-registration, no user table lookups, no per-user
// permissions. The WebUI is gated by a single password set via the
// `APP_PASSWORD` env var. After a successful POST /api/auth/login,
// the server issues an HttpOnly signed session cookie (`sf_session`)
// containing a JWT whose `sub` is always `DEFAULT_USER_ID`.
//
// Programmatic clients skip the password and present an issued API key
// as `Authorization: Bearer sf_…` instead. Both paths resolve to the
// same single user — a key is another credential, not another identity.
//
// The CLI and MCP stdio paths are local (shell access already
// implies trust) and bypass this gate — they call `resolve_first_user`
// which returns `DEFAULT_USER_ID` directly.

use axum::{
    extract::{FromRequestParts, Request, State},
    http::{header, request::Parts, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

use crate::auth::jwt;
use crate::db::PgPool;

/// The single local user. All data in the DB is owned by this id.
/// Kept as a stable constant so existing rows survive restarts and
/// so FK constraints have a deterministic target.
pub const DEFAULT_USER_ID: Uuid = Uuid::from_u128(0x22222222_2222_2222_2222_222222222222);

/// Cookie name used for the session token.
pub const SESSION_COOKIE: &str = "sf_session";

/// Every issued API key starts with this marker. Programmatic clients
/// send `Authorization: Bearer sf_…`.
pub const API_KEY_PREFIX: &str = "sf_";

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
}

/// Shared state injected into `auth_middleware` via `from_fn_with_state`.
/// Carries the HMAC secret used to sign session cookies and the pool
/// used to look up API keys.
#[derive(Clone)]
pub struct AuthState {
    pub session_secret: String,
    pub db: PgPool,
}

/// The single SHA-256 scheme for API keys at rest. Shared by issuance
/// (`api::developer::generate_api_key`) and verification (below) so a
/// key can never be hashed one way and looked up another — do not add a
/// second hashing helper.
pub fn hash_api_key(raw_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_key.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Verify a raw `sf_…` key against `api_keys` and stamp `last_used_at`.
///
/// Postgres does the digest equality (`key_hash = $1`), so a Rust-side
/// constant-time compare would be redundant — and the compared value is
/// a 256-bit digest, never a usable credential, so a timing signal on
/// the digest does not leak the key.
///
/// `None` means unknown, revoked, or expired — all three are 401 to the
/// caller, so an attacker cannot tell them apart.
///
/// ponytail: no index on `key_hash` (migration 011 only indexes user_id).
/// Seq scan over a handful of keys is fine; add a unique index if an
/// operator ever holds thousands.
pub async fn verify_api_key(pool: &PgPool, raw_key: &str) -> Option<Uuid> {
    let row = sqlx::query(
        r#"
        UPDATE api_keys
        SET last_used_at = NOW()
        WHERE key_hash = $1
          AND is_active = true
          AND (expires_at IS NULL OR expires_at > NOW())
        RETURNING user_id
        "#,
    )
    .bind(hash_api_key(raw_key))
    .fetch_one(pool)
    .await
    .ok()?;

    row.try_get::<Uuid, _>("user_id").ok()
}

/// Pull the credential out of an `Authorization: Bearer <token>` header.
pub fn extract_bearer(header_value: &str) -> Option<&str> {
    let (scheme, value) = header_value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

/// 401 body shared by both auth paths.
fn unauthorized(code: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({"error": "Not authenticated", "code": code})),
    )
}

/// Authenticate a request by API key *or* session cookie, then inject
/// `AuthenticatedUser` for downstream handlers.
///
/// Two paths, checked in order:
///
/// 1. `Authorization: Bearer sf_…` — a key issued by
///    `POST /api/developer/api-keys`, verified against `api_keys`.
/// 2. `sf_session` cookie — the WebUI path, validated against the JWT
///    secret. Unchanged.
///
/// A present-but-rejected `Authorization` header short-circuits to 401
/// rather than falling through to the cookie path: silently downgrading a
/// bad key to a session would mask the credential error.
pub async fn auth_middleware(
    State(auth): State<AuthState>,
    mut req: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let bearer = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(extract_bearer)
        .map(str::to_owned);

    if let Some(bearer) = bearer {
        let user_id = if bearer.starts_with(API_KEY_PREFIX) {
            verify_api_key(&auth.db, &bearer).await
        } else {
            None
        };
        let Some(user_id) = user_id else {
            return Err(unauthorized("invalid_api_key"));
        };
        req.extensions_mut()
            .insert(AuthenticatedUser { user_id });
        return Ok(next.run(req).await);
    }

    let cookie_header = req
        .headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let token = extract_cookie(cookie_header, SESSION_COOKIE);
    let user_id = match token.and_then(|t| jwt::validate_token(t, &auth.session_secret).ok()) {
        Some(claims) => Uuid::parse_str(&claims.sub).unwrap_or(DEFAULT_USER_ID),
        None => return Err(unauthorized("no_session")),
    };

    req.extensions_mut()
        .insert(AuthenticatedUser { user_id });
    Ok(next.run(req).await)
}

/// Parse `name=value` out of a `Cookie:` header value.
pub fn extract_cookie<'a>(header: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name}=");
    header
        .split(';')
        .map(|p| p.trim())
        .find_map(|p| p.strip_prefix(&prefix))
}

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthenticatedUser>()
            .cloned()
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"error": "Not authenticated"})),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the at-rest scheme to SHA-256 hex. If someone swaps this for
    /// another digest, existing rows stop verifying and this test says why.
    #[test]
    fn hash_api_key_matches_known_sha256_vectors() {
        assert_eq!(
            hash_api_key("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hash_api_key(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    /// The property `verify_api_key` relies on: two keys collide only when
    /// they are byte-identical, so a one-character difference never
    /// verifies against a stored digest.
    #[test]
    fn hash_api_key_distinguishes_keys_that_differ_by_one_character() {
        let stored = hash_api_key("sf_0123456789abcdef0123456789abcdef");

        assert_ne!(
            stored,
            hash_api_key("sf_0123456789abcdef0123456789abcdee"),
            "a one-character key change must not produce the stored digest"
        );
        assert_eq!(
            stored,
            hash_api_key("sf_0123456789abcdef0123456789abcdef"),
            "the same key must always produce the same digest"
        );
    }

    #[test]
    fn extract_bearer_returns_the_token() {
        assert_eq!(
            extract_bearer("Bearer sf_abc123"),
            Some("sf_abc123"),
            "the sf_ credential should be returned verbatim"
        );
    }

    #[test]
    fn extract_bearer_rejects_other_schemes_and_empty_tokens() {
        assert_eq!(extract_bearer("Basic dXNlcjpwYXNz"), None);
        assert_eq!(extract_bearer("Bearer "), None);
        assert_eq!(extract_bearer(""), None);
    }
}
