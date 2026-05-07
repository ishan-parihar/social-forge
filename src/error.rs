// ─── Unified Error Types ───────────────────────────────────────
// All errors in the system flow through AppError.
// Each variant maps to an HTTP status code for the API layer.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Auth error: {0}")]
    Auth(#[from] jsonwebtoken::errors::Error),

    #[error("Hashing error: {0}")]
    Hash(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Token expired")]
    TokenExpired,

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<argon2::password_hash::Error> for AppError {
    fn from(e: argon2::password_hash::Error) -> Self {
        AppError::Hash(e.to_string())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}

impl From<crate::social::ProviderError> for AppError {
    fn from(e: crate::social::ProviderError) -> Self {
        match e {
            crate::social::ProviderError::Auth(m) => AppError::Unauthorized(m),
            crate::social::ProviderError::Api(m) => AppError::Provider(m),
            crate::social::ProviderError::TokenExpired => AppError::TokenExpired,
            crate::social::ProviderError::RateLimited(m) => AppError::RateLimited(m),
            crate::social::ProviderError::InvalidRequest(m) => AppError::BadRequest(m),
            crate::social::ProviderError::Network(e) => {
                AppError::Internal(format!("Provider network error: {e}"))
            }
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            AppError::Database(e) => {
                tracing::error!("Database error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error".into())
            }
            AppError::Auth(e) => {
                tracing::error!("Auth error: {:?}", e);
                (StatusCode::UNAUTHORIZED, "Invalid token".into())
            }
            AppError::Hash(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Hashing error".into()),
            AppError::Provider(msg) => (StatusCode::BAD_GATEWAY, msg.clone()),
            AppError::TokenExpired => (StatusCode::UNAUTHORIZED, "Token expired".into()),
            AppError::RateLimited(msg) => (StatusCode::TOO_MANY_REQUESTS, msg.clone()),
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal error".into())
            }
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
