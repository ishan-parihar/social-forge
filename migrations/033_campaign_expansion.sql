-- v22 Phase 6: Campaign model expansion.
--
-- Adds: status (active/paused/archived/completed), progress_metric,
-- progress_target, audience_persona (JSONB), content_pillars (JSONB),
-- budget_cents, kpi_targets (JSONB), deleted_at (soft delete),
-- sort_order (manual ordering).
--
-- Also adds soft-delete to campaigns (BUG #10 fix) — previously
-- DELETE /api/campaigns/{id} was a hard delete with no recovery.

ALTER TABLE campaigns ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'active'
  CHECK (status IN ('active', 'paused', 'archived', 'completed'));
ALTER TABLE campaigns ADD COLUMN IF NOT EXISTS progress_metric TEXT;
  -- 'posts' | 'engagement' | 'reach' | 'followers' | 'custom'
ALTER TABLE campaigns ADD COLUMN IF NOT EXISTS progress_target INT;
ALTER TABLE campaigns ADD COLUMN IF NOT EXISTS audience_persona JSONB;
  -- {age_range, location, interests: [], pain_points: []}
ALTER TABLE campaigns ADD COLUMN IF NOT EXISTS content_pillars JSONB;
  -- [{title, description, tags: []}]
ALTER TABLE campaigns ADD COLUMN IF NOT EXISTS budget_cents INT;
  -- for paid amplification tracking (cents to avoid float rounding)
ALTER TABLE campaigns ADD COLUMN IF NOT EXISTS kpi_targets JSONB;
  -- {min_engagement_rate, min_reach, target_clicks}
ALTER TABLE campaigns ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ;
ALTER TABLE campaigns ADD COLUMN IF NOT EXISTS sort_order INT NOT NULL DEFAULT 0;

-- Index for listing active campaigns (excludes archived + soft-deleted).
CREATE INDEX IF NOT EXISTS idx_campaigns_status
  ON campaigns(user_id, status)
  WHERE deleted_at IS NULL;

-- Index for soft-delete filtering.
CREATE INDEX IF NOT EXISTS idx_campaigns_not_deleted
  ON campaigns(user_id)
  WHERE deleted_at IS NULL;
