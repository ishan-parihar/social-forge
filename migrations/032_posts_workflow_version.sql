-- v22 Phase 2 (D.4): Workflow versioning for the publish state machine.
--
-- Problem: when the publish state machine logic changes (e.g. adding
-- idempotency-key handling in v22 Phase 2), existing `publishing` posts
-- are handled by the NEW code, which may or may not be compatible with
-- the state they were claimed under. Postiz solves this with Temporal
-- workflow versions (v1.0.1 → v1.0.5); we use a simpler in-code approach.
--
-- Design: add `publish_workflow_version INT NOT NULL DEFAULT 1` to
-- `posts`. When the scheduler claims a post, it reads the version and
-- dispatches to the corresponding `publish_v1`, `publish_v2`, etc.
-- function. New posts get the latest version (set at create time);
-- existing posts keep their original version until they complete.
--
-- Version history:
--   1 = original publish logic (pre-v22)
--   2 = v22 Phase 2: idempotency-key header on X v2, outbox write,
--       abort-on-timeout, permit-inside-spawn
--
-- After all in-flight v1 posts complete, v1 can be deprecated (the
-- dispatcher falls through to v2 for unknown versions, so old rows
-- are never stuck).

ALTER TABLE posts ADD COLUMN IF NOT EXISTS publish_workflow_version INT NOT NULL DEFAULT 1;

-- Backfill: any post still in 'publishing' or 'queued' state gets v2
-- (the new logic is backwards-compatible — it just adds idempotency
-- headers and outbox writes that v1 posts would have skipped).
UPDATE posts
SET publish_workflow_version = 2
WHERE state IN ('queued', 'publishing') AND deleted_at IS NULL;

-- New posts default to v2 (the current latest).
ALTER TABLE posts ALTER COLUMN publish_workflow_version SET DEFAULT 2;
