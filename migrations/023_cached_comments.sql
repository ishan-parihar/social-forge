-- Cache layer for platform comments (B-3).
--
-- Problem: the comments list endpoint was making 50 sequential network
-- calls per page load — one per external post — to fetch live comments
-- from each provider. That made the page take 10+ seconds to render
-- and rate-limited the user's X/Reddit/Bluesky accounts.
--
-- Solution: cache comments in this table. The background feed refresher
-- (which already pulls posts and engagement metrics) now also pulls
-- comments and writes them here. The comments list endpoint reads
-- from the cache instead of doing live API calls.
--
-- A comment is uniquely identified by (user_id, platform_comment_id).
-- Platform comment IDs are globally unique (X snowflake, Reddit fullname,
-- etc.) so we don't need a separate provider column in the PK, but we
-- keep `provider` as a regular column for filtering.

CREATE TABLE IF NOT EXISTS cached_comments (
    id                  BIGSERIAL PRIMARY KEY,
    user_id             UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    comment_id          TEXT NOT NULL,
    post_id             UUID NOT NULL REFERENCES external_posts(id) ON DELETE CASCADE,
    provider            TEXT NOT NULL,
    author_name         TEXT,
    author_handle       TEXT,
    author_avatar       TEXT,
    text                TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL,
    fetched_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, comment_id)
);

-- Index for the common "list all comments for this user, newest first" query.
CREATE INDEX IF NOT EXISTS idx_cached_comments_user_created
    ON cached_comments(user_id, created_at DESC);

-- Index for filtering by provider.
CREATE INDEX IF NOT EXISTS idx_cached_comments_user_provider
    ON cached_comments(user_id, provider);

-- Index for joining back to external_posts (already FK-indexed by the
-- CASCADE, but add an explicit index for the comments::list join).
CREATE INDEX IF NOT EXISTS idx_cached_comments_post_id
    ON cached_comments(post_id);
