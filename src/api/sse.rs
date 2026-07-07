// ─── Server-Sent Events ───────────────────────────────────────
// Real-time event stream for the frontend and AI agents.
// All state changes (post created/published/failed, integration updates)
// are broadcast here.
//
// v22 Phase 1: This endpoint now lives behind the auth middleware
// (moved from `public_routes` to `protected_routes` in `mod.rs`).
// The auth_middleware validates the `sf_session` cookie before the
// request reaches `sse_handler`, so unauthenticated clients get 401
// instead of subscribing to all events. This closes BUG #19 (SSE
// auth bypass on networked deployments with BIND_HOST=0.0.0.0).
//
// v22 Phase 1: Lagged events now emit a synthetic `"lagged"` SSE
// event so the frontend can refetch stale views (BUG #18). Previously
// `BroadcastStream::Lagged(n)` was silently swallowed, leaving the
// calendar/kanban/dashboard out of sync after a tab was backgrounded.

use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive},
        IntoResponse, Sse,
    },
};
use futures::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
// `BroadcastStreamRecvError` is exported by tokio-stream alongside
// `BroadcastStream`. Re-importing it here so we can pattern-match on
// the `Lagged` variant without the long path.
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;

use crate::api::AppState;

/// GET /api/events — SSE stream (auth-gated; see `mod.rs`)
pub async fn sse_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let rx = state.broadcast.subscribe();
    // Map the BroadcastStream directly into SSE Events — no custom
    // ReceiverStream wrapper needed (axum's Sse accepts any Stream
    // of Result<Event, _>).
    //
    // On `Err(BroadcastStreamRecvError::Lagged(n))` we emit a synthetic
    // `"lagged"` event with `{ "lagged": true, "count": n }` so the
    // frontend can trigger a refetch of the current view. Without this
    // signal, a tab that was backgrounded for >1024 events would
    // silently miss updates and show stale data with no indication.
    let stream = BroadcastStream::new(rx).filter_map(|res| async move {
        match res {
            Ok(event) => {
                let data = serde_json::to_string(&event.data).unwrap_or_else(|_| "{}".into());
                let sse = Event::default().event(event.event).data(data);
                Some(Ok::<_, std::convert::Infallible>(sse))
            }
            Err(BroadcastStreamRecvError::Lagged(count)) => {
                // Tell the frontend it missed `count` events so it can
                // refetch the active view (calendar, kanban, dashboard).
                let data = serde_json::json!({ "lagged": true, "count": count })
                    .to_string();
                let sse = Event::default().event("lagged").data(data);
                Some(Ok(sse))
            }
        }
    });
    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keepalive"),
    )
}
