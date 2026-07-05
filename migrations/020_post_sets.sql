-- ─── 020: Post sets/templates ────────────────────────────────
-- Reusable post templates stored server-side (was localStorage only).
-- A set stores the full post payload (content, channels, media, settings)
-- as a JSONB blob so it can be loaded into the composer with one click.

CREATE TABLE IF NOT EXISTS post_sets (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    description TEXT,
    content     JSONB NOT NULL DEFAULT '{}',
    channel_ids UUID[] NOT NULL DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_post_sets_user_id ON post_sets(user_id);
