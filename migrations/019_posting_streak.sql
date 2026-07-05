-- ─── 019: Add posting streak tracking ───────────────────────
-- The streak counter motivates the solo founder to post daily.
-- A flame icon in the top bar shows the current day streak.
-- The streak resets if no post is published within 24 hours.

ALTER TABLE users ADD COLUMN IF NOT EXISTS streak_since TIMESTAMPTZ;
ALTER TABLE users ADD COLUMN IF NOT EXISTS streak_days INTEGER NOT NULL DEFAULT 0;
