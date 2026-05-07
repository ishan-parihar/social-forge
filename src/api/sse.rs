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
use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio_stream::wrappers::BroadcastStream;
use futures::Stream;

use crate::api::AppState;
use crate::realtime::ServerEvent;

/// GET /api/events — SSE stream
pub async fn sse_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let rx = state.broadcast.subscribe();
    Sse::new(ReceiverStream { inner: BroadcastStream::new(rx) }).keep_alive(
        KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keepalive"),
    )
}

/// Wraps a broadcast receiver into a Stream using tokio-stream's BroadcastStream
struct ReceiverStream {
    inner: BroadcastStream<ServerEvent>,
}

impl Stream for ReceiverStream {
    type Item = Result<Event, Infallible>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(event))) => {
                let data = serde_json::to_string(&event.data).unwrap_or_else(|_| "{}".into());
                let sse = Event::default().event(event.event).data(data);
                Poll::Ready(Some(Ok(sse)))
            }
            Poll::Ready(Some(Err(_))) | Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
