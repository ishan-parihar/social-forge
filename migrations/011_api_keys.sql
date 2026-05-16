-- API keys for programmatic access to the Social Forge API.
-- Keys are hashed with SHA-256 at rest; the full key is returned only once at creation.
-- Part of the Developer Portal feature.

CREATE TABLE IF NOT EXISTS api_keys (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name          TEXT NOT NULL,
    key_prefix    TEXT NOT NULL,       -- first 8 chars of key for display
    key_hash      TEXT NOT NULL,       -- SHA-256 hash of full key
    last_used_at  TIMESTAMPTZ,
    expires_at    TIMESTAMPTZ,
    is_active     BOOLEAN NOT NULL DEFAULT true,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_api_keys_user ON api_keys(user_id);
