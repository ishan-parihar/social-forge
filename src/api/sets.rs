// ─── Post Sets API Routes ─────────────────────────────────────
// CRUD for reusable post templates (sets). Stored server-side so they
// sync across devices and can be loaded into the composer with one click.

use axum::{extract::{Path, State}, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::error::AppError;

use super::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateSetRequest {
    pub name: String,
    pub description: Option<String>,
    pub content: serde_json::Value,
    pub channel_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct SetResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub content: serde_json::Value,
    pub channel_ids: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, sqlx::FromRow)]
struct SetRow {
    id: Uuid,
    name: String,
    description: Option<String>,
    content: serde_json::Value,
    channel_ids: Vec<Uuid>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<SetRow> for SetResponse {
    fn from(r: SetRow) -> Self {
        Self {
            id: r.id.to_string(),
            name: r.name,
            description: r.description,
            content: r.content,
            channel_ids: r.channel_ids.iter().map(|u| u.to_string()).collect(),
            created_at: r.created_at.to_rfc3339(),
            updated_at: r.updated_at.to_rfc3339(),
        }
    }
}

/// GET /api/sets
pub async fn list_sets(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<SetResponse>>, AppError> {
    let rows: Vec<SetRow> = sqlx::query_as(
        "SELECT id, name, description, content, channel_ids, created_at, updated_at
         FROM post_sets WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("DB error: {e}")))?;

    Ok(Json(rows.into_iter().map(SetResponse::from).collect()))
}

/// POST /api/sets
pub async fn create_set(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<CreateSetRequest>,
) -> Result<Json<SetResponse>, AppError> {
    if body.name.trim().is_empty() {
        return Err(AppError::BadRequest("Name is required".into()));
    }

    let row: SetRow = sqlx::query_as(
        "INSERT INTO post_sets (user_id, name, description, content, channel_ids)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, name, description, content, channel_ids, created_at, updated_at",
    )
    .bind(auth.user_id)
    .bind(&body.name)
    .bind(&body.description)
    .bind(&body.content)
    .bind(&body.channel_ids)
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("DB error: {e}")))?;

    Ok(Json(SetResponse::from(row)))
}

/// DELETE /api/sets/:id
pub async fn delete_set(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("DELETE FROM post_sets WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {e}")))?;

    Ok(Json(serde_json::json!({ "deleted": true })))
}
