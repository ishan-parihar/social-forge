-- v24-4: Brand profile table.
--
-- Previously the brand profile (brand name, description, tone of voice,
-- audience, content pillars, keywords, hashtag sets, avoid topics,
-- posting frequency) was stored in localStorage only — not synced across
-- devices, and not read by the AiAssistant. This migration creates a
-- proper table so the brand profile is:
--   (a) synced across devices (single-user, but the user may use multiple
--       browsers/machines),
--   (b) available to the AiAssistant as context for generate/improve/tone,
--   (c) available to the analytics cadence endpoint for goal_per_day.
--
-- Single row per user (enforced by the unique user_id index). Updated
-- via PUT /api/profile.

CREATE TABLE IF NOT EXISTS brand_profiles (
    user_id             UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    brand_name          TEXT,
    description         TEXT,
    tone_of_voice       TEXT,
    audience            TEXT,
    content_pillars     JSONB,   -- [{title, description}]
    keywords            JSONB,   -- ["keyword1", "keyword2"]
    hashtag_sets        JSONB,   -- [{name, tags: ["#tag1", "#tag2"]}]
    avoid_topics        JSONB,   -- ["topic1", "topic2"]
    posting_frequency   TEXT,    -- "daily" | "weekly" | "3x-weekly" | etc.
    posts_per_day_goal  DOUBLE PRECISION,  -- for the cadence widget
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
