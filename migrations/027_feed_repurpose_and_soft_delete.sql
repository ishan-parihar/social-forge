-- Migration 027: Feed repurpose provenance + posts soft-delete
--
-- Goals:
--   1. Add `posts.source_external_post_id` so a repurposed post can be
--      traced back to the imported feed post it was created from.
--   2. Add `posts.deleted_at` so deletes are soft (recoverable) and
--      the calendar/posts-list queries can filter them out by default.
--
-- Architectural note: social-forge is single-user (DEFAULT_USER_ID
-- hardcoded in src/auth/middleware.rs). We do NOT add multi-tenant
-- columns. We also do NOT backfill `deleted_at` for existing rows —
-- NULL means "not deleted", which is the correct default.

-- (1) Repurpose provenance FK
ALTER TABLE posts
    ADD COLUMN IF NOT EXISTS source_external_post_id UUID
        REFERENCES external_posts(id) ON DELETE SET NULL;

COMMENT ON COLUMN posts.source_external_post_id IS
    'If this post was created by repurposing an imported feed post, this FK points to the source external_posts row. NULL otherwise. ON DELETE SET NULL so hiding/deleting the feed post does not cascade to the repurposed post.';

-- (2) Soft delete for posts
ALTER TABLE posts
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ;

COMMENT ON COLUMN posts.deleted_at IS
    'Soft-delete timestamp. NULL = not deleted. Set by DELETE /api/posts/{id} (and group-cascade). Filtered out by default in list/calendar queries.';

-- Index for the soft-delete filter (partial index — only non-deleted rows)
CREATE INDEX IF NOT EXISTS idx_posts_not_deleted
    ON posts(user_id, state)
    WHERE deleted_at IS NULL;

-- Index for repurpose-provenance lookup (reverse: given a feed post, find repurposed posts)
CREATE INDEX IF NOT EXISTS idx_posts_source_external_post_id
    ON posts(source_external_post_id)
    WHERE source_external_post_id IS NOT NULL;
