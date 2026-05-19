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
    let limit = params.limit.unwrap_or(50).min(200);
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

/// GET /api/notifications/prefs
pub async fn get_prefs(
    State(_state): State<AppState>,
    _auth: AuthenticatedUser,
) -> Json<Value> {
    // Return defaults — prefs are stored client-side for now
    Json(json!({
        "post_published": "push",
        "post_failed": "push",
        "team_invite": "push",
        "analytics_weekly": "none",
        "quiet_hours_start": null,
        "quiet_hours_end": null,
        "timezone": 0
    }))
}

/// PUT /api/notifications/prefs
pub async fn update_prefs(
    State(_state): State<AppState>,
    _auth: AuthenticatedUser,
    Json(body): Json<Value>,
) -> Json<Value> {
    // Accept and echo back — client stores locally
    let defaults = json!({
        "post_published": "push",
        "post_failed": "push",
        "team_invite": "push",
        "analytics_weekly": "none",
        "quiet_hours_start": null,
        "quiet_hours_end": null,
        "timezone": 0
    });
    let mut merged = defaults;
    if let Some(obj) = body.as_object() {
        for (k, v) in obj {
            merged[k] = v.clone();
        }
    }
    Json(merged)
}
