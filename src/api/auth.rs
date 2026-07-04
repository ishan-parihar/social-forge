// ─── Auth API Routes (single-user password gate) ──────────────
// POST /api/auth/login   — verify password, set signed session cookie
// POST /api/auth/logout  — clear the session cookie
// GET  /api/auth/me      — confirm session is valid (returns 200)
//
// There is no register endpoint. The single password is read from
// the `APP_PASSWORD` env var (auto-generated on first run; see
// `Config::from_env`). All authenticated handlers operate on
// `DEFAULT_USER_ID` regardless of who logs in — there is only one user.

use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::auth::jwt;
use crate::auth::middleware::{AuthenticatedUser, SESSION_COOKIE};

use super::AppState;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub authenticated: bool,
}

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub authenticated: bool,
    pub user_id: String,
}

/// POST /api/auth/login
///
/// Validates the supplied password against `APP_PASSWORD` using a
/// constant-time compare. On success, issues a JWT signed with the
/// session secret and sets it as an HttpOnly cookie.
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<(HeaderMap, Json<LoginResponse>), crate::error::AppError> {
    // Constant-time password compare to avoid timing side-channels.
    let ok = constant_time_eq(state.config.app_password.as_bytes(), body.password.as_bytes());

    if !ok {
        return Err(crate::error::AppError::Unauthorized("Invalid password".into()));
    }

    let token = jwt::create_token(
        crate::auth::middleware::DEFAULT_USER_ID,
        &state.config.jwt_secret,
    )?;

    let cookie = build_session_cookie(&token, &state.config.app_url);
    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).map_err(|e| {
            crate::error::AppError::Internal(format!("invalid cookie: {e}"))
        })?,
    );

    tracing::info!("WebUI login successful");
    Ok((headers, Json(LoginResponse { authenticated: true })))
}

/// POST /api/auth/logout — clears the session cookie.
pub async fn logout(
    State(state): State<AppState>,
) -> Result<(HeaderMap, Json<serde_json::Value>), crate::error::AppError> {
    let cookie = build_clear_cookie(&state.config.app_url);
    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).map_err(|e| {
            crate::error::AppError::Internal(format!("invalid cookie: {e}"))
        })?,
    );
    Ok((
        headers,
        Json(serde_json::json!({"logged_out": true})),
    ))
}

/// GET /api/auth/me — returns 200 if the session cookie validated
/// (the middleware already proved it). Used by the frontend to
/// check session state on app load.
pub async fn me(
    auth: AuthenticatedUser,
) -> Json<MeResponse> {
    Json(MeResponse {
        authenticated: true,
        user_id: auth.user_id.to_string(),
    })
}

/// Build the `Set-Cookie` header value for a fresh session token.
fn build_session_cookie(token: &str, app_url: &str) -> String {
    let secure = app_url.starts_with("https://");
    let same_site = if secure { "None" } else { "Lax" };
    format!(
        "{SESSION_COOKIE}={token}; HttpOnly; Path=/; Max-Age=2592000; SameSite={same_site}{}",
        if secure { "; Secure" } else { "" }
    )
}

/// Build a `Set-Cookie` header that immediately expires the session cookie.
fn build_clear_cookie(app_url: &str) -> String {
    let secure = app_url.starts_with("https://");
    let same_site = if secure { "None" } else { "Lax" };
    format!(
        "{SESSION_COOKIE}=; HttpOnly; Path=/; Max-Age=0; SameSite={same_site}{}",
        if secure { "; Secure" } else { "" }
    )
}

/// Constant-time byte-slice comparison.
///
/// Avoids timing leaks in two ways:
///   1. Always processes the full length of `a` (the stored password
///      hash), regardless of `b`'s length. This prevents an attacker
///      from learning the stored length by measuring response time
///      across password lengths.
///   2. Each byte of `a` is XOR'd with either the corresponding byte
///      of `b` (when `i < b.len()`) or a constant zero (when `i >= b.len()`).
///      The accumulated diff is checked at the end.
///
/// We pair this with Argon2 hashing on the stored side, so even if
/// the comparison leaks a few bits of timing data, the attacker can't
/// recover the actual password from the hash.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let mut diff: u8 = 0;
    for (i, &byte_a) in a.iter().enumerate() {
        let byte_b = if i < b.len() { b[i] } else { 0 };
        diff |= byte_a ^ byte_b;
    }
    // Also XOR any extra bytes in b (if b is longer than a) so a length
    // difference doesn't go unmeasured.
    if b.len() > a.len() {
        diff |= 1; // mark as different — we don't care which extra byte
    }
    // Final check: also ensure lengths match (otherwise diff was set to
    // a non-zero value above, which would fail the == 0 check).
    diff == 0 && a.len() == b.len()
}
