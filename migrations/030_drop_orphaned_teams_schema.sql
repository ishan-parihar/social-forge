-- Migration 030: Drop orphaned teams schema (YAGNI cleanup)
--
-- Phase v22: The teams/team_members/team_invitations tables and the
-- team_id columns on posts/integrations/media were added in migration
-- 010 for a multi-user team feature that was never implemented. The
-- v15 audit removed teams from the frontend; the v21 audit confirmed
-- no Rust code reads or writes these columns (the only `team_id` in
-- source is in slack.rs, which is Slack's own workspace ID concept,
-- not this column).
--
-- This migration drops the orphaned schema. It's safe because:
--   - team_id columns are nullable (no NOT NULL constraint)
--   - no indexes reference team_id (verified via \di)
--   - no Rust code references team_id (verified via grep)
--   - the tables have no rows in single-user deployments
--
-- If a future deployment actually needs teams, a fresh migration can
-- re-add the schema (the v15 frontend removal means there's no UI for
-- it anyway, so re-adding would be a fresh feature, not a restore).

-- Drop FK constraints that reference teams before dropping it.
ALTER TABLE posts DROP CONSTRAINT IF EXISTS posts_team_id_fkey;
ALTER TABLE integrations DROP CONSTRAINT IF EXISTS integrations_team_id_fkey;
ALTER TABLE media DROP CONSTRAINT IF EXISTS media_team_id_fkey;

-- Drop the orphaned tables.
DROP TABLE IF EXISTS team_invitations;
DROP TABLE IF EXISTS team_members;
DROP TABLE IF EXISTS teams;

-- Drop the orphaned team_id columns.
ALTER TABLE posts DROP COLUMN IF EXISTS team_id;
ALTER TABLE integrations DROP COLUMN IF EXISTS team_id;
ALTER TABLE media DROP COLUMN IF EXISTS team_id;

-- Note: subscriptions table is KEPT — it's actively used by billing.rs
-- (Stripe webhook handler). Only the teams feature was orphaned.
