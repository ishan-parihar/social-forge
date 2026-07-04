use crate::api::AppState;
use crate::cli::NotificationsAction;

pub async fn handle(action: NotificationsAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        NotificationsAction::List { limit } => {
            let user_id = match crate::mcp::tools_posts::resolve_first_user(state).await {
                Ok(id) => id,
                Err(e) => return Err(anyhow::anyhow!("Auth error: {e}")),
            };
            let notifs = match crate::db::queries::list_notifications(&state.db, user_id, limit as i64, 0).await {
                Ok(n) => n,
                Err(e) => return Err(anyhow::anyhow!("DB error: {e}")),
            };
            let data: Vec<_> = notifs.into_iter().map(|n| {
                let p: crate::db::models::NotificationPublic = n.into();
                serde_json::to_value(p).unwrap_or_default()
            }).collect();
            Ok(serde_json::json!({ "data": data }))
        }
        NotificationsAction::Read { id } => {
            let user_id = match crate::mcp::tools_posts::resolve_first_user(state).await {
                Ok(id) => id,
                Err(e) => return Err(anyhow::anyhow!("Auth error: {e}")),
            };
            let notif_id = match uuid::Uuid::parse_str(&id) {
                Ok(id) => id,
                Err(_) => return Err(anyhow::anyhow!("Invalid notification ID")),
            };
            let notif = match crate::db::queries::mark_notification_read(&state.db, notif_id, user_id).await {
                Ok(Some(n)) => n,
                Ok(None) => return Err(anyhow::anyhow!("Notification not found")),
                Err(e) => return Err(anyhow::anyhow!("DB error: {e}")),
            };
            let p: crate::db::models::NotificationPublic = notif.into();
            Ok(serde_json::json!({ "data": serde_json::to_value(p).unwrap_or_default() }))
        }
        NotificationsAction::ReadAll => {
            let user_id = match crate::mcp::tools_posts::resolve_first_user(state).await {
                Ok(id) => id,
                Err(e) => return Err(anyhow::anyhow!("Auth error: {e}")),
            };
            let count = match crate::db::queries::mark_all_notifications_read(&state.db, user_id).await {
                Ok(c) => c,
                Err(e) => return Err(anyhow::anyhow!("DB error: {e}")),
            };
            Ok(serde_json::json!({ "updated": count }))
        }
        NotificationsAction::Create { title, body } => {
            // Route through the MCP handler so the notification is created
            // via queries::create_notification (consistent timestamps,
            // is_read=false, notification_type, etc.) instead of inlining
            // a raw SQL INSERT that bypasses those defaults.
            let input = crate::mcp::tools_notifications::NotifCreateInput {
                title,
                body: Some(body),
                notification_type: "manual".into(),
                reference_type: None,
                reference_id: None,
            };
            crate::mcp::tools_notifications::handle_notif_create(state, &input)
                .await
                .map(|v| v.0)
                .map_err(|e| e.to_string())
        }
    };

    super::emit_result(result)
}
