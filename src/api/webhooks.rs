// ─── Webhooks API Routes ───────────────────────────────────────
// CRUD for outgoing webhooks with test delivery support.

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;

use super::AppState;

// ── Database Row Types ──────────────────────────────────────

#[derive(Debug, FromRow, Serialize)]
pub struct WebhookRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub url: String,
    pub secret: Option<String>,
    pub event_types: Vec<String>,
    pub is_active: bool,
    pub last_triggered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct WebhookDeliveryRow {
    pub id: Uuid,
    pub webhook_id: Uuid,
    pub event_type: String,
    pub status: String,
    pub status_code: Option<i32>,
    pub response_body: Option<String>,
    pub attempted_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
}

// ── Request Types ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct WebhookCreateRequest {
    pub name: String,
    pub url: String,
    pub secret: Option<String>,
    pub event_types: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookUpdateRequest {
    pub name: Option<String>,
    pub url: Option<String>,
    pub secret: Option<String>,
    pub event_types: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

// ── Response Types ────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub url: String,
    pub secret: Option<String>,
    pub event_types: Vec<String>,
    pub is_active: bool,
    pub last_triggered_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct WebhookDeliveryResponse {
    pub id: Uuid,
    pub webhook_id: Uuid,
    pub event_type: String,
    pub status: String,
    pub status_code: Option<i32>,
    pub response_body: Option<String>,
    pub attempted_at: String,
    pub delivered_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WebhookTestResult {
    pub status_code: u16,
    pub response_body: String,
    pub delivery: WebhookDeliveryResponse,
}

// ── Helpers ─────────────────────────────────────────────────

fn webhook_to_response(w: WebhookRow) -> WebhookResponse {
    WebhookResponse {
        id: w.id,
        user_id: w.user_id,
        name: w.name,
        url: w.url,
        secret: w.secret,
        event_types: w.event_types,
        is_active: w.is_active,
        last_triggered_at: w.last_triggered_at.map(|dt| dt.to_rfc3339()),
        created_at: w.created_at.to_rfc3339(),
        updated_at: w.updated_at.to_rfc3339(),
    }
}

fn delivery_to_response(d: WebhookDeliveryRow) -> WebhookDeliveryResponse {
    WebhookDeliveryResponse {
        id: d.id,
        webhook_id: d.webhook_id,
        event_type: d.event_type,
        status: d.status,
        status_code: d.status_code,
        response_body: d.response_body,
        attempted_at: d.attempted_at.to_rfc3339(),
        delivered_at: d.delivered_at.map(|dt| dt.to_rfc3339()),
    }
}

// ── Handlers ─────────────────────────────────────────────────

/// POST /api/webhooks
pub async fn create(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<WebhookCreateRequest>,
) -> Result<Json<WebhookResponse>, crate::error::AppError> {
    if body.name.trim().is_empty() {
        return Err(crate::error::AppError::BadRequest(
            "Webhook name is required".into(),
        ));
    }
    if body.url.trim().is_empty() {
        return Err(crate::error::AppError::BadRequest(
            "Webhook URL is required".into(),
        ));
    }

    let row: WebhookRow = sqlx::query_as(
        r#"
        INSERT INTO webhooks (user_id, name, url, secret, event_types)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, user_id, name, url, secret, event_types, is_active,
                  last_triggered_at, created_at, updated_at
        "#,
    )
    .bind(auth.user_id)
    .bind(body.name.trim())
    .bind(body.url.trim())
    .bind(body.secret)
    .bind(&body.event_types)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(webhook_to_response(row)))
}

/// GET /api/webhooks
pub async fn list(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<WebhookResponse>>, crate::error::AppError> {
    let rows: Vec<WebhookRow> = sqlx::query_as(
        r#"
        SELECT id, user_id, name, url, secret, event_types, is_active,
               last_triggered_at, created_at, updated_at
        FROM webhooks
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows.into_iter().map(webhook_to_response).collect()))
}

/// GET /api/webhooks/:id
pub async fn get(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<WebhookResponse>, crate::error::AppError> {
    let row: WebhookRow = sqlx::query_as(
        r#"
        SELECT id, user_id, name, url, secret, event_types, is_active,
               last_triggered_at, created_at, updated_at
        FROM webhooks
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| crate::error::AppError::NotFound("Webhook not found".into()))?;

    Ok(Json(webhook_to_response(row)))
}

/// PUT /api/webhooks/:id
pub async fn update(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(body): Json<WebhookUpdateRequest>,
) -> Result<Json<WebhookResponse>, crate::error::AppError> {
    // Fetch current row first to merge with updates
    let current: WebhookRow = sqlx::query_as(
        r#"
        SELECT id, user_id, name, url, secret, event_types, is_active,
               last_triggered_at, created_at, updated_at
        FROM webhooks
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| crate::error::AppError::NotFound("Webhook not found".into()))?;

    let new_name = body.name.unwrap_or(current.name);
    let new_url = body.url.unwrap_or(current.url);
    let new_secret = body.secret.or(current.secret);
    let new_event_types = body.event_types.unwrap_or(current.event_types);
    let new_is_active = body.is_active.unwrap_or(current.is_active);

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
    .bind(id)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(webhook_to_response(row)))
}

/// DELETE /api/webhooks/:id
pub async fn delete(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let result = sqlx::query("DELETE FROM webhooks WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(crate::error::AppError::NotFound("Webhook not found".into()));
    }

    Ok(Json(serde_json::json!({"deleted": true})))
}

/// POST /api/webhooks/:id/test
/// Sends a test event to the webhook URL and records the delivery result.
pub async fn test(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<WebhookTestResult>, crate::error::AppError> {
    // Fetch the webhook
    let row: WebhookRow = sqlx::query_as(
        r#"
        SELECT id, user_id, name, url, secret, event_types, is_active,
               last_triggered_at, created_at, updated_at
        FROM webhooks
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| crate::error::AppError::NotFound("Webhook not found".into()))?;

    // Build test payload
    let payload = serde_json::json!({
        "event_type": "test",
        "timestamp": Utc::now().to_rfc3339(),
        "data": {
            "message": "This is a test webhook event from Social Forge"
        }
    });

    // Send the test via the dispatcher
    let result = crate::services::webhook_dispatcher::send_webhook(
        &row.url,
        row.secret.as_deref(),
        "test",
        &payload,
    )
    .await
    .map_err(|e| crate::error::AppError::BadRequest(format!("HTTP request failed: {e}")))?;

    let (status_code, response_body) = result;

    // Record delivery
    let delivery_row: WebhookDeliveryRow = sqlx::query_as(
        r#"
        INSERT INTO webhook_deliveries (webhook_id, event_type, payload, status, status_code, response_body, delivered_at)
        VALUES ($1, $2, $3, $4, $5, $6, now())
        RETURNING id, webhook_id, event_type, status, status_code, response_body, attempted_at, delivered_at
        "#,
    )
    .bind(id)
    .bind("test")
    .bind(&payload)
    .bind(if status_code == 200 || status_code == 201 { "delivered" } else { "failed" })
    .bind(status_code as i32)
    .bind(&response_body)
    .fetch_one(&state.db)
    .await?;

    // Update last_triggered_at
    let _ = sqlx::query("UPDATE webhooks SET last_triggered_at = now() WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await;

    Ok(Json(WebhookTestResult {
        status_code,
        response_body,
        delivery: delivery_to_response(delivery_row),
    }))
}
