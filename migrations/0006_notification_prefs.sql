-- Notification preferences were never persisted.
--
-- `GET /api/notifications/prefs` returned hard-coded defaults and
-- `PUT /api/notifications/prefs` echoed the body back without writing
-- anything, so the settings page reported "Notification preferences
-- saved" and then showed the defaults again on the next visit. The
-- backend comment claimed "client stores locally" - the client never
-- did either.
--
-- One row per user. `prefs` holds the JSON object the API already
-- speaks, so the shape does not change when the preferences gain a
-- field.

CREATE TABLE IF NOT EXISTS notification_prefs (
    user_id    TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    prefs      TEXT NOT NULL DEFAULT '{}',
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);
