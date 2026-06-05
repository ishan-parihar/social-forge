-- External Post Import: store imported posts from social platforms
CREATE TABLE IF NOT EXISTS external_posts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    platform_post_id TEXT NOT NULL,
    text TEXT NOT NULL DEFAULT '',
    author_name TEXT,
    author_handle TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    url TEXT,
    media JSONB NOT NULL DEFAULT '[]',
    metadata JSONB NOT NULL DEFAULT '{}',
    imported_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(provider, platform_post_id)
);

CREATE INDEX IF NOT EXISTS idx_external_posts_user_provider ON external_posts(user_id, provider);
CREATE INDEX IF NOT EXISTS idx_external_posts_imported_at ON external_posts(imported_at DESC);
