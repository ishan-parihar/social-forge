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
        let (status, message, code) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone(), "not_found"),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone(), "unauthorized"),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone(), "bad_request"),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone(), "conflict"),
            AppError::Database(e) => {
                tracing::error!("Database error: {:?}", e);
                // Include a stable error code so the frontend can branch on it
                // without parsing the human-readable message. We do NOT leak
                // SQL details — just the sqlx::Error discriminant.
                //
                // Note: sqlx::Error variants vary slightly across versions.
                // We list the ones that exist in sqlx 0.8 and use a fallback
                // for everything else. Pool-related errors are represented
                // by PoolTimedOut / PoolClosed (there is no generic `Pool`
                // variant in sqlx::Error itself — pool errors come through
                // these specific variants or as Database/Configuration).
                let code = match e {
                    sqlx::Error::RowNotFound => "db_row_not_found",
                    sqlx::Error::TypeNotFound { .. } => "db_type_not_found",
                    sqlx::Error::ColumnNotFound(_) => "db_column_not_found",
                    sqlx::Error::ColumnDecode { .. } => "db_column_decode",
                    sqlx::Error::Decode(_) => "db_decode",
                    sqlx::Error::PoolTimedOut => "db_pool_timed_out",
                    sqlx::Error::PoolClosed => "db_pool_closed",
                    sqlx::Error::WorkerCrashed => "db_worker_crashed",
                    sqlx::Error::Database(_) => "db_database",
                    sqlx::Error::Io(_) => "db_io",
                    sqlx::Error::Tls(_) => "db_tls",
                    sqlx::Error::Protocol(_) => "db_protocol",
                    sqlx::Error::Configuration(_) => "db_configuration",
                    _ => "db_unknown",
                };
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error".into(), code)
            }
            AppError::Auth(e) => {
                tracing::error!("Auth error: {:?}", e);
                (StatusCode::UNAUTHORIZED, "Invalid token".into(), "auth")
            }
            AppError::Hash(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Hashing error".into(), "hash"),
            AppError::Provider(msg) => (StatusCode::BAD_GATEWAY, msg.clone(), "provider"),
            AppError::TokenExpired => (StatusCode::UNAUTHORIZED, "Token expired".into(), "token_expired"),
            AppError::RateLimited(msg) => (StatusCode::TOO_MANY_REQUESTS, msg.clone(), "rate_limited"),
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal error".into(), "internal")
            }
        };

        (status, Json(json!({ "error": message, "code": code }))).into_response()
    }
}
