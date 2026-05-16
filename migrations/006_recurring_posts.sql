-- Add recurring post support (idempotent)
ALTER TABLE posts
  ADD COLUMN IF NOT EXISTS repeat_interval_days INT,
  ADD COLUMN IF NOT EXISTS repeat_end_date TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS group_id UUID;

-- Index for faster group lookups (e.g. deleting all posts in a series)
CREATE INDEX IF NOT EXISTS idx_posts_group_id ON posts(group_id);
