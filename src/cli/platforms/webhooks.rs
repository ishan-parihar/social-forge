use crate::api::AppState;
use crate::cli::WebhooksAction;

use crate::db::types::EpochUtc;

pub async fn handle(action: WebhooksAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        WebhooksAction::List => {
            let user_id = match crate::mcp::tools_posts::resolve_first_user(state).await {
                Ok(id) => id,
                Err(e) => return Err(anyhow::anyhow!("Auth error: {e}")),
            };
            let rows: Vec<serde_json::Value> = match sqlx::query!(
                r#"SELECT id as "id!: String", name, url, is_active as "is_active!: i64",
                          created_at as "created_at: EpochUtc"
                   FROM webhooks WHERE user_id = ? ORDER BY created_at DESC"#,
                user_id,
            )
            .fetch_all(&state.db)
            .await
            {
                Ok(rows) => rows.into_iter()
                    .map(|r| serde_json::json!({
                        "id": r.id, "name": r.name, "url": r.url,
                        "is_active": r.is_active != 0, "created_at": r.created_at.0.to_rfc3339(),
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
            let id = uuid::Uuid::new_v4();
            let row = match sqlx::query!(
                r#"INSERT INTO webhooks (id, user_id, name, url) VALUES (?, ?, ?, ?)
                 RETURNING id as "id!: String", name, url, is_active as "is_active!: i64",
                           created_at as "created_at: EpochUtc""#,
                id, user_id, name, url,
            )
            .fetch_one(&state.db)
            .await
            {
                Ok(row) => row,
                Err(e) => return Err(anyhow::anyhow!("Failed to create webhook: {e}")),
            };
            // SQLite has no `TEXT[]`; the subscribe list lives in the
            // webhook_event_types join table. '*' is the catch-all.
            let webhook_id = row.id.clone();
            if let Err(e) = sqlx::query(
                "INSERT INTO webhook_event_types (webhook_id, event_type) VALUES (?, '*')
                 ON CONFLICT DO NOTHING",
            )
            .bind(webhook_id)
            .execute(&state.db)
            .await
            {
                return Err(anyhow::anyhow!("Failed to subscribe webhook to events: {e}"));
            }
            Ok(serde_json::json!({
                "id": row.id, "name": row.name, "url": row.url,
                "is_active": row.is_active != 0, "created_at": row.created_at.0.to_rfc3339(),
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
            let del_result = match sqlx::query!("DELETE FROM webhooks WHERE id = ? AND user_id = ?", webhook_id, user_id)
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
            let row = match sqlx::query!(r#"SELECT id as "id!: String", name, url, is_active as "is_active!: i64",
                                                  created_at as "created_at: EpochUtc"
                                           FROM webhooks WHERE id = ? AND user_id = ?"#, webhook_id, user_id)
                .fetch_optional(&state.db).await
            {
                Ok(Some(r)) => r,
                Ok(None) => return Err(anyhow::anyhow!("Webhook not found")),
                Err(e) => return Err(anyhow::anyhow!("DB error: {e}")),
            };
            Ok(serde_json::json!({
                "id": row.id, "name": row.name, "url": row.url,
                "is_active": row.is_active != 0, "created_at": row.created_at.0.to_rfc3339(),
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
            let existing = match sqlx::query!(r#"SELECT id as "id!: String", name, url, is_active as "is_active!: i64"
                                                 FROM webhooks WHERE id = ? AND user_id = ?"#, webhook_id, user_id)
                .fetch_optional(&state.db).await
            {
                Ok(Some(r)) => r,
                Ok(None) => return Err(anyhow::anyhow!("Webhook not found")),
                Err(e) => return Err(anyhow::anyhow!("DB error: {e}")),
            };
            let new_name = name.unwrap_or(existing.name);
            let new_url = url.unwrap_or(existing.url);
            let new_active = active.map(i64::from).unwrap_or(existing.is_active);
            match sqlx::query!("UPDATE webhooks SET name = ?, url = ?, is_active = ? WHERE id = ? AND user_id = ?", new_name, new_url, new_active, webhook_id, user_id)
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
            let row = match sqlx::query!(r#"SELECT id as "id!: String", url FROM webhooks WHERE id = ? AND user_id = ?"#, webhook_id, user_id)
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
