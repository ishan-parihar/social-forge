-- Add recurring post support
ALTER TABLE posts
  ADD COLUMN repeat_interval_days INT,
  ADD COLUMN repeat_end_date TIMESTAMPTZ,
  ADD COLUMN group_id UUID;

-- Index for faster group lookups (e.g. deleting all posts in a series)
CREATE INDEX IF NOT EXISTS idx_posts_group_id ON posts(group_id);
