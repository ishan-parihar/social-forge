// ─── Auth Middleware ───────────────────────────────────────────
// axum middleware that validates JWT from Authorization: Bearer <token>
// and injects the authenticated user ID into request extensions.

use axum::{
    extract::{FromRequestParts, Request},
    http::{request::Parts, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::auth::jwt;

/// JWT secret extracted from AppState and injected into request extensions
/// by a setup layer that wraps all protected routes.
#[derive(Clone)]
pub struct JwtSecret(pub String);

/// Authenticated user extracted from validated JWT
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
}

/// axum middleware: validates JWT from Authorization Bearer header.
/// The JWT secret MUST be injected into request extensions BEFORE this
/// middleware runs (via a JwtSecret extension layer).
pub async fn auth_middleware(
    mut req: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let secret = req
        .extensions()
        .get::<JwtSecret>()
        .map(|s| s.0.clone())
        .ok_or_else(|| {
            tracing::error!("JwtSecret not in extensions — misconfigured middleware stack");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Server configuration error"})),
            )
        })?;

    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Missing Authorization header"})),
            )
        })?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid Authorization format, expected Bearer <token>"})),
            )
        })?;

    let claims = jwt::validate_token(token, &secret).map_err(|e| {
        tracing::debug!("JWT validation failed: {}", e);
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid or expired token"})),
        )
    })?;

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid user ID in token"})),
        )
    })?;

    req.extensions_mut().insert(AuthenticatedUser { user_id });
    Ok(next.run(req).await)
}



/// Extractor that pulls the authenticated user from request extensions
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
