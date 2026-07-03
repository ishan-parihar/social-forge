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

/// Manual constant-time byte-slice comparison. Avoids pulling in the
/// `subtle` crate just for one call site.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}
