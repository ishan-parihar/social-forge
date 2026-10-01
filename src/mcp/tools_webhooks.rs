// ─── MCP Webhook Tools ──────────────────────────────────────────
// Tool handlers for webhook CRUD and test delivery.

use chrono::{DateTime, Utc};
use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::api::AppState;

// ── Input Types ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WhCreateInput {
    pub name: String,
    pub url: String,
    pub secret: Option<String>,
    pub event_types: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WhListInput {
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WhGetInput {
    pub webhook_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WhUpdateInput {
    pub webhook_id: String,
    pub name: Option<String>,
    pub url: Option<String>,
    pub secret: Option<String>,
    pub event_types: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WhDeleteInput {
    pub webhook_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WhTestInput {
    pub webhook_id: String,
}

// ── Internal Row Types ──────────────────────────────────────

/// A `webhooks` row. `event_types` is not a column on the table any more —
/// it lives in `webhook_event_types(webhook_id, event_type)`, so it is read
/// separately and stitched on by [`load_event_types`].
#[derive(Debug, FromRow)]
struct WebhookRow {
    id: Uuid,
    user_id: Uuid,
    name: String,
    url: String,
    secret: Option<String>,
    is_active: bool,
    last_triggered_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl WebhookRow {
    /// Shared by every SELECT/RETURNING so the call sites can't drift apart.
    const SELECT_COLS: &'static str = "id, user_id, name, url, secret, is_active, \
         last_triggered_at, created_at, updated_at";
}

/// A webhook row plus its event types.
#[derive(Debug)]
struct WebhookWithEvents {
    base: WebhookRow,
    event_types: Vec<String>,
}

#[derive(Debug, FromRow)]
struct WebhookDeliveryRow {
    id: Uuid,
    webhook_id: Uuid,
    event_type: String,
    status: String,
    status_code: Option<i32>,
    response_body: Option<String>,
    attempted_at: DateTime<Utc>,
    delivered_at: Option<DateTime<Utc>>,
}

// ── Helpers ──────────────────────────────────────────────────

/// Single-user mode — every webhook operation is owned by
/// `DEFAULT_USER_ID`. The `token` field on input structs is now
/// vestigial (kept for schema compatibility) and ignored.
fn resolve_user(_token: &str, _state: &AppState) -> Result<Uuid, String> {
    Ok(crate::auth::middleware::DEFAULT_USER_ID)
}

fn webhook_to_json(w: WebhookWithEvents) -> serde_json::Value {
    let WebhookWithEvents { base: w, event_types } = w;
    serde_json::json!({
        "id": w.id.to_string(),
        "user_id": w.user_id.to_string(),
        "name": w.name,
        "url": w.url,
        "secret": w.secret,
        "event_types": event_types,
        "is_active": w.is_active,
        "last_triggered_at": w.last_triggered_at.map(|dt| dt.to_rfc3339()),
        "created_at": w.created_at.to_rfc3339(),
        "updated_at": w.updated_at.to_rfc3339(),
    })
}

/// Read one webhook's event types from `webhook_event_types`.
async fn load_event_types(
    db: &crate::db::SqlitePool,
    webhook_id: Uuid,
) -> Result<Vec<String>, String> {
    let types: Vec<String> = sqlx::query_scalar(
        "SELECT event_type FROM webhook_event_types
         WHERE webhook_id = ? ORDER BY event_type",
    )
    .bind(webhook_id)
    .fetch_all(db)
    .await
    .map_err(|e| format!("Database error: {e}"))?;
    Ok(types)
}

/// Replace a webhook's event types wholesale, inside the caller's transaction.
async fn replace_event_types(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    webhook_id: Uuid,
    event_types: &[String],
) -> Result<(), String> {
    sqlx::query("DELETE FROM webhook_event_types WHERE webhook_id = ?")
        .bind(webhook_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("Database error: {e}"))?;
    for event_type in event_types {
        sqlx::query("INSERT INTO webhook_event_types (webhook_id, event_type) VALUES (?, ?)")
            .bind(webhook_id)
            .bind(event_type)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Database error: {e}"))?;
    }
    Ok(())
}

/// Fetch a webhook by id scoped to `user_id`; `None` when absent.
async fn fetch_webhook(
    db: &crate::db::SqlitePool,
    id: Uuid,
    user_id: Uuid,
) -> Result<Option<WebhookWithEvents>, String> {
    let sql = format!(
        "SELECT {} FROM webhooks WHERE id = ? AND user_id = ?",
        WebhookRow::SELECT_COLS
    );
    let base: Option<WebhookRow> = sqlx::query_as(&sql)
        .bind(id)
        .bind(user_id)
        .fetch_optional(db)
        .await
        .map_err(|e| format!("Database error: {e}"))?;
    match base {
        Some(base) => {
            let event_types = load_event_types(db, id).await?;
            Ok(Some(WebhookWithEvents { base, event_types }))
        }
        None => Ok(None),
    }
}

// ── Tool Implementations ─────────────────────────────────────

pub async fn handle_wh_create(
    state: &AppState,
    input: &WhCreateInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_user("", state)?;

    if input.name.trim().is_empty() {
        return Err("Webhook name is required".into());
    }
    if input.url.trim().is_empty() {
        return Err("Webhook URL is required".into());
    }

    // The webhook row and its `webhook_event_types` lines go in one
    // transaction — a partial write would leave a webhook that never fires.
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| format!("Failed to begin tx: {e}"))?;

    let sql = format!(
        "INSERT INTO webhooks (id, user_id, name, url, secret)
         VALUES (?, ?, ?, ?, ?)
         RETURNING {}",
        WebhookRow::SELECT_COLS
    );
    // `webhooks.id` is TEXT with no DB-side default — SQLite has no
    // gen_random_uuid(), so Rust generates the value.
    let base: WebhookRow = sqlx::query_as(&sql)
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(input.name.trim())
        .bind(input.url.trim())
        .bind(&input.secret)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| format!("Failed to create webhook: {e}"))?;

    replace_event_types(&mut tx, base.id, &input.event_types).await?;

    tx.commit()
        .await
        .map_err(|e| format!("Failed to commit webhook: {e}"))?;

    let event_types = load_event_types(&state.db, base.id).await?;
    Ok(Json(webhook_to_json(WebhookWithEvents { base, event_types })))
}

pub async fn handle_wh_list(
    state: &AppState,
    _input: &WhListInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_user("", state)?;

    let sql = format!(
        "SELECT {} FROM webhooks WHERE user_id = ? ORDER BY created_at DESC",
        WebhookRow::SELECT_COLS
    );
    let rows: Vec<WebhookRow> = sqlx::query_as(&sql)
        .bind(user_id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| format!("Failed to list webhooks: {e}"))?;

    let mut webhooks: Vec<serde_json::Value> = Vec::with_capacity(rows.len());
    for base in rows {
        let event_types = load_event_types(&state.db, base.id).await?;
        webhooks.push(webhook_to_json(WebhookWithEvents { base, event_types }));
    }
    Ok(Json(serde_json::json!({ "webhooks": webhooks })))
}

pub async fn handle_wh_get(
    state: &AppState,
    input: &WhGetInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_user("", state)?;
    let webhook_id = Uuid::parse_str(&input.webhook_id)
        .map_err(|_| "Invalid webhook ID format".to_string())?;

    let row = fetch_webhook(&state.db, webhook_id, user_id)
        .await?
        .ok_or_else(|| "Webhook not found".to_string())?;

    Ok(Json(webhook_to_json(row)))
}

pub async fn handle_wh_update(
    state: &AppState,
    input: &WhUpdateInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_user("", state)?;
    let webhook_id = Uuid::parse_str(&input.webhook_id)
        .map_err(|_| "Invalid webhook ID format".to_string())?;

    let current = fetch_webhook(&state.db, webhook_id, user_id)
        .await?
        .ok_or_else(|| "Webhook not found".to_string())?;
    let WebhookWithEvents { base: current, event_types: current_event_types } = current;

    let new_name = input.name.clone().unwrap_or(current.name);
    let new_url = input.url.clone().unwrap_or(current.url);
    let new_secret = input.secret.clone().or(current.secret);
    let new_event_types = input.event_types.clone().unwrap_or(current_event_types);
    let new_is_active = input.is_active.unwrap_or(current.is_active);

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| format!("Failed to begin tx: {e}"))?;

    // `event_types` is no longer a column on `webhooks`, so the base UPDATE
    // drops it and the list is rewritten separately in the same tx.
    let sql = format!(
        "UPDATE webhooks
         SET name = ?, url = ?, secret = ?, is_active = ?,
             updated_at = unixepoch()
         WHERE id = ? AND user_id = ?
         RETURNING {}",
        WebhookRow::SELECT_COLS
    );
    let base: WebhookRow = sqlx::query_as(&sql)
        .bind(new_name.trim())
        .bind(new_url.trim())
        .bind(new_secret)
        .bind(new_is_active)
        .bind(webhook_id)
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| format!("Failed to update webhook: {e}"))?;

    replace_event_types(&mut tx, webhook_id, &new_event_types).await?;

    tx.commit()
        .await
        .map_err(|e| format!("Failed to commit webhook: {e}"))?;

    Ok(Json(webhook_to_json(WebhookWithEvents {
        base,
        event_types: new_event_types,
    })))
}

pub async fn handle_wh_delete(
    state: &AppState,
    input: &WhDeleteInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_user("", state)?;
    let webhook_id = Uuid::parse_str(&input.webhook_id)
        .map_err(|_| "Invalid webhook ID format".to_string())?;

    let result = sqlx::query("DELETE FROM webhooks WHERE id = ? AND user_id = ?")
        .bind(webhook_id)
        .bind(user_id)
        .execute(&state.db)
        .await
        .map_err(|e| format!("Failed to delete webhook: {e}"))?;

    if result.rows_affected() == 0 {
        return Err("Webhook not found".into());
    }

    Ok(Json(serde_json::json!({ "deleted": true })))
}

pub async fn handle_wh_test(
    state: &AppState,
    input: &WhTestInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_user("", state)?;
    let webhook_id = Uuid::parse_str(&input.webhook_id)
        .map_err(|_| "Invalid webhook ID format".to_string())?;

    let row = fetch_webhook(&state.db, webhook_id, user_id)
        .await?
        .ok_or_else(|| "Webhook not found".to_string())?;

    let payload = serde_json::json!({
        "event_type": "test",
        "timestamp": Utc::now().to_rfc3339(),
        "data": {
            "message": "This is a test webhook event from Social Forge"
        }
    });

    let result = crate::services::webhook_dispatcher::send_webhook(
        &row.base.url,
        row.base.secret.as_deref(),
        "test",
        &payload,
    )
    .await?;

    let (status_code, response_body) = result;

    let delivery_row: WebhookDeliveryRow = sqlx::query_as(
        r#"
        INSERT INTO webhook_deliveries (id, webhook_id, event_type, payload, status, status_code, response_body, delivered_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, unixepoch())
        RETURNING id, webhook_id, event_type, status, status_code, response_body, attempted_at, delivered_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(webhook_id)
    .bind("test")
    .bind(&payload)
    .bind(if status_code == 200 || status_code == 201 { "delivered" } else { "failed" })
    .bind(status_code as i32)
    .bind(&response_body)
    .fetch_one(&state.db)
    .await
    .map_err(|e| format!("Failed to record delivery: {e}"))?;

    let _ = sqlx::query("UPDATE webhooks SET last_triggered_at = unixepoch() WHERE id = ?")
        .bind(webhook_id)
        .execute(&state.db)
        .await;

    Ok(Json(serde_json::json!({
        "status_code": status_code,
        "response_body": response_body,
        "delivery": {
            "id": delivery_row.id.to_string(),
            "webhook_id": delivery_row.webhook_id.to_string(),
            "event_type": delivery_row.event_type,
            "status": delivery_row.status,
            "status_code": delivery_row.status_code,
            "response_body": delivery_row.response_body,
            "attempted_at": delivery_row.attempted_at.to_rfc3339(),
            "delivered_at": delivery_row.delivered_at.map(|dt| dt.to_rfc3339()),
        }
    })))
}
