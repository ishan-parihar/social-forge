-- Phase 3: soft-hide + bookmark for external_posts (feed items).
--
-- Instead of hard-deleting feed items (which re-imports them on the next
-- refresh cycle), we add a hidden_at column for soft-hide and a saved_at
-- column for bookmarking items the user wants to revisit.

ALTER TABLE external_posts ADD COLUMN IF NOT EXISTS hidden_at TIMESTAMPTZ NULL;
ALTER TABLE external_posts ADD COLUMN IF NOT EXISTS saved_at TIMESTAMPTZ NULL;

COMMENT ON COLUMN external_posts.hidden_at IS
  'If set, this feed post is hidden from the feed view. Re-import will NOT clear this (soft-hide persists across refresh cycles).';
COMMENT ON COLUMN external_posts.saved_at IS
  'If set, this feed post has been bookmarked by the user for later reference.';
