use uuid::Uuid;

use crate::api::AppState;
use crate::db::queries;
use crate::db::models::NotificationPublic;
use crate::error::AppError;

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
        let notif = queries::create_notification(
            &state.db,
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
        state.broadcast.send(
            "notification_new",
            &serde_json::json!({
                "user_id": user_id.to_string(),
                "notification": &public,
            }),
        );

        Ok(public)
    }
}
