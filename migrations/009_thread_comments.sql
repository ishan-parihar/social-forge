-- Thread + First Comment support
-- Adds columns for thread sequencing and per-post first comments

ALTER TABLE posts ADD COLUMN IF NOT EXISTS first_comment TEXT;
ALTER TABLE posts ADD COLUMN IF NOT EXISTS sequence INT NOT NULL DEFAULT 0;

-- Index already exists from 006_recurring_posts.sql, but ensure it does
CREATE INDEX IF NOT EXISTS idx_posts_group_id ON posts(group_id);
