-- Phase 7: Campaigns table for strategic content management.
--
-- A campaign is a named group of posts with a color, description, and
-- optional date range. Posts can be assigned to a campaign via the
-- existing group_id field (reused) or the new campaign_id FK.
--
-- The kanban board groups posts by their post_state (idea, draft,
-- queued, published) and optionally by campaign.

CREATE TABLE IF NOT EXISTS campaigns (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    description TEXT,
    color       TEXT NOT NULL DEFAULT '#6366f1',
    start_date  DATE,
    end_date    DATE,
    goal        TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_campaigns_user ON campaigns(user_id);

-- Add 'idea' to the post_state enum for the kanban "Ideas" column.
-- This is idempotent — if the value already exists, it's a no-op.
DO $$
BEGIN
    ALTER TYPE post_state ADD VALUE IF NOT EXISTS 'idea';
EXCEPTION WHEN OTHERS THEN
    -- Ignore if the type doesn't exist or the value is already there.
    NULL;
END $$;

-- Add campaign_id to posts (nullable FK to campaigns).
ALTER TABLE posts ADD COLUMN IF NOT EXISTS campaign_id UUID REFERENCES campaigns(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_posts_campaign ON posts(campaign_id) WHERE campaign_id IS NOT NULL;
