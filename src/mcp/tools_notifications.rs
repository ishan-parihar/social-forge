use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::db::models::NotificationPublic;
use crate::db::queries;

// ── Input Types ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct NotifListInput {
    /// JWT auth token
    pub token: String,
    /// Max notifications to return (default 50, max 200)
    pub limit: Option<i32>,
    /// Number of notifications to skip
    pub offset: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct NotifMarkReadInput {
    /// JWT auth token
    pub token: String,
    /// Notification ID (UUID)
    pub notification_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct NotifMarkAllReadInput {
    /// JWT auth token
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct NotifCreateInput {
    /// JWT auth token
    pub token: String,
    /// Notification title
    pub title: String,
    /// Notification body
    pub body: Option<String>,
    /// Notification type (e.g. post_published, post_failed, post_created)
    pub notification_type: String,
    /// Optional reference type (e.g. post, integration)
    pub reference_type: Option<String>,
    /// Optional reference ID
    pub reference_id: Option<String>,
}

// ── Handlers ─────────────────────────────────────────────────

/// List notifications for the user
pub async fn handle_notif_list(
    state: &AppState,
    input: &NotifListInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let limit = input.limit.unwrap_or(50).min(200) as i64;
    let offset = input.offset.unwrap_or(0) as i64;

    let notifs = queries::list_notifications(&state.db, user_id, limit, offset)
        .await
        .map_err(|e| format!("Failed to list notifications: {e}"))?;

    let data: Vec<NotificationPublic> = notifs.into_iter().map(NotificationPublic::from).collect();
    Ok(Json(serde_json::json!({"data": data})))
}

/// Mark a single notification as read
pub async fn handle_notif_mark_read(
    state: &AppState,
    input: &NotifMarkReadInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let notif_id = Uuid::parse_str(&input.notification_id)
        .map_err(|_| format!("Invalid notification ID: {}", input.notification_id))?;

    let notif = queries::mark_notification_read(&state.db, notif_id, user_id)
        .await
        .map_err(|e| format!("Failed to mark notification as read: {e}"))?
        .ok_or_else(|| "Notification not found".to_string())?;

    let data: NotificationPublic = NotificationPublic::from(notif);
    Ok(Json(serde_json::json!({"data": data})))
}

/// Mark all notifications as read
pub async fn handle_notif_mark_all_read(
    state: &AppState,
    _input: &NotifMarkAllReadInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;

    let count = queries::mark_all_notifications_read(&state.db, user_id)
        .await
        .map_err(|e| format!("Failed to mark all notifications as read: {e}"))?;

    Ok(Json(serde_json::json!({"updated": count})))
}

/// Create a notification (for testing/programmatic use)
pub async fn handle_notif_create(
    state: &AppState,
    input: &NotifCreateInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;

    let notif = queries::create_notification(
        &state.db,
        user_id,
        &input.title,
        input.body.as_deref().unwrap_or(""),
        &input.notification_type,
        input.reference_type.as_deref(),
        input.reference_id.as_deref(),
    )
    .await
    .map_err(|e| format!("Failed to create notification: {e}"))?;

    let data: NotificationPublic = NotificationPublic::from(notif);
    Ok(Json(serde_json::json!({"data": data})))
}
