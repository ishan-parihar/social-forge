// ─── MCP Tags Tools ─────────────────────────────────────────────
// CRUD tools for user-defined tags.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;

// ── Input Types ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TagCreateInput {
    /// JWT auth token
    pub token: String,
    /// Tag name (required)
    pub name: String,
    /// Hex color code (e.g. "#6366f1", defaults to indigo)
    pub color: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TagListInput {
    /// JWT auth token
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TagGetInput {
    /// JWT auth token
    pub token: String,
    /// Tag ID (UUID)
    pub tag_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TagUpdateInput {
    /// JWT auth token
    pub token: String,
    /// Tag ID (UUID)
    pub tag_id: String,
    /// New tag name (optional)
    pub name: Option<String>,
    /// New hex color code (optional)
    pub color: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TagDeleteInput {
    /// JWT auth token
    pub token: String,
    /// Tag ID (UUID)
    pub tag_id: String,
}

// ── Handlers ─────────────────────────────────────────────────

/// Create a new tag
pub async fn handle_tag_create(
    state: &AppState,
    input: &TagCreateInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err("Tag name cannot be empty".into());
    }
    let color = input.color.as_deref().unwrap_or("#6366f1");

    let tag = sqlx::query!(
        r#"INSERT INTO tags (user_id, name, color)
           VALUES ($1, $2, $3)
           RETURNING id, name, color, created_at, updated_at"#,
        user_id,
        name,
        color,
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| format!("Failed to create tag: {e}"))?;

    Ok(Json(serde_json::json!({
        "data": {
            "id": tag.id.to_string(),
            "name": tag.name,
            "color": tag.color,
            "created_at": tag.created_at.to_rfc3339(),
            "updated_at": tag.updated_at.to_rfc3339(),
        }
    })))
}

/// List all tags for the user
pub async fn handle_tag_list(
    state: &AppState,
    _input: &TagListInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;

    let tags = sqlx::query!(
        r#"SELECT id, name, color, created_at, updated_at
           FROM tags WHERE user_id = $1 ORDER BY name ASC"#,
        user_id,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| format!("Failed to list tags: {e}"))?;

    let data: Vec<serde_json::Value> = tags
        .into_iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id.to_string(),
                "name": t.name,
                "color": t.color,
                "created_at": t.created_at.to_rfc3339(),
                "updated_at": t.updated_at.to_rfc3339(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "data": data })))
}

/// Get a single tag by ID
pub async fn handle_tag_get(
    state: &AppState,
    input: &TagGetInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let tag_id = Uuid::parse_str(&input.tag_id)
        .map_err(|_| format!("Invalid tag ID: {}", input.tag_id))?;

    let tag = sqlx::query!(
        r#"SELECT id, name, color, created_at, updated_at
           FROM tags WHERE id = $1 AND user_id = $2"#,
        tag_id,
        user_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| format!("Failed to get tag: {e}"))?
    .ok_or_else(|| "Tag not found".to_string())?;

    Ok(Json(serde_json::json!({
        "data": {
            "id": tag.id.to_string(),
            "name": tag.name,
            "color": tag.color,
            "created_at": tag.created_at.to_rfc3339(),
            "updated_at": tag.updated_at.to_rfc3339(),
        }
    })))
}

/// Update an existing tag
pub async fn handle_tag_update(
    state: &AppState,
    input: &TagUpdateInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let tag_id = Uuid::parse_str(&input.tag_id)
        .map_err(|_| format!("Invalid tag ID: {}", input.tag_id))?;

    let tag = sqlx::query!(
        r#"UPDATE tags SET
              name = COALESCE($3, name),
              color = COALESCE($4, color),
              updated_at = now()
           WHERE id = $1 AND user_id = $2
           RETURNING id, name, color, created_at, updated_at"#,
        tag_id,
        user_id,
        input.name.as_deref(),
        input.color.as_deref(),
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| format!("Failed to update tag: {e}"))?
    .ok_or_else(|| "Tag not found".to_string())?;

    Ok(Json(serde_json::json!({
        "data": {
            "id": tag.id.to_string(),
            "name": tag.name,
            "color": tag.color,
            "created_at": tag.created_at.to_rfc3339(),
            "updated_at": tag.updated_at.to_rfc3339(),
        }
    })))
}

/// Delete a tag
pub async fn handle_tag_delete(
    state: &AppState,
    input: &TagDeleteInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let tag_id = Uuid::parse_str(&input.tag_id)
        .map_err(|_| format!("Invalid tag ID: {}", input.tag_id))?;

    let result = sqlx::query!(
        "DELETE FROM tags WHERE id = $1 AND user_id = $2",
        tag_id,
        user_id,
    )
    .execute(&state.db)
    .await
    .map_err(|e| format!("Failed to delete tag: {e}"))?;

    if result.rows_affected() == 0 {
        return Err("Tag not found".to_string());
    }

    Ok(Json(serde_json::json!({ "deleted": true })))
}
