-- v22 Phase 2 (D.2): Transactional outbox for publishes.
--
-- Problem: when `provider.publish()` succeeds but the subsequent
-- `UPDATE posts SET state='published'` fails (e.g. DB connection blip),
-- the post is live on the platform but the DB still shows `publishing`.
-- The 3-attempt DB-write retry in the scheduler mitigates this, but a
-- durable log is more robust.
--
-- Design: after `provider.publish()` returns, the scheduler writes the
-- result to BOTH `posts` (source of truth for state) AND `publish_outbox`
-- (durability log). If the `posts` write fails, the outbox drain loop
-- (runs every 30s alongside `process_due_posts`) retries it.
--
-- This is NOT a full transactional-outbox pattern (which would write to
-- the outbox in the SAME transaction as the state change). That would
-- require wrapping the publish+write in a Postgres transaction, which
-- doesn't work because the publish is an HTTP call. Instead, this is a
-- "best-effort durability log" — the publish writes to the outbox
-- immediately after success, and the drain loop reconciles. If the
-- outbox write itself fails, we fall back to the existing 3-attempt
-- retry on the `posts` write.
--
-- Architectural preference: single-binary, no Temporal, no Redis. The
-- drain loop is a tokio task, the outbox is a Postgres table. This is
-- the maximum durability achievable within the single-binary constraint.

CREATE TABLE IF NOT EXISTS publish_outbox (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id         UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    idempotency_key UUID NOT NULL,
    -- Result of the successful publish (NULL until publish succeeds)
    platform_post_id  TEXT,
    platform_post_url TEXT,
    published_at      TIMESTAMPTZ,
    -- Error message if the publish failed AFTER the outbox row was
    -- written (rare — usually errors prevent the outbox write).
    error_message     TEXT,
    -- Drain-loop bookkeeping
    attempts          INT NOT NULL DEFAULT 0,
    next_attempt_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- NULL until the result has been successfully applied to `posts`.
    completed_at      TIMESTAMPTZ
);

-- Index for the drain loop: fetch pending rows ordered by creation.
CREATE INDEX IF NOT EXISTS idx_publish_outbox_pending
    ON publish_outbox(next_attempt_at)
    WHERE completed_at IS NULL;

-- Index for dedup lookups (check if a post already has a successful publish).
CREATE INDEX IF NOT EXISTS idx_publish_outbox_post_id
    ON publish_outbox(post_id);

-- Index for idempotency-key lookups (check if a key was already used).
CREATE INDEX IF NOT EXISTS idx_publish_outbox_idempotency
    ON publish_outbox(idempotency_key)
    WHERE platform_post_id IS NOT NULL;
