-- v23: Events log table for the dashboard's "Recent Activity" widget.
--
-- The Broadcaster only fires events when a subscriber is connected (see
-- src/realtime/mod.rs:send() — `if self.tx.receiver_count() > 0`). This
-- means events fired while no SSE client is connected are lost forever.
-- For a "recent activity" widget on the dashboard, we need to persist
-- the last N events so they can be queried on page load.
--
-- Design: a simple append-only log. A background cleanup task trims
-- rows older than 7 days (configurable via EVENTS_LOG_RETENTION_DAYS)
-- to prevent unbounded growth. The dashboard queries the last 10-50.
--
-- This is NOT a full audit log (no user_id scoping beyond the single-
-- user model, no immutable retention). It's a best-effort recent-
-- activity feed. A proper audit log would be a separate table with
-- different retention rules.

CREATE TABLE IF NOT EXISTS events_log (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    event_type  TEXT NOT NULL,
    -- The event payload (JSONB). Matches what the Broadcaster sends
    -- over SSE, so the dashboard can render the same data.
    payload     JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for the dashboard's "recent activity" query (last N by user).
CREATE INDEX IF NOT EXISTS idx_events_log_user_created
    ON events_log(user_id, created_at DESC);

-- Index for the cleanup task (find rows older than retention).
CREATE INDEX IF NOT EXISTS idx_events_log_created
    ON events_log(created_at);
