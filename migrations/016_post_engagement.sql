-- Post Engagement: unified engagement metrics for external posts across all platforms
CREATE TABLE IF NOT EXISTS post_engagement (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id UUID NOT NULL REFERENCES external_posts(id) ON DELETE CASCADE,

    -- Core metrics (all platforms normalize to these)
    likes INTEGER NOT NULL DEFAULT 0,
    comments INTEGER NOT NULL DEFAULT 0,
    shares INTEGER NOT NULL DEFAULT 0,
    views INTEGER NOT NULL DEFAULT 0,

    -- Platform-specific engagement
    saves INTEGER NOT NULL DEFAULT 0,         -- Instagram saves, X/Twitter bookmarks
    quotes INTEGER NOT NULL DEFAULT 0,         -- X/Twitter quotes, Bluesky quotes
    reposts INTEGER NOT NULL DEFAULT 0,        -- X/Twitter retweets, Bluesky reposts, Mastodon reblogs
    replies INTEGER NOT NULL DEFAULT 0,        -- X/Twitter replies, Reddit comments

    -- Reaction breakdown (Facebook: {"like": 42, "love": 7, "haha": 3, "wow": 2, "sad": 1, "angry": 1})
    reactions JSONB NOT NULL DEFAULT '{}',

    -- Reddit-specific metrics
    upvotes INTEGER NOT NULL DEFAULT 0,
    downvotes INTEGER NOT NULL DEFAULT 0,
    upvote_ratio REAL,
    awards INTEGER NOT NULL DEFAULT 0,

    -- Raw platform data for extensibility
    raw JSONB NOT NULL DEFAULT '{}',

    -- Timestamps
    fetched_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(post_id)
);

CREATE INDEX IF NOT EXISTS idx_post_engagement_fetched ON post_engagement(fetched_at DESC);
CREATE INDEX IF NOT EXISTS idx_post_engagement_post_id ON post_engagement(post_id);

COMMENT ON TABLE post_engagement IS 'Unified engagement metrics for external social media posts';
COMMENT ON COLUMN post_engagement.likes IS 'Likes/favorites across all platforms';
COMMENT ON COLUMN post_engagement.comments IS 'Comment count across all platforms';
COMMENT ON COLUMN post_engagement.shares IS 'Share/repost count across all platforms';
COMMENT ON COLUMN post_engagement.views IS 'View/impression count across all platforms';
COMMENT ON COLUMN post_engagement.saves IS 'Bookmarks/saves (Instagram saves, X bookmarks)';
COMMENT ON COLUMN post_engagement.quotes IS 'Quote posts (X quotes, Bluesky quotes)';
COMMENT ON COLUMN post_engagement.reposts IS 'Reposts/retweets/reblogs';
COMMENT ON COLUMN post_engagement.replies IS 'Reply count (X replies, Reddit comments)';
COMMENT ON COLUMN post_engagement.reactions IS 'Reaction breakdown (Facebook: {"like": 42, "love": 7})';
COMMENT ON COLUMN post_engagement.upvotes IS 'Reddit upvotes (positive votes)';
COMMENT ON COLUMN post_engagement.downvotes IS 'Reddit downvotes (negative votes)';
COMMENT ON COLUMN post_engagement.upvote_ratio IS 'Reddit upvote ratio (0.0 to 1.0)';
COMMENT ON COLUMN post_engagement.awards IS 'Reddit awards count';
COMMENT ON COLUMN post_engagement.raw IS 'Full raw platform response for extensibility';
