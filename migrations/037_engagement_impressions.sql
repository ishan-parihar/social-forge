-- v25: post_engagement.impressions
--
-- `GET /api/analytics/engagement` and the MCP `feed_analytics` tool both read
-- `post_engagement.impressions`, but the column was never created — migration
-- 016 defined the table with `views` only. Every call to those endpoints failed
-- with SQLSTATE 42703 ("column pe.impressions does not exist").
--
-- `views` and `impressions` are the same quantity on every platform we ingest
-- (see src/social/mod.rs: views is parsed from "views" | "plays" | "reach" |
-- "impressions"), so existing rows are backfilled from `views` rather than left
-- at 0 — otherwise historical charts would silently read as flat-zero.
--
-- Additive only: one column, no drops, no renames. Idempotent via IF NOT EXISTS
-- so a re-run on an already-migrated database is a no-op.

ALTER TABLE post_engagement
    ADD COLUMN IF NOT EXISTS impressions INTEGER NOT NULL DEFAULT 0;

-- Backfill: impressions := views for rows that predate this migration.
UPDATE post_engagement
   SET impressions = views;

COMMENT ON COLUMN post_engagement.impressions IS
    'Impression/reach count across all platforms (aliases: views, plays, reach)';
