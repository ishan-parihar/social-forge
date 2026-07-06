-- Track which comments the user has marked as "resolved" in the UI.
-- Comments themselves are fetched live from provider APIs (no local cache),
-- so this lightweight table is the only place resolved state can live.
-- Each row is a (user_id, comment_id) pair — provider-agnostic because
-- platform comment IDs are already globally unique (X snowflake, Reddit fullname, etc.).

CREATE TABLE IF NOT EXISTS resolved_comments (
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    comment_id TEXT NOT NULL,
    resolved_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, comment_id)
);

CREATE INDEX IF NOT EXISTS idx_resolved_comments_user
    ON resolved_comments(user_id);
