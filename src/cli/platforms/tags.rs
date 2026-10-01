use crate::api::AppState;
use crate::cli::TagsAction;

use crate::db::types::EpochUtc;

pub async fn handle(action: TagsAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        TagsAction::List => {
            let user_id = match crate::mcp::tools_posts::resolve_first_user(state).await {
                Ok(id) => id,
                Err(e) => return Err(anyhow::anyhow!("Auth error: {e}")),
            };
            let tags = match sqlx::query!(r#"SELECT id as "id!: String", name, color,
                                                created_at as "created_at: EpochUtc",
                                                updated_at as "updated_at: EpochUtc"
                                         FROM tags WHERE user_id = ? ORDER BY name"#, user_id)
                .fetch_all(&state.db).await
            {
                Ok(t) => t,
                Err(e) => return Err(anyhow::anyhow!("DB error: {e}")),
            };
            let data: Vec<_> = tags.into_iter().map(|t| serde_json::json!({
                "id": t.id, "name": t.name, "color": t.color,
                "created_at": t.created_at.0.to_rfc3339(), "updated_at": t.updated_at.0.to_rfc3339(),
            })).collect();
            Ok(serde_json::json!({ "data": data }))
        }
        TagsAction::Create { name, color } => {
            let user_id = match crate::mcp::tools_posts::resolve_first_user(state).await {
                Ok(id) => id,
                Err(e) => return Err(anyhow::anyhow!("Auth error: {e}")),
            };
            let tag_color = color.as_deref().unwrap_or("#6366f1").to_string();
            let id = uuid::Uuid::new_v4();
            let tag = match sqlx::query!(
                r#"INSERT INTO tags (id, user_id, name, color) VALUES (?, ?, ?, ?)
                   RETURNING id as "id!: String", name, color,
                             created_at as "created_at: EpochUtc",
                             updated_at as "updated_at: EpochUtc""#,
                id, user_id, name, tag_color,
            ).fetch_one(&state.db).await
            {
                Ok(t) => t,
                Err(e) => return Err(anyhow::anyhow!("Create failed: {e}")),
            };
            Ok(serde_json::json!({ "data": {
                "id": tag.id, "name": tag.name, "color": tag.color,
                "created_at": tag.created_at.0.to_rfc3339(), "updated_at": tag.updated_at.0.to_rfc3339(),
            }}))
        }
        TagsAction::Delete { id } => {
            let user_id = match crate::mcp::tools_posts::resolve_first_user(state).await {
                Ok(id) => id,
                Err(e) => return Err(anyhow::anyhow!("Auth error: {e}")),
            };
            let tag_id = match uuid::Uuid::parse_str(&id) {
                Ok(id) => id,
                Err(_) => return Err(anyhow::anyhow!("Invalid tag ID")),
            };
            let r = match sqlx::query!("DELETE FROM tags WHERE id = ? AND user_id = ?", tag_id, user_id)
                .execute(&state.db).await
            {
                Ok(r) => r,
                Err(e) => return Err(anyhow::anyhow!("Delete failed: {e}")),
            };
            if r.rows_affected() == 0 {
                return Err(anyhow::anyhow!("Tag not found"));
            }
            Ok(serde_json::json!({ "deleted": true }))
        }
        TagsAction::Get { id } => {
            let user_id = match crate::mcp::tools_posts::resolve_first_user(state).await {
                Ok(id) => id,
                Err(e) => return Err(anyhow::anyhow!("Auth error: {e}")),
            };
            let tag_id = match uuid::Uuid::parse_str(&id) {
                Ok(id) => id,
                Err(_) => return Err(anyhow::anyhow!("Invalid tag ID")),
            };
            let tag = match sqlx::query!(r#"SELECT id as "id!: String", name, color,
                                                 created_at as "created_at: EpochUtc",
                                                 updated_at as "updated_at: EpochUtc"
                                          FROM tags WHERE id = ? AND user_id = ?"#, tag_id, user_id)
                .fetch_optional(&state.db).await
            {
                Ok(Some(t)) => t,
                Ok(None) => return Err(anyhow::anyhow!("Tag not found")),
                Err(e) => return Err(anyhow::anyhow!("DB error: {e}")),
            };
            Ok(serde_json::json!({
                "id": tag.id, "name": tag.name, "color": tag.color,
                "created_at": tag.created_at.0.to_rfc3339(), "updated_at": tag.updated_at.0.to_rfc3339(),
            }))
        }
        TagsAction::Update { id, name, color } => {
            let user_id = match crate::mcp::tools_posts::resolve_first_user(state).await {
                Ok(id) => id,
                Err(e) => return Err(anyhow::anyhow!("Auth error: {e}")),
            };
            let tag_id = match uuid::Uuid::parse_str(&id) {
                Ok(id) => id,
                Err(_) => return Err(anyhow::anyhow!("Invalid tag ID")),
            };
            let existing = match sqlx::query!(r#"SELECT id as "id!: String", name, color FROM tags WHERE id = ? AND user_id = ?"#, tag_id, user_id)
                .fetch_optional(&state.db).await
            {
                Ok(Some(t)) => t,
                Ok(None) => return Err(anyhow::anyhow!("Tag not found")),
                Err(e) => return Err(anyhow::anyhow!("DB error: {e}")),
            };
            let new_name = name.unwrap_or(existing.name);
            let new_color = color.unwrap_or(existing.color);
            match sqlx::query!("UPDATE tags SET name = ?, color = ?, updated_at = unixepoch() WHERE id = ? AND user_id = ?", new_name, new_color, tag_id, user_id)
                .execute(&state.db).await
            {
                Ok(_) => Ok(serde_json::json!({ "updated": true })),
                Err(e) => Err(format!("Update failed: {e}")),
            }
        }
    };

    super::emit_result(result)
}
