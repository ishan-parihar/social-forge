use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::models::NotificationPublic;
use crate::db::queries;
use crate::error::AppError;

use super::AppState;

#[derive(Debug, Deserialize)]
pub struct ListParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// GET /api/notifications
pub async fn list(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(params): Query<ListParams>,
) -> Result<Json<Value>, AppError> {
    let limit = params.limit.unwrap_or(50).clamp(1, 200);
    let offset = params.offset.unwrap_or(0);
    let notifs = queries::list_notifications(&state.db, auth.user_id, limit, offset).await?;
    let data: Vec<NotificationPublic> = notifs.into_iter().map(NotificationPublic::from).collect();
    Ok(Json(json!({"data": data})))
}

/// GET /api/notifications/unread-count
pub async fn unread_count(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<Value>, AppError> {
    let count = queries::count_unread_notifications(&state.db, auth.user_id).await?;
    Ok(Json(json!({"count": count})))
}

/// PUT /api/notifications/{id}/read
pub async fn mark_read(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    let notif = queries::mark_notification_read(&state.db, id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Notification not found".into()))?;
    let data: NotificationPublic = NotificationPublic::from(notif);
    Ok(Json(json!({"data": data})))
}

/// PUT /api/notifications/read-all
pub async fn mark_all_read(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<Value>, AppError> {
    let count = queries::mark_all_notifications_read(&state.db, auth.user_id).await?;
    Ok(Json(json!({"updated": count})))
}

/// DELETE /api/notifications/{id}
pub async fn delete(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    let deleted = queries::delete_notification(&state.db, id, auth.user_id).await?;
    if !deleted {
        return Err(AppError::NotFound("Notification not found".into()));
    }
    Ok(Json(json!({"deleted": true})))
}

/// The default notification preferences.
///
/// "push" means the in-app notification feed: Social Forge has exactly
/// one delivery channel today. There is no push transport and no email
/// transport, so the "email" option in the settings page is honoured as
/// a stored preference but has no transport behind it yet.
fn default_prefs() -> Value {
    json!({
        "post_published": "push",
        "post_failed": "push",
        "team_invite": "push",
        "analytics_weekly": "none",
        "quiet_hours_start": null,
        "quiet_hours_end": null,
        "timezone": 0
    })
}

/// GET /api/notifications/prefs
pub async fn get_prefs(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<Value>, AppError> {
    let stored: Option<String> =
        sqlx::query_scalar("SELECT prefs FROM notification_prefs WHERE user_id = ?")
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to read notification prefs: {e}")))?;

    let Some(raw) = stored else {
        return Ok(Json(default_prefs()));
    };
    // A row that does not parse falls back to the defaults rather than
    // erroring: a corrupt preference blob must not break the settings
    // page.
    let parsed: Option<Value> = serde_json::from_str(&raw).ok();
    Ok(Json(match parsed {
        Some(v) if v.is_object() => v,
        _ => default_prefs(),
    }))
}

/// PUT /api/notifications/prefs
pub async fn update_prefs(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    // Merge over the defaults so a partial body never drops a key.
    let mut merged = default_prefs();
    if let Some(obj) = body.as_object() {
        for (k, v) in obj {
            merged[k] = v.clone();
        }
    }

    let serialized = serde_json::to_string(&merged)
        .map_err(|e| AppError::Internal(format!("Failed to serialize notification prefs: {e}")))?;
    sqlx::query(
        "INSERT INTO notification_prefs (user_id, prefs, updated_at)
         VALUES (?, ?, unixepoch())
         ON CONFLICT(user_id) DO UPDATE SET prefs = excluded.prefs, updated_at = unixepoch()",
    )
    .bind(auth.user_id)
    .bind(&serialized)
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to save notification prefs: {e}")))?;

    Ok(Json(merged))
}
