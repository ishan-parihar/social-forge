// ─── Tags API Routes ─────────────────────────────────────────
// CRUD for user-defined tags attached to posts.

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::error::AppError;

use super::AppState;

// ── Types ────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tag {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub color: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct TagResponse {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Tag> for TagResponse {
    fn from(t: Tag) -> Self {
        Self {
            id: t.id,
            name: t.name,
            color: t.color,
            created_at: t.created_at.to_rfc3339(),
            updated_at: t.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TagCreateRequest {
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TagUpdateRequest {
    pub name: Option<String>,
    pub color: Option<String>,
}

// ── Handlers ─────────────────────────────────────────────────

/// GET /api/tags
pub async fn list(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<TagResponse>>, AppError> {
    let tags = sqlx::query_as::<_, Tag>(
        r#"SELECT id, user_id, name, color, created_at, updated_at
           FROM tags WHERE user_id = $1 ORDER BY name ASC"#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(tags.into_iter().map(TagResponse::from).collect()))
}

/// POST /api/tags
pub async fn create(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(input): Json<TagCreateRequest>,
) -> Result<Json<TagResponse>, AppError> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("Tag name cannot be empty".into()));
    }
    let color = input.color.unwrap_or_else(|| "#6366f1".into());

    let tag = sqlx::query_as::<_, Tag>(
        r#"INSERT INTO tags (user_id, name, color)
           VALUES ($1, $2, $3)
           RETURNING id, user_id, name, color, created_at, updated_at"#,
    )
    .bind(auth.user_id)
    .bind(&name)
    .bind(&color)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(TagResponse::from(tag)))
}

/// GET /api/tags/{id}
pub async fn get(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<TagResponse>, AppError> {
    let tag = sqlx::query_as::<_, Tag>(
        r#"SELECT id, user_id, name, color, created_at, updated_at
           FROM tags WHERE id = $1 AND user_id = $2"#,
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Tag not found".into()))?;

    Ok(Json(TagResponse::from(tag)))
}

/// PUT /api/tags/{id}
pub async fn update(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(input): Json<TagUpdateRequest>,
) -> Result<Json<TagResponse>, AppError> {
    // First check tag exists and belongs to user
    let _existing = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM tags WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Tag not found".into()))?;

    // Build dynamic update
    let name = input.name.as_deref();
    let color = input.color.as_deref();

    let tag = sqlx::query_as::<_, Tag>(
        r#"UPDATE tags SET
              name = COALESCE($3, name),
              color = COALESCE($4, color),
              updated_at = now()
           WHERE id = $1 AND user_id = $2
           RETURNING id, user_id, name, color, created_at, updated_at"#,
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(name)
    .bind(color)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(TagResponse::from(tag)))
}

/// DELETE /api/tags/{id}
pub async fn delete(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = sqlx::query(
        "DELETE FROM tags WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Tag not found".into()));
    }

    Ok(Json(serde_json::json!({"deleted": true})))
}
