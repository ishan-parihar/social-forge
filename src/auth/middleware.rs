// ─── Auth Middleware (single-user password gate) ──────────────
//
// Social Forge is a local-deployment tool: one user, one password.
// There is no user-registration, no user table lookups, no per-user
// permissions. The WebUI is gated by a single password set via the
// `APP_PASSWORD` env var. After a successful POST /api/auth/login,
// the server issues an HttpOnly signed session cookie (`sf_session`)
// containing a JWT whose `sub` is always `DEFAULT_USER_ID`.
//
// The CLI and MCP stdio paths are local (shell access already
// implies trust) and bypass this gate — they call `resolve_first_user`
// which returns `DEFAULT_USER_ID` directly.

use axum::{
    extract::{FromRequestParts, Request, State},
    http::{request::Parts, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::auth::jwt;

/// The single local user. All data in the DB is owned by this id.
/// Kept as a stable constant so existing rows survive restarts and
/// so FK constraints have a deterministic target.
pub const DEFAULT_USER_ID: Uuid = Uuid::from_u128(0x22222222_2222_2222_2222_222222222222);

/// Cookie name used for the session token.
pub const SESSION_COOKIE: &str = "sf_session";

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
}

/// Shared state injected into `auth_middleware` via `from_fn_with_state`.
/// Carries the HMAC secret used to sign session cookies.
#[derive(Clone)]
pub struct AuthState {
    pub session_secret: String,
}

/// Validate the `sf_session` cookie against the configured JWT secret.
/// Returns 401 if missing/invalid — the frontend's `+layout.ts` guard
/// then redirects to `/login`.
pub async fn auth_middleware(
    State(auth): State<AuthState>,
    mut req: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let cookie_header = req
        .headers()
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let token = extract_cookie(cookie_header, SESSION_COOKIE);
    let user_id = match token.and_then(|t| jwt::validate_token(t, &auth.session_secret).ok()) {
        Some(claims) => Uuid::parse_str(&claims.sub).unwrap_or(DEFAULT_USER_ID),
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Not authenticated", "code": "no_session"})),
            ));
        }
    };

    req.extensions_mut()
        .insert(AuthenticatedUser { user_id });
    Ok(next.run(req).await)
}

/// Parse `name=value` out of a `Cookie:` header value.
pub fn extract_cookie(header: &str, name: &str) -> Option<&str> {
    header
        .split(';')
        .map(|p| p.trim())
        .find_map(|p| p.strip_prefix(&format!("{name}=")))
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
