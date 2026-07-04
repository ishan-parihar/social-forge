use crate::api::AppState;
use crate::cli::WebhooksAction;

pub async fn handle(action: WebhooksAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        WebhooksAction::List => {
            let user_id = match crate::mcp::tools_posts::resolve_first_user(state).await {
                Ok(id) => id,
                Err(e) => return Err(anyhow::anyhow!("Auth error: {e}")),
            };
            let rows: Vec<serde_json::Value> = match sqlx::query!(
                "SELECT id, name, url, is_active, created_at FROM webhooks WHERE user_id = $1 ORDER BY created_at DESC",
                user_id,
            )
            .fetch_all(&state.db)
            .await
            {
                Ok(rows) => rows.into_iter()
                    .map(|r| serde_json::json!({
                        "id": r.id.to_string(), "name": r.name, "url": r.url,
                        "is_active": r.is_active, "created_at": r.created_at.to_rfc3339(),
                    }))
                    .collect(),
                Err(e) => return Err(anyhow::anyhow!("DB error: {e}")),
            };
            Ok(serde_json::json!({ "webhooks": rows }))
        }
        WebhooksAction::Create { url, name } => {
            let user_id = match crate::mcp::tools_posts::resolve_first_user(state).await {
                Ok(id) => id,
                Err(e) => return Err(anyhow::anyhow!("Auth error: {e}")),
            };
            let row = match sqlx::query!(
                "INSERT INTO webhooks (user_id, name, url, event_types) VALUES ($1, $2, $3, ARRAY['*']::text[])
                 RETURNING id, name, url, is_active, created_at",
                user_id, name, url,
            )
            .fetch_one(&state.db)
            .await
            {
                Ok(row) => row,
                Err(e) => return Err(anyhow::anyhow!("Failed to create webhook: {e}")),
            };
            Ok(serde_json::json!({
                "id": row.id.to_string(), "name": row.name, "url": row.url,
                "is_active": row.is_active, "created_at": row.created_at.to_rfc3339(),
            }))
        }
        WebhooksAction::Delete { id } => {
            let user_id = match crate::mcp::tools_posts::resolve_first_user(state).await {
                Ok(id) => id,
                Err(e) => return Err(anyhow::anyhow!("Auth error: {e}")),
            };
            let webhook_id = match uuid::Uuid::parse_str(&id) {
                Ok(id) => id,
                Err(_) => return Err(anyhow::anyhow!("Invalid webhook ID")),
            };
            let del_result = match sqlx::query!("DELETE FROM webhooks WHERE id = $1 AND user_id = $2", webhook_id, user_id)
                .execute(&state.db).await
            {
                Ok(r) => r,
                Err(e) => return Err(anyhow::anyhow!("Delete failed: {e}")),
            };
            if del_result.rows_affected() == 0 {
                return Err(anyhow::anyhow!("Webhook not found"));
            }
            Ok(serde_json::json!({ "deleted": true }))
        }
        WebhooksAction::Get { id } => {
            let user_id = match crate::mcp::tools_posts::resolve_first_user(state).await {
                Ok(id) => id,
                Err(e) => return Err(anyhow::anyhow!("Auth error: {e}")),
            };
            let webhook_id = match uuid::Uuid::parse_str(&id) {
                Ok(id) => id,
                Err(_) => return Err(anyhow::anyhow!("Invalid webhook ID")),
            };
            let row = match sqlx::query!("SELECT id, name, url, is_active, created_at FROM webhooks WHERE id = $1 AND user_id = $2", webhook_id, user_id)
                .fetch_optional(&state.db).await
            {
                Ok(Some(r)) => r,
                Ok(None) => return Err(anyhow::anyhow!("Webhook not found")),
                Err(e) => return Err(anyhow::anyhow!("DB error: {e}")),
            };
            Ok(serde_json::json!({
                "id": row.id.to_string(), "name": row.name, "url": row.url,
                "is_active": row.is_active, "created_at": row.created_at.to_rfc3339(),
            }))
        }
        WebhooksAction::Update { id, name, url, active } => {
            let user_id = match crate::mcp::tools_posts::resolve_first_user(state).await {
                Ok(id) => id,
                Err(e) => return Err(anyhow::anyhow!("Auth error: {e}")),
            };
            let webhook_id = match uuid::Uuid::parse_str(&id) {
                Ok(id) => id,
                Err(_) => return Err(anyhow::anyhow!("Invalid webhook ID")),
            };
            let existing = match sqlx::query!("SELECT id, name, url, is_active FROM webhooks WHERE id = $1 AND user_id = $2", webhook_id, user_id)
                .fetch_optional(&state.db).await
            {
                Ok(Some(r)) => r,
                Ok(None) => return Err(anyhow::anyhow!("Webhook not found")),
                Err(e) => return Err(anyhow::anyhow!("DB error: {e}")),
            };
            let new_name = name.unwrap_or(existing.name);
            let new_url = url.unwrap_or(existing.url);
            let new_active = active.unwrap_or(existing.is_active);
            match sqlx::query!("UPDATE webhooks SET name = $3, url = $4, is_active = $5 WHERE id = $1 AND user_id = $2", webhook_id, user_id, new_name, new_url, new_active)
                .execute(&state.db).await
            {
                Ok(_) => Ok(serde_json::json!({ "updated": true })),
                Err(e) => Err(format!("Update failed: {e}")),
            }
        }
        WebhooksAction::Test { id } => {
            let user_id = match crate::mcp::tools_posts::resolve_first_user(state).await {
                Ok(id) => id,
                Err(e) => return Err(anyhow::anyhow!("Auth error: {e}")),
            };
            let webhook_id = match uuid::Uuid::parse_str(&id) {
                Ok(id) => id,
                Err(_) => return Err(anyhow::anyhow!("Invalid webhook ID")),
            };
            let row = match sqlx::query!("SELECT id, url FROM webhooks WHERE id = $1 AND user_id = $2", webhook_id, user_id)
                .fetch_optional(&state.db).await
            {
                Ok(Some(r)) => r,
                Ok(None) => return Err(anyhow::anyhow!("Webhook not found")),
                Err(e) => return Err(anyhow::anyhow!("DB error: {e}")),
            };
            let test_payload = serde_json::json!({"event": "test", "timestamp": chrono::Utc::now().to_rfc3339()});
            match reqwest::Client::new().post(&row.url).json(&test_payload).send().await {
                Ok(resp) => Ok(serde_json::json!({ "status": resp.status().as_u16(), "ok": resp.status().is_success() })),
                Err(e) => Ok(serde_json::json!({ "status": 0, "ok": false, "error": e.to_string() })),
            }
        }
    };

    super::emit_result(result)
}
