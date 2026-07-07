// ─── Real-Time Event Broadcasting ─────────────────────────────
// Central event bus using tokio's broadcast channel.
// Both the SvelteKit frontend (via SSE) and AI agents (via MCP tools)
// subscribe to events to get real-time state changes.
//
// v23: `send_and_log` persists events to the events_log table so the
// dashboard's "Recent Activity" widget can show events that happened
// while no SSE client was connected. The Broadcaster itself doesn't
// hold a DB handle (it's created before the DB pool is ready), so
// `send_and_log` takes the DB pool as a parameter.

use serde::Serialize;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::db::PgPool;

/// An event emitted by the system
#[derive(Debug, Clone, Serialize)]
pub struct ServerEvent {
    pub event: &'static str,
    pub data: serde_json::Value,
}

/// Thread-safe event broadcaster
#[derive(Clone)]
pub struct Broadcaster {
    tx: broadcast::Sender<ServerEvent>,
}

impl Broadcaster {
    /// Create a new broadcaster with a 1024-event buffer
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self { tx }
    }

    /// Broadcast an event to all subscribers.
    ///
    /// Note: if no SSE client is connected, the event is silently
    /// dropped (no persistence). Use `send_and_log` if the event
    /// should appear in the dashboard's "Recent Activity" widget.
    pub fn send(&self, event: &'static str, data: &impl Serialize) {
        let event = ServerEvent {
            event,
            data: serde_json::to_value(data).unwrap_or_default(),
        };
        if self.tx.receiver_count() > 0 {
            let _ = self.tx.send(event);
        }
    }

    /// Broadcast an event AND persist it to the events_log table so
    /// the dashboard's "Recent Activity" widget can show it later
    /// (even if no SSE client was connected at the time).
    ///
    /// v23: this closes the "events lost when no client connected" gap
    /// identified in the v22 audit (Part C.5 #16). The log write is
    /// best-effort — failures are logged but don't break the broadcast.
    pub async fn send_and_log(
        &self,
        db: &PgPool,
        user_id: Uuid,
        event: &'static str,
        data: &impl Serialize,
    ) {
        let data_value = serde_json::to_value(data).unwrap_or_default();
        // Broadcast first (don't block the SSE push on the DB write).
        if self.tx.receiver_count() > 0 {
            let _ = self.tx.send(ServerEvent { event, data: data_value.clone() });
        }
        // Persist to events_log (best-effort).
        let _ = sqlx::query(
            r#"INSERT INTO events_log (user_id, event_type, payload)
               VALUES ($1, $2, $3)"#,
        )
        .bind(user_id)
        .bind(event)
        .bind(&data_value)
        .execute(db)
        .await
        .map_err(|e| {
            tracing::warn!("Failed to persist event '{event}' to events_log: {e}");
            e
        });
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<ServerEvent> {
        self.tx.subscribe()
    }
}

impl Default for Broadcaster {
    fn default() -> Self {
        Self::new()
    }
}
