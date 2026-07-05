// ─── Server-Sent Events ───────────────────────────────────────
// Real-time event stream for the frontend and AI agents.
// All state changes (post created/published/failed, integration updates)
// are broadcast here.

use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive},
        IntoResponse, Sse,
    },
};
use futures::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::api::AppState;

/// GET /api/events — SSE stream
pub async fn sse_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let rx = state.broadcast.subscribe();
    // Map the BroadcastStream directly into SSE Events — no custom
    // ReceiverStream wrapper needed (axum's Sse accepts any Stream
    // of Result<Event, _>).
    let stream = BroadcastStream::new(rx).filter_map(|res| async move {
        match res {
            Ok(event) => {
                let data = serde_json::to_string(&event.data).unwrap_or_else(|_| "{}".into());
                let sse = Event::default().event(event.event).data(data);
                Some(Ok::<_, std::convert::Infallible>(sse))
            }
            Err(_) => None,
        }
    });
    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keepalive"),
    )
}
