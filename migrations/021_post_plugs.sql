-- ─── 021: Outbound post plugs ────────────────────────────────
-- Plugs are post-publish automations: auto-repost after N likes,
-- cross-post from a secondary account, etc.
-- The plug_runner background task checks due plugs every minute and
-- executes them. No Temporal/Redis needed — simple tokio task + DB.

CREATE TABLE IF NOT EXISTS post_plugs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    post_id         UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    integration_id  UUID NOT NULL REFERENCES integrations(id) ON DELETE CASCADE,
    plug_type       TEXT NOT NULL CHECK (plug_type IN ('auto_repost_after_likes', 'cross_post_from_secondary')),
    config          JSONB NOT NULL DEFAULT '{}',
    -- For auto_repost: { threshold: 10, max_runs: 3, interval_minutes: 360 }
    -- For cross_post: { secondary_integration_id: "uuid", delay_minutes: 30 }
    runs_so_far     INTEGER NOT NULL DEFAULT 0,
    max_runs        INTEGER NOT NULL DEFAULT 1,
    next_run_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    fired_at        TIMESTAMPTZ,
    completed       BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_post_plugs_due ON post_plugs(next_run_at)
    WHERE completed = false;
CREATE INDEX IF NOT EXISTS idx_post_plugs_post ON post_plugs(post_id);
