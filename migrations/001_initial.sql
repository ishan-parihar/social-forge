-- Postiz-Rust initial schema
-- Run automatically on startup via sqlx::migrate!

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ── Users ──────────────────────────────────────────────────
-- Single-user MVP, but schema supports multi-user from day 1.

CREATE TABLE IF NOT EXISTS users (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email       TEXT NOT NULL UNIQUE,
    password    TEXT NOT NULL,          -- argon2 hash
    name        TEXT NOT NULL DEFAULT '',
    timezone    INT NOT NULL DEFAULT 0, -- UTC offset in minutes
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ── Social Channel Integrations ───────────────────────────

CREATE TABLE IF NOT EXISTS integrations (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id               UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider_identifier   TEXT NOT NULL,   -- 'x', 'linkedin', 'bluesky', etc.
    provider_name         TEXT NOT NULL DEFAULT '',
    internal_id           TEXT NOT NULL,   -- platform-specific user/page ID
    access_token          TEXT NOT NULL DEFAULT '',
    refresh_token         TEXT DEFAULT '',
    token_expires_at      TIMESTAMPTZ,
    profile_name          TEXT DEFAULT '',
    profile_picture       TEXT DEFAULT '',
    profile_url           TEXT DEFAULT '',
    disabled              BOOLEAN NOT NULL DEFAULT false,
    refresh_needed        BOOLEAN NOT NULL DEFAULT false,
    posting_times         JSONB NOT NULL DEFAULT '[{"time":120},{"time":400},{"time":700}]',
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, provider_identifier, internal_id)
);

-- ── Posts ─────────────────────────────────────────────────

CREATE TYPE post_state AS ENUM ('draft', 'queued', 'published', 'error');

CREATE TABLE IF NOT EXISTS posts (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id           UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    integration_id    UUID NOT NULL REFERENCES integrations(id) ON DELETE CASCADE,
    state             post_state NOT NULL DEFAULT 'draft',
    content           TEXT NOT NULL DEFAULT '',
    title             TEXT DEFAULT '',
    media             JSONB DEFAULT '[]'::jsonb,   -- array of {url, mime_type, alt?}
    settings          JSONB DEFAULT '{}'::jsonb,    -- provider-specific settings
    scheduled_at      TIMESTAMPTZ,                   -- when to publish (null = draft/now)
    published_at      TIMESTAMPTZ,                   -- actual publish time
    platform_post_id  TEXT DEFAULT '',               -- ID returned by platform
    platform_post_url TEXT DEFAULT '',               -- URL of the post on the platform
    error_message     TEXT DEFAULT '',
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ── Media Files ───────────────────────────────────────────

CREATE TABLE IF NOT EXISTS media (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    original_name   TEXT NOT NULL,
    storage_path    TEXT NOT NULL,
    mime_type       TEXT NOT NULL DEFAULT 'application/octet-stream',
    file_size       BIGINT NOT NULL DEFAULT 0,
    width           INT,
    height          INT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ── OAuth State Store ─────────────────────────────────────
-- Ephemeral: stores PKCE code_verifier + state during OAuth flow.
-- Cleaned up after callback completes or expires.

CREATE TABLE IF NOT EXISTS oauth_states (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    state           TEXT NOT NULL UNIQUE,
    provider        TEXT NOT NULL,
    code_verifier   TEXT NOT NULL,
    redirect_uri    TEXT DEFAULT '',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at      TIMESTAMPTZ NOT NULL DEFAULT (now() + INTERVAL '10 minutes')
);

-- ── Indexes ───────────────────────────────────────────────

CREATE INDEX IF NOT EXISTS idx_posts_user_state      ON posts(user_id, state);
CREATE INDEX IF NOT EXISTS idx_posts_scheduled       ON posts(scheduled_at) WHERE state = 'queued';
CREATE INDEX IF NOT EXISTS idx_integrations_user     ON integrations(user_id);
CREATE INDEX IF NOT EXISTS idx_integrations_provider ON integrations(provider_identifier);
CREATE INDEX IF NOT EXISTS idx_oauth_states_state    ON oauth_states(state);
CREATE INDEX IF NOT EXISTS idx_oauth_states_expires  ON oauth_states(expires_at);
CREATE INDEX IF NOT EXISTS idx_media_user            ON media(user_id);
