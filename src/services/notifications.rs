use uuid::Uuid;

use crate::api::AppState;
use crate::db::queries;
use crate::db::models::NotificationPublic;
use sqlx::SqlitePool;
use crate::error::AppError;
use crate::realtime::Broadcaster;

pub struct NotificationService;

impl NotificationService {
    /// Create a notification and broadcast it via SSE.
    pub async fn create(
        state: &AppState,
        user_id: Uuid,
        title: &str,
        body: &str,
        notification_type: &str,
        reference_type: Option<&str>,
        reference_id: Option<&str>,
    ) -> Result<NotificationPublic, AppError> {
        Self::create_via(
            &state.db,
            &state.broadcast,
            user_id,
            title,
            body,
            notification_type,
            reference_type,
            reference_id,
        )
        .await
    }

    /// Same as [`Self::create`], for callers that hold the pool and
    /// broadcaster directly instead of a full [`AppState`] — background
    /// tasks in `src/scheduler/` have no HTTP state. Keeps a single
    /// implementation of the notify-then-broadcast path.
    pub async fn create_via(
        pool: &SqlitePool,
        broadcast: &Broadcaster,
        user_id: Uuid,
        title: &str,
        body: &str,
        notification_type: &str,
        reference_type: Option<&str>,
        reference_id: Option<&str>,
    ) -> Result<NotificationPublic, AppError> {
        let notif = queries::create_notification(
            pool,
            user_id,
            title,
            body,
            notification_type,
            reference_type,
            reference_id,
        )
        .await?;

        let public = NotificationPublic::from(notif);

        // Broadcast the notification event
        broadcast.send(
            "notification_new",
            &serde_json::json!({
                "user_id": user_id.to_string(),
                "notification": &public,
            }),
        );

        Ok(public)
    }
}
