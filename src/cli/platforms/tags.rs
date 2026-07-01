use crate::api::AppState;
use crate::cli::TagsAction;

pub async fn handle(action: TagsAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        TagsAction::List => {
            let user_id = match crate::mcp::tools_posts::resolve_first_user(state).await {
                Ok(id) => id,
                Err(e) => return Err(anyhow::anyhow!("Auth error: {e}")),
            };
            let tags = match sqlx::query!("SELECT id, name, color, created_at, updated_at FROM tags WHERE user_id = $1 ORDER BY name", user_id)
                .fetch_all(&state.db).await
            {
                Ok(t) => t,
                Err(e) => return Err(anyhow::anyhow!("DB error: {e}")),
            };
            let data: Vec<_> = tags.into_iter().map(|t| serde_json::json!({
                "id": t.id.to_string(), "name": t.name, "color": t.color,
                "created_at": t.created_at.to_rfc3339(), "updated_at": t.updated_at.to_rfc3339(),
            })).collect();
            Ok(serde_json::json!({ "data": data }))
        }
        TagsAction::Create { name, color } => {
            let user_id = match crate::mcp::tools_posts::resolve_first_user(state).await {
                Ok(id) => id,
                Err(e) => return Err(anyhow::anyhow!("Auth error: {e}")),
            };
            let tag = match sqlx::query!(
                "INSERT INTO tags (user_id, name, color) VALUES ($1, $2, $3) RETURNING id, name, color, created_at, updated_at",
                user_id, name, color.as_deref().unwrap_or("#6366f1"),
            ).fetch_one(&state.db).await
            {
                Ok(t) => t,
                Err(e) => return Err(anyhow::anyhow!("Create failed: {e}")),
            };
            Ok(serde_json::json!({ "data": {
                "id": tag.id.to_string(), "name": tag.name, "color": tag.color,
                "created_at": tag.created_at.to_rfc3339(), "updated_at": tag.updated_at.to_rfc3339(),
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
            let r = match sqlx::query!("DELETE FROM tags WHERE id = $1 AND user_id = $2", tag_id, user_id)
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
            let tag = match sqlx::query!("SELECT id, name, color, created_at, updated_at FROM tags WHERE id = $1 AND user_id = $2", tag_id, user_id)
                .fetch_optional(&state.db).await
            {
                Ok(Some(t)) => t,
                Ok(None) => return Err(anyhow::anyhow!("Tag not found")),
                Err(e) => return Err(anyhow::anyhow!("DB error: {e}")),
            };
            Ok(serde_json::json!({
                "id": tag.id.to_string(), "name": tag.name, "color": tag.color,
                "created_at": tag.created_at.to_rfc3339(), "updated_at": tag.updated_at.to_rfc3339(),
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
            let existing = match sqlx::query!("SELECT id, name, color FROM tags WHERE id = $1 AND user_id = $2", tag_id, user_id)
                .fetch_optional(&state.db).await
            {
                Ok(Some(t)) => t,
                Ok(None) => return Err(anyhow::anyhow!("Tag not found")),
                Err(e) => return Err(anyhow::anyhow!("DB error: {e}")),
            };
            let new_name = name.unwrap_or(existing.name);
            let new_color = color.unwrap_or(existing.color);
            match sqlx::query!("UPDATE tags SET name = $3, color = $4, updated_at = NOW() WHERE id = $1 AND user_id = $2", tag_id, user_id, new_name, new_color)
                .execute(&state.db).await
            {
                Ok(_) => Ok(serde_json::json!({ "updated": true })),
                Err(e) => Err(format!("Update failed: {e}")),
            }
        }
    };
    match result {
        Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
        Err(e) => { eprintln!("{}", serde_json::json!({"error": e})); std::process::exit(1); }
    }
    Ok(())
}
