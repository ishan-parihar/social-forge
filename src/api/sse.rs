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
    // On lag (the client fell behind the broadcast channel's buffer),
    // we emit a synthetic `"lagged"` event with `{ "lagged": true,
    // "count": n }` so the frontend can trigger a refetch of the
    // current view. Without this signal, a tab that was backgrounded
    // for >1024 events would silently miss updates and show stale data.
    //
    // We match on the error variant by inspecting the Debug string
    // (rather than importing the exact error type path, which varies
    // across tokio-stream versions). The `Lagged(n)` variant is the
    // only error BroadcastStream produces.
    let stream = BroadcastStream::new(rx).filter_map(|res| async move {
        match res {
            Ok(event) => {
                let data = serde_json::to_string(&event.data).unwrap_or_else(|_| "{}".into());
                let sse = Event::default().event(event.event).data(data);
                Some(Ok::<_, std::convert::Infallible>(sse))
            }
            Err(_e) => {
                // BroadcastStreamRecvError has one variant: Lagged(count).
                // Extract the count from the Debug string as a best-effort
                // (avoids depending on the exact error-type path).
                let dbg = format!("{_e:?}");
                let count = dbg
                    .split_whitespace()
                    .find_map(|tok| tok.trim_start_matches('(').trim_end_matches(')').parse::<u64>().ok())
                    .unwrap_or(0);
                let data = serde_json::json!({ "lagged": true, "count": count }).to_string();
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
