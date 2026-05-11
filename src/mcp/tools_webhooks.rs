// ─── MCP Webhook Tools ──────────────────────────────────────────
// Tool handlers for webhook CRUD and test delivery.

use chrono::{DateTime, Utc};
use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::api::AppState;
use crate::auth::jwt;

// ── Input Types ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WhCreateInput {
    pub token: String,
    pub name: String,
    pub url: String,
    pub secret: Option<String>,
    pub event_types: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WhListInput {
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WhGetInput {
    pub token: String,
    pub webhook_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WhUpdateInput {
    pub token: String,
    pub webhook_id: String,
    pub name: Option<String>,
    pub url: Option<String>,
    pub secret: Option<String>,
    pub event_types: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WhDeleteInput {
    pub token: String,
    pub webhook_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WhTestInput {
    pub token: String,
    pub webhook_id: String,
}

// ── Internal Row Types ──────────────────────────────────────

#[derive(Debug, FromRow)]
struct WebhookRow {
    id: Uuid,
    user_id: Uuid,
    name: String,
    url: String,
    secret: Option<String>,
    event_types: Vec<String>,
    is_active: bool,
    last_triggered_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct WebhookDeliveryRow {
    id: Uuid,
    webhook_id: Uuid,
    event_type: String,
    payload: serde_json::Value,
    status: String,
    status_code: Option<i32>,
    response_body: Option<String>,
    attempted_at: DateTime<Utc>,
    delivered_at: Option<DateTime<Utc>>,
}

// ── Helpers ──────────────────────────────────────────────────

fn resolve_user(token: &str, state: &AppState) -> Result<Uuid, String> {
    let claims = jwt::validate_token(token, &state.config.jwt_secret)
        .map_err(|e| format!("Invalid token: {e}"))?;
    Uuid::parse_str(&claims.sub).map_err(|_| "Invalid user ID in token".to_string())
}

fn webhook_to_json(w: WebhookRow) -> serde_json::Value {
    serde_json::json!({
        "id": w.id.to_string(),
        "user_id": w.user_id.to_string(),
        "name": w.name,
        "url": w.url,
        "secret": w.secret,
        "event_types": w.event_types,
        "is_active": w.is_active,
        "last_triggered_at": w.last_triggered_at.map(|dt| dt.to_rfc3339()),
        "created_at": w.created_at.to_rfc3339(),
        "updated_at": w.updated_at.to_rfc3339(),
    })
}

// ── Tool Implementations ─────────────────────────────────────

pub async fn handle_wh_create(
    state: &AppState,
    input: &WhCreateInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_user(&input.token, state)?;

    if input.name.trim().is_empty() {
        return Err("Webhook name is required".into());
    }
    if input.url.trim().is_empty() {
        return Err("Webhook URL is required".into());
    }

    let row: WebhookRow = sqlx::query_as(
        r#"
        INSERT INTO webhooks (user_id, name, url, secret, event_types)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, user_id, name, url, secret, event_types, is_active,
                  last_triggered_at, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(input.name.trim())
    .bind(input.url.trim())
    .bind(&input.secret)
    .bind(&input.event_types)
    .fetch_one(&state.db)
    .await
    .map_err(|e| format!("Failed to create webhook: {e}"))?;

    Ok(Json(webhook_to_json(row)))
}

pub async fn handle_wh_list(
    state: &AppState,
    input: &WhListInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_user(&input.token, state)?;

    let rows: Vec<WebhookRow> = sqlx::query_as(
        r#"
        SELECT id, user_id, name, url, secret, event_types, is_active,
               last_triggered_at, created_at, updated_at
        FROM webhooks
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| format!("Failed to list webhooks: {e}"))?;

    let webhooks: Vec<serde_json::Value> = rows.into_iter().map(webhook_to_json).collect();
    Ok(Json(serde_json::json!({ "webhooks": webhooks })))
}

pub async fn handle_wh_get(
    state: &AppState,
    input: &WhGetInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_user(&input.token, state)?;
    let webhook_id = Uuid::parse_str(&input.webhook_id)
        .map_err(|_| "Invalid webhook ID format".to_string())?;

    let row: WebhookRow = sqlx::query_as(
        r#"
        SELECT id, user_id, name, url, secret, event_types, is_active,
               last_triggered_at, created_at, updated_at
        FROM webhooks
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(webhook_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| format!("Database error: {e}"))?
    .ok_or_else(|| "Webhook not found".to_string())?;

    Ok(Json(webhook_to_json(row)))
}

pub async fn handle_wh_update(
    state: &AppState,
    input: &WhUpdateInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_user(&input.token, state)?;
    let webhook_id = Uuid::parse_str(&input.webhook_id)
        .map_err(|_| "Invalid webhook ID format".to_string())?;

    let current: WebhookRow = sqlx::query_as(
        r#"
        SELECT id, user_id, name, url, secret, event_types, is_active,
               last_triggered_at, created_at, updated_at
        FROM webhooks
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(webhook_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| format!("Database error: {e}"))?
    .ok_or_else(|| "Webhook not found".to_string())?;

    let new_name = input.name.clone().unwrap_or(current.name);
    let new_url = input.url.clone().unwrap_or(current.url);
    let new_secret = input.secret.clone().or(current.secret);
    let new_event_types = input.event_types.clone().unwrap_or(current.event_types);
    let new_is_active = input.is_active.unwrap_or(current.is_active);

    let row: WebhookRow = sqlx::query_as(
        r#"
        UPDATE webhooks
        SET name = $1, url = $2, secret = $3, event_types = $4, is_active = $5,
            updated_at = now()
        WHERE id = $6 AND user_id = $7
        RETURNING id, user_id, name, url, secret, event_types, is_active,
                  last_triggered_at, created_at, updated_at
        "#,
    )
    .bind(new_name.trim())
    .bind(new_url.trim())
    .bind(new_secret)
    .bind(&new_event_types)
    .bind(new_is_active)
    .bind(webhook_id)
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| format!("Failed to update webhook: {e}"))?;

    Ok(Json(webhook_to_json(row)))
}

pub async fn handle_wh_delete(
    state: &AppState,
    input: &WhDeleteInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_user(&input.token, state)?;
    let webhook_id = Uuid::parse_str(&input.webhook_id)
        .map_err(|_| "Invalid webhook ID format".to_string())?;

    let result = sqlx::query("DELETE FROM webhooks WHERE id = $1 AND user_id = $2")
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
    let user_id = resolve_user(&input.token, state)?;
    let webhook_id = Uuid::parse_str(&input.webhook_id)
        .map_err(|_| "Invalid webhook ID format".to_string())?;

    let row: WebhookRow = sqlx::query_as(
        r#"
        SELECT id, user_id, name, url, secret, event_types, is_active,
               last_triggered_at, created_at, updated_at
        FROM webhooks
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(webhook_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| format!("Database error: {e}"))?
    .ok_or_else(|| "Webhook not found".to_string())?;

    let payload = serde_json::json!({
        "event_type": "test",
        "timestamp": Utc::now().to_rfc3339(),
        "data": {
            "message": "This is a test webhook event from Social Forge"
        }
    });

    let result = crate::services::webhook_dispatcher::send_webhook(
        &row.url,
        row.secret.as_deref(),
        "test",
        &payload,
    )
    .await?;

    let (status_code, response_body) = result;

    let delivery_row: WebhookDeliveryRow = sqlx::query_as(
        r#"
        INSERT INTO webhook_deliveries (webhook_id, event_type, payload, status, status_code, response_body, delivered_at)
        VALUES ($1, $2, $3, $4, $5, $6, now())
        RETURNING id, webhook_id, event_type, payload, status, status_code, response_body, attempted_at, delivered_at
        "#,
    )
    .bind(webhook_id)
    .bind("test")
    .bind(&payload)
    .bind(if status_code == 200 || status_code == 201 { "delivered" } else { "failed" })
    .bind(status_code as i32)
    .bind(&response_body)
    .fetch_one(&state.db)
    .await
    .map_err(|e| format!("Failed to record delivery: {e}"))?;

    let _ = sqlx::query("UPDATE webhooks SET last_triggered_at = now() WHERE id = $1")
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
