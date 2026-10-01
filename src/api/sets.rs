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

/// A `post_sets` row. `channel_ids` is not a column on the table any more —
/// it lives in `post_set_channels(post_set_id, channel_id)`, so it is read
/// separately and stitched on by [`load_channel_ids`].
#[derive(Debug, sqlx::FromRow)]
struct SetRow {
    id: Uuid,
    name: String,
    description: Option<String>,
    content: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl SetRow {
    /// Shared by every SELECT/RETURNING so the call sites can't drift apart.
    const SELECT_COLS: &'static str = "id, name, description, content, created_at, updated_at";
}

fn to_response(row: SetRow, channel_ids: Vec<Uuid>) -> SetResponse {
    SetResponse {
        id: row.id.to_string(),
        name: row.name,
        description: row.description,
        content: row.content,
        channel_ids: channel_ids.iter().map(|u| u.to_string()).collect(),
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
    }
}

/// Read one set's channel ids from `post_set_channels`.
async fn load_channel_ids(
    db: &crate::db::SqlitePool,
    post_set_id: Uuid,
) -> Result<Vec<Uuid>, AppError> {
    let ids: Vec<String> = sqlx::query_scalar(
        "SELECT channel_id FROM post_set_channels
         WHERE post_set_id = ? ORDER BY rowid",
    )
    .bind(post_set_id)
    .fetch_all(db)
    .await
    .map_err(|e| AppError::Internal(format!("DB error: {e}")))?;

    // A row that doesn't parse is a corrupt channel reference; skip it rather
    // than failing the whole list — the set itself is still usable.
    Ok(ids.iter().filter_map(|id| Uuid::parse_str(id).ok()).collect())
}

/// GET /api/sets
pub async fn list_sets(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<SetResponse>>, AppError> {
    let sql = format!(
        "SELECT {} FROM post_sets WHERE user_id = ? ORDER BY created_at DESC",
        SetRow::SELECT_COLS
    );
    let rows: Vec<SetRow> = sqlx::query_as(&sql)
        .bind(auth.user_id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {e}")))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let channel_ids = load_channel_ids(&state.db, row.id).await?;
        out.push(to_response(row, channel_ids));
    }
    Ok(Json(out))
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

    // The set row and its `post_set_channels` lines go in one transaction —
    // a partial write would leave a template bound to no channels.
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {e}")))?;

    let sql = format!(
        "INSERT INTO post_sets (id, user_id, name, description, content)
         VALUES (?, ?, ?, ?, ?)
         RETURNING {}",
        SetRow::SELECT_COLS
    );
    // `post_sets.id` is TEXT with no DB-side default — SQLite has no
    // gen_random_uuid(), so Rust generates the value.
    let row: SetRow = sqlx::query_as(&sql)
        .bind(Uuid::new_v4())
        .bind(auth.user_id)
        .bind(&body.name)
        .bind(&body.description)
        .bind(&body.content)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {e}")))?;

    for channel_id in &body.channel_ids {
        sqlx::query("INSERT INTO post_set_channels (post_set_id, channel_id) VALUES (?, ?)")
            .bind(row.id)
            .bind(channel_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::Internal(format!("DB error: {e}")))?;
    }

    tx.commit()
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {e}")))?;

    Ok(Json(to_response(row, body.channel_ids)))
}

/// DELETE /api/sets/:id
pub async fn delete_set(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("DELETE FROM post_sets WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(format!("DB error: {e}")))?;

    Ok(Json(serde_json::json!({ "deleted": true })))
}
