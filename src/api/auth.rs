// ─── Auth API Routes ───────────────────────────────────────────
// POST /api/auth/register — create account
// POST /api/auth/login   — authenticate, receive JWT
// GET  /api/auth/me      — current user profile

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::jwt;
use crate::auth::middleware::AuthenticatedUser;
use crate::db::queries;


use super::AppState;

// ── Request/Response Types ───────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub name: String,
}

// ── Handlers ─────────────────────────────────────────────────

/// POST /api/auth/register
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, crate::error::AppError> {
    // Rate limit by email
    // Rate limit by email
    state.rate_limiter.check(&body.email).await.map_err(crate::error::AppError::RateLimited)?;

    // Validate input
    if body.email.is_empty() || !body.email.contains('@') {
        return Err(crate::error::AppError::BadRequest("Invalid email".into()));
    }
    if body.password.len() < 6 {
        return Err(crate::error::AppError::BadRequest(
            "Password must be at least 6 characters".into(),
        ));
    }
    if body.name.is_empty() {
        return Err(crate::error::AppError::BadRequest("Name is required".into()));
    }

    // Check if user exists
    if queries::get_user_by_email(&state.db, &body.email)
        .await?
        .is_some()
    {
        return Err(crate::error::AppError::Conflict("Email already registered".into()));
    }

    // Hash password and create user
    let hash = jwt::hash_password(&body.password)?;
    let user = queries::create_user(&state.db, &body.email, &hash, &body.name).await?;

    let token = jwt::create_token(user.id, &state.config.jwt_secret)?;

    tracing::info!("User registered: {} ({})", user.email, user.id);
    Ok(Json(AuthResponse {
        token,
        user: UserResponse {
            id: user.id,
            email: user.email,
            name: user.name,
        },
    }))
}

/// POST /api/auth/login
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, crate::error::AppError> {
    // Rate limit by email
    // Rate limit by email
    state.rate_limiter.check(&body.email).await.map_err(crate::error::AppError::RateLimited)?;

    let user = queries::get_user_by_email(&state.db, &body.email)
        .await?
        .ok_or_else(|| crate::error::AppError::Unauthorized("Invalid email or password".into()))?;

    let valid = jwt::verify_password(&body.password, &user.password)?;
    if !valid {
        return Err(crate::error::AppError::Unauthorized("Invalid email or password".into()));
    }

    let token = jwt::create_token(user.id, &state.config.jwt_secret)?;

    tracing::info!("User logged in: {}", user.email);
    Ok(Json(AuthResponse {
        token,
        user: UserResponse {
            id: user.id,
            email: user.email,
            name: user.name,
        },
    }))
}

/// GET /api/auth/me — returns the default user, auto-creating if needed
pub async fn me(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<UserResponse>, crate::error::AppError> {
    let user = match queries::get_user_by_id(&state.db, auth.user_id).await? {
        Some(u) => u,
        None => {
            let hash = crate::auth::jwt::hash_password("socialforge")?;
            queries::create_user_with_id(&state.db, auth.user_id, "user@socialforge.local", &hash, "User").await?
        }
    };

    Ok(Json(UserResponse {
        id: user.id,
        email: user.email,
        name: user.name,
    }))
}
