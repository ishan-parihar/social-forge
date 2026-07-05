-- ─── 018: Add 'publishing' state + audit trail ──────────────────
--
-- Purpose: fix the dual-instance double-publish + kill-mid-publish-on-shutdown
-- bugs identified in the v9 production-readiness audit.
--
-- The scheduler now atomically transitions queued → publishing before
-- calling provider.publish(), then publishing → published/error after.
-- This means:
--   (a) Two `social-forge serve` instances can't both pull the same
--       queued post (the `WHERE state = 'queued'` claim skips rows
--       already in `publishing` state).
--   (b) On shutdown, posts stuck in `publishing` are visibly
--       distinct from `queued` — the operator can see "3 posts were
--       mid-flight when the process died" and decide whether to
--       retry them manually.
--
-- Also adds `publish_attempts` table for a full audit trail of every
-- publish attempt (success or failure), and `retry_count`/`next_retry_at`
-- columns on `posts` for proper exponential backoff tracking.

-- Add 'publishing' to the post_state enum.
-- ALTER TYPE ... ADD VALUE must be outside a transaction block in older
-- Postgres, but sqlx runs each migration file as one tx. We use the
-- `IF NOT EXISTS` guard (Postgres 9.3+) so re-running is safe.
ALTER TYPE post_state ADD VALUE IF NOT EXISTS 'publishing' AFTER 'queued';

-- Track retry state on the post itself so the scheduler can resume
-- backoff across ticks (was: in-memory only, lost on restart).
ALTER TABLE posts ADD COLUMN IF NOT EXISTS retry_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE posts ADD COLUMN IF NOT EXISTS next_retry_at TIMESTAMPTZ;

-- Full audit trail of every publish attempt. Each row = one call to
-- provider.publish(). The scheduler writes one row per attempt with
-- the error (or success) result, so the operator can see "attempted
-- at T1 with 429, attempted at T2 with 429, attempted at T3 with
-- success" — previously only the last error was kept.
CREATE TABLE IF NOT EXISTS publish_attempts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id         UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    attempt_number  INTEGER NOT NULL,
    status          TEXT NOT NULL CHECK (status IN ('success', 'failed')),
    error_message   TEXT,
    started_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finished_at     TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for querying "all attempts for this post, ordered by attempt #"
CREATE INDEX IF NOT EXISTS idx_publish_attempts_post_id
    ON publish_attempts(post_id, attempt_number);

-- Index for querying "recent failures across all posts" (dashboard)
CREATE INDEX IF NOT EXISTS idx_publish_attempts_status_started
    ON publish_attempts(status, started_at DESC);
