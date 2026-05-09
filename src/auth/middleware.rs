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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::jwt::{create_token, validate_token};
    use axum::{
        body::Body,
        http::Request,
        middleware::{self as axum_middleware},
        response::IntoResponse,
        routing::get,
        Extension, Router,
    };
    use chrono::{Duration, Utc};
    use tower::ServiceExt;

    /// A protected handler that echoes the authenticated user ID
    async fn protected_handler(user: AuthenticatedUser) -> impl IntoResponse {
        Json(json!({"user_id": user.user_id.to_string()}))
    }

    fn test_secret() -> String {
        "test-jwt-secret-that-is-at-least-32-bytes-long-for-hmac".into()
    }

    fn make_test_app(secret: String) -> Router {
        Router::new()
            .route("/", get(protected_handler))
            .layer(axum_middleware::from_fn(auth_middleware))
            .layer(Extension(JwtSecret(secret)))
    }

    fn valid_token() -> String {
        create_token(Uuid::new_v4(), &test_secret()).unwrap()
    }

    #[tokio::test]
    async fn test_missing_auth_header() {
        let app = make_test_app(test_secret());
        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_invalid_auth_format_not_bearer() {
        let app = make_test_app(test_secret());
        let req = Request::builder()
            .uri("/")
            .header("Authorization", "Token some-token")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_valid_token_passes() {
        let user_id = Uuid::new_v4();
        let secret = test_secret();
        let token = create_token(user_id, &secret).unwrap();

        let app = make_test_app(secret);
        let req = Request::builder()
            .uri("/")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_expired_token_rejected() {
        let secret = test_secret();
        // Create a token with expired claims manually
        let now = Utc::now();
        let expired_claims = crate::auth::jwt::Claims {
            sub: Uuid::new_v4().to_string(),
            iat: (now - Duration::hours(2)).timestamp() as usize,
            exp: (now - Duration::hours(1)).timestamp() as usize, // 1 hour ago
        };
        let token = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &expired_claims,
            &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap();

        let app = make_test_app(secret);
        let req = Request::builder()
            .uri("/")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_wrong_secret_rejected() {
        let token = create_token(Uuid::new_v4(), "correct-secret-for-signing").unwrap();

        // Different secret in the middleware
        let app = make_test_app("different-secret-for-validation".into());
        let req = Request::builder()
            .uri("/")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_tampered_token_rejected() {
        let secret = test_secret();
        let token = create_token(Uuid::new_v4(), &secret).unwrap();

        // Tamper the payload
        let parts: Vec<&str> = token.split('.').collect();
        let tampered = if parts.len() == 3 {
            format!("{}.eyJmYWtlIjogInBheWxvYWQifQ.{}", parts[0], parts[2])
        } else {
            token.clone()
        };

        let app = make_test_app(secret);
        let req = Request::builder()
            .uri("/")
            .header("Authorization", format!("Bearer {}", tampered))
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_missing_jwt_secret_extension() {
        // App WITHOUT the JwtSecret extension — should 500
        let app = Router::new()
            .route("/", get(protected_handler))
            .layer(axum_middleware::from_fn(auth_middleware));

        let req = Request::builder()
            .uri("/")
            .header("Authorization", format!("Bearer {}", valid_token()))
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_valid_token_injects_user_id() {
        let secret = test_secret();
        let user_id = Uuid::new_v4();
        let token = create_token(user_id, &secret).unwrap();

        let app = make_test_app(secret);
        let req = Request::builder()
            .uri("/")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // Parse the response body to verify user_id
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            json["user_id"].as_str().unwrap(),
            user_id.to_string()
        );
    }

    #[tokio::test]
    async fn test_invalid_user_id_in_token_rejected() {
        let secret = test_secret();
        // Manually create token with non-UUID sub
        let claims = crate::auth::jwt::Claims {
            sub: "not-a-uuid".into(),
            iat: Utc::now().timestamp() as usize,
            exp: (Utc::now() + Duration::days(1)).timestamp() as usize,
        };
        let token = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap();

        let app = make_test_app(secret);
        let req = Request::builder()
            .uri("/")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}
