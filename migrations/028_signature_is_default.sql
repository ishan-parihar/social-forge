-- Migration 028: Signature is_default column + set_default endpoint
--
-- Phase v21/v22: Adds an `is_default BOOLEAN` column to the `signatures`
-- table so the composer can auto-append the user's default signature
-- when creating a new post (postiz-app pattern: `onlyValues: [{content:
-- '\n' + signature.content}]`).
--
-- Design: at most ONE signature per (user_id, provider) can be the
-- default. provider = NULL is the global default. We enforce this with
-- a partial unique index.
--
-- The composer's auto-append behavior (Phase 5.3) is gated on the
-- user having a default signature — if none is set, no auto-append
-- happens (no surprising behavior changes for existing users).

ALTER TABLE signatures
    ADD COLUMN IF NOT EXISTS is_default BOOLEAN NOT NULL DEFAULT FALSE;

-- At most one default per (user_id, provider). NULL provider = global
-- default. The partial index only applies to rows where is_default is
-- TRUE, so non-default rows don't conflict.
CREATE UNIQUE INDEX IF NOT EXISTS idx_signatures_default_per_provider
    ON signatures(user_id, provider)
    WHERE is_default = TRUE;

COMMENT ON COLUMN signatures.is_default IS
    'If TRUE, this signature is auto-appended to new posts for its provider (or globally if provider is NULL). At most one default per (user_id, provider) enforced by idx_signatures_default_per_provider.';
