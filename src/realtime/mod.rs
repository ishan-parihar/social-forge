// ─── Real-Time Event Broadcasting ─────────────────────────────
// Central event bus using tokio's broadcast channel.
// Both the SvelteKit frontend (via SSE) and AI agents (via MCP tools)
// subscribe to events to get real-time state changes.

use serde::Serialize;
use tokio::sync::broadcast;

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

    /// Broadcast an event to all subscribers
    pub fn send(&self, event: &'static str, data: &impl Serialize) {
        let event = ServerEvent {
            event,
            data: serde_json::to_value(data).unwrap_or_default(),
        };
        if self.tx.receiver_count() > 0 {
            let _ = self.tx.send(event);
        }
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
