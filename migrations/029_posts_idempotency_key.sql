-- Migration 029: Idempotency keys for posts
--
-- Phase v22: Adds an `idempotency_key` column to the `posts` table so
-- that provider.publish() can send a stable key with every publish
-- attempt. Providers that support idempotency (X, LinkedIn, etc.) will
-- deduplicate — if the same key is sent twice, the second request is
-- a no-op (returns the original post's platform_post_id) instead of
-- creating a duplicate post.
--
-- The double-publish risk this addresses:
--   1. Scheduler calls provider.publish() — succeeds on the platform.
--   2. The subsequent UPDATE posts SET state='published' fails (DB
--      connection drops, process crashes, etc.).
--   3. Post stays in 'publishing' state. On next startup,
--      reclaim_stuck_publishing marks it 'error'.
--   4. User clicks "Publish Now" → provider.publish() runs again →
--      DUPLICATE post created on the platform.
--
-- With idempotency keys:
--   - The key is generated once when the post is created (or when it
--     transitions from draft→queued).
--   - Every publish attempt for that post sends the SAME key.
--   - If the provider supports idempotency, the second publish returns
--     the original post's platform_post_id — no duplicate.
--
-- The key is a UUID v4 (random) — not derived from post content, so
-- editing a post's content doesn't change the key (which would defeat
-- the purpose). A NEW key is only generated when a published post is
-- re-scheduled for re-publishing (action='schedule' on reschedule).

ALTER TABLE posts
    ADD COLUMN IF NOT EXISTS idempotency_key UUID DEFAULT gen_random_uuid();

-- Backfill existing rows with a random UUID per row (the DEFAULT handles
-- new rows, but existing rows added the column as NULL).
UPDATE posts SET idempotency_key = gen_random_uuid() WHERE idempotency_key IS NULL;

-- Make it NOT NULL (keep the DEFAULT for future INSERTs that don't
-- explicitly set the key — e.g. create_post).
ALTER TABLE posts ALTER COLUMN idempotency_key SET NOT NULL;
ALTER TABLE posts ALTER COLUMN idempotency_key SET DEFAULT gen_random_uuid();

-- Index for looking up a post by its idempotency key (provider-side
-- dedup would query by this key if we ever expose it via API).
CREATE INDEX IF NOT EXISTS idx_posts_idempotency_key ON posts(idempotency_key);

COMMENT ON COLUMN posts.idempotency_key IS
    'Stable UUID sent to the provider on every publish attempt for this post. Providers that support idempotency deduplicate on this key — if the same key is sent twice (e.g. after a crash-recovery retry), the second publish is a no-op. A NEW key is generated only when a published post is re-scheduled for re-publishing (action=schedule on reschedule).';
