-- ─── Social Forge — full schema (SQLite) ───────────────────────
--
-- Squashed from the 37 Postgres migrations 001–037 into one init
-- file. This is the only migration; it runs on boot via
-- `sqlx::migrate!`.
--
-- SQLite dialect notes (all deliberate, no Postgres residue):
--   * UUID            → TEXT. No DB-side default: Rust generates the
--                       value with `Uuid::new_v4()` before the INSERT.
--   * TIMESTAMPTZ     → INTEGER epoch seconds, default `unixepoch()`.
--                       Read/write through `db::types::EpochUtc`.
--   * JSONB / JSON    → TEXT holding JSON. Read/write through
--                       `serde_json::Value` (sqlx maps it to TEXT).
--   * BOOLEAN         → INTEGER 0/1.
--   * ENUM            → TEXT + CHECK constraint. `post_state` carries
--                       the six live values across 001 + 018 + 026.
--   * TEXT[] / UUID[] → a join table (webhook_event_types,
--                       post_set_channels).
--   * BIGSERIAL       → INTEGER PRIMARY KEY AUTOINCREMENT.
--   * Teams schema    → dropped (migration 030, never implemented).
--   * DDL blocks      → the two `DO $$` ALTER TYPE blocks from 018/026
--                       are gone; the CHECK constraint carries the
--                       values instead.

PRAGMA foreign_keys = ON;

-- ── Users ──────────────────────────────────────────────────
-- Single-user MVP, but schema supports multi-user from day 1.

CREATE TABLE users (
    id          TEXT PRIMARY KEY,
    email       TEXT NOT NULL UNIQUE,
    password    TEXT NOT NULL,          -- argon2 hash
    name        TEXT NOT NULL DEFAULT '',
    timezone    INTEGER NOT NULL DEFAULT 0, -- UTC offset in minutes
    streak_since   INTEGER,             -- migration 019
    streak_days    INTEGER NOT NULL DEFAULT 0,
    created_at  INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at  INTEGER NOT NULL DEFAULT (unixepoch())
);

-- ── Social Channel Integrations ───────────────────────────

CREATE TABLE integrations (
    id                    TEXT PRIMARY KEY,
    user_id               TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider_identifier   TEXT NOT NULL,   -- 'x', 'linkedin', 'bluesky', etc.
    provider_name         TEXT NOT NULL DEFAULT '',
    internal_id           TEXT NOT NULL,   -- platform-specific user/page ID
    root_internal_id      TEXT DEFAULT '', -- migration 002: parent account
    access_token          TEXT NOT NULL DEFAULT '',
    refresh_token         TEXT DEFAULT '',
    token_expires_at      INTEGER,
    profile_name          TEXT DEFAULT '',
    profile_picture       TEXT DEFAULT '',
    profile_url           TEXT DEFAULT '',
    disabled              INTEGER NOT NULL DEFAULT 0,
    refresh_needed        INTEGER NOT NULL DEFAULT 0,
    posting_times         TEXT NOT NULL DEFAULT '[{"time":120},{"time":400},{"time":700}]',
    auth_method           TEXT NOT NULL DEFAULT 'oauth', -- migration 013
    created_at            INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at            INTEGER NOT NULL DEFAULT (unixepoch()),
    UNIQUE(user_id, provider_identifier, internal_id)
);

-- ── Posts ─────────────────────────────────────────────────

CREATE TABLE posts (
    id                TEXT PRIMARY KEY,
    user_id           TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    integration_id    TEXT NOT NULL REFERENCES integrations(id) ON DELETE CASCADE,
    -- migrations 001 (draft/queued/published/error) + 018 (publishing)
    -- + 026 (idea)
    state             TEXT NOT NULL DEFAULT 'draft'
                      CHECK (state IN ('idea','draft','queued','publishing','published','error')),
    content           TEXT NOT NULL DEFAULT '',
    title             TEXT DEFAULT '',
    media             TEXT DEFAULT '[]',  -- array of {url, mime_type, alt?}
    settings          TEXT DEFAULT '{}',  -- provider-specific settings
    scheduled_at      INTEGER,            -- when to publish (null = draft/now)
    published_at      INTEGER,            -- actual publish time
    platform_post_id  TEXT DEFAULT '',    -- ID returned by platform
    platform_post_url TEXT DEFAULT '',    -- URL of the post on the platform
    error_message     TEXT DEFAULT '',
    -- migration 006: recurring posts / series
    repeat_interval_days INTEGER,
    repeat_end_date      INTEGER,
    group_id             TEXT,
    -- migration 009: threads
    first_comment     TEXT,
    sequence          INTEGER NOT NULL DEFAULT 0,
    -- migration 018: retry backoff bookkeeping
    retry_count       INTEGER NOT NULL DEFAULT 0,
    next_retry_at     INTEGER,
    -- migration 026: campaign assignment. `campaigns` is declared
    -- later in this file; SQLite resolves FK targets lazily at DML
    -- time, so the forward reference is fine.
    campaign_id       TEXT REFERENCES campaigns(id) ON DELETE SET NULL,
    -- migration 027: repurpose provenance + soft delete
    source_external_post_id TEXT REFERENCES external_posts(id) ON DELETE SET NULL,
    deleted_at        INTEGER,
    -- migration 029: stable idempotency key for provider dedup
    idempotency_key   TEXT NOT NULL,
    -- migration 032: publish state-machine version
    publish_workflow_version INTEGER NOT NULL DEFAULT 2,
    -- migration 034: kanban fields
    kanban_sort_order INTEGER NOT NULL DEFAULT 0,
    kanban_substate   TEXT CHECK (kanban_substate IS NULL OR kanban_substate IN ('ready_to_publish','in_review','blocked')),
    due_date          INTEGER,
    priority          TEXT NOT NULL DEFAULT 'medium'
                      CHECK (priority IN ('low','medium','high','urgent')),
    created_at        INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at        INTEGER NOT NULL DEFAULT (unixepoch())
);

-- ── Media Files ───────────────────────────────────────────

CREATE TABLE media (
    id              TEXT PRIMARY KEY,
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    original_name   TEXT NOT NULL,
    storage_path    TEXT NOT NULL,
    mime_type       TEXT NOT NULL DEFAULT 'application/octet-stream',
    file_size       INTEGER NOT NULL DEFAULT 0,
    width           INTEGER,
    height          INTEGER,
    created_at      INTEGER NOT NULL DEFAULT (unixepoch())
);

-- ── OAuth State Store ─────────────────────────────────────
-- Ephemeral: stores PKCE code_verifier + state during OAuth flow.
-- Cleaned up after callback completes or expires.

CREATE TABLE oauth_states (
    id              TEXT PRIMARY KEY,
    state           TEXT NOT NULL UNIQUE,
    provider        TEXT NOT NULL,
    code_verifier   TEXT NOT NULL,
    redirect_uri    TEXT DEFAULT '',
    created_at      INTEGER NOT NULL DEFAULT (unixepoch()),
    expires_at      INTEGER NOT NULL DEFAULT (unixepoch() + 600)
);

-- ── Tags ──────────────────────────────────────────────────

CREATE TABLE tags (
    id         TEXT PRIMARY KEY,
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    color      TEXT NOT NULL DEFAULT '#6366f1',
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    UNIQUE(user_id, name)
);

CREATE TABLE post_tags (
    post_id TEXT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    tag_id  TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (post_id, tag_id)
);

-- ── Analytics Cache ───────────────────────────────────────

CREATE TABLE analytics_cache (
    id                TEXT PRIMARY KEY,
    user_id           TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider          TEXT NOT NULL,
    platform_post_id  TEXT,           -- NULL for account-level analytics
    data              TEXT NOT NULL DEFAULT '[]',
    cached_at         INTEGER NOT NULL DEFAULT (unixepoch()),
    expires_at        INTEGER NOT NULL DEFAULT (unixepoch() + 3600)
);

-- ── Webhooks ──────────────────────────────────────────────

CREATE TABLE webhooks (
    id            TEXT PRIMARY KEY,
    user_id       TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name          TEXT NOT NULL,
    url           TEXT NOT NULL,
    secret        TEXT,              -- Optional HMAC secret for payload signing
    is_active     INTEGER NOT NULL DEFAULT 1,
    last_triggered_at INTEGER,
    created_at    INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at    INTEGER NOT NULL DEFAULT (unixepoch())
);

-- Was `webhooks.event_types TEXT[]` in Postgres — e.g.
-- {'post.published', 'post.scheduled'}.
CREATE TABLE webhook_event_types (
    webhook_id  TEXT NOT NULL REFERENCES webhooks(id) ON DELETE CASCADE,
    event_type  TEXT NOT NULL,
    PRIMARY KEY (webhook_id, event_type)
);

CREATE TABLE webhook_deliveries (
    id            TEXT PRIMARY KEY,
    webhook_id    TEXT NOT NULL REFERENCES webhooks(id) ON DELETE CASCADE,
    event_type    TEXT NOT NULL,
    payload       TEXT NOT NULL DEFAULT '{}',
    status        TEXT NOT NULL DEFAULT 'pending',  -- pending, delivered, failed
    status_code   INTEGER,
    response_body TEXT,
    attempted_at  INTEGER NOT NULL DEFAULT (unixepoch()),
    delivered_at  INTEGER
);

-- ── Notifications ─────────────────────────────────────────

CREATE TABLE notifications (
    id                TEXT PRIMARY KEY,
    user_id           TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title             TEXT NOT NULL,
    body              TEXT NOT NULL DEFAULT '',
    notification_type TEXT NOT NULL,
    reference_type    TEXT,
    reference_id      TEXT,
    is_read           INTEGER NOT NULL DEFAULT 0,
    created_at        INTEGER NOT NULL DEFAULT (unixepoch())
);

-- ── RSS Autopost Engine ───────────────────────────────────

CREATE TABLE rss_feeds (
    id                TEXT PRIMARY KEY,
    user_id           TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    feed_url          TEXT NOT NULL,
    integration_id    TEXT NOT NULL REFERENCES integrations(id) ON DELETE CASCADE,
    title             TEXT NOT NULL DEFAULT '',
    last_polled_at    INTEGER,
    poll_interval_min INTEGER NOT NULL DEFAULT 30,
    enabled           INTEGER NOT NULL DEFAULT 1,
    use_ai_summary    INTEGER NOT NULL DEFAULT 0,
    created_at        INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at        INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE rss_posts (
    id            TEXT PRIMARY KEY,
    feed_id       TEXT NOT NULL REFERENCES rss_feeds(id) ON DELETE CASCADE,
    post_id       TEXT REFERENCES posts(id) ON DELETE SET NULL,
    guid          TEXT NOT NULL,
    title         TEXT NOT NULL,
    url           TEXT NOT NULL,
    published_at  INTEGER,
    content_hash  TEXT NOT NULL,
    is_imported   INTEGER NOT NULL DEFAULT 0,
    created_at    INTEGER NOT NULL DEFAULT (unixepoch()),
    UNIQUE(feed_id, guid)
);

-- ── Signatures ────────────────────────────────────────────
-- provider = NULL means a global signature, otherwise provider-specific.

CREATE TABLE signatures (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    content     TEXT NOT NULL,
    provider    TEXT,
    is_default  INTEGER NOT NULL DEFAULT 0,  -- migration 028
    created_at  INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at  INTEGER NOT NULL DEFAULT (unixepoch())
);

-- ── API Keys ──────────────────────────────────────────────
-- Keys are SHA-256 hashed at rest; the full key is returned only once
-- at creation.

CREATE TABLE api_keys (
    id            TEXT PRIMARY KEY,
    user_id       TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name          TEXT NOT NULL,
    key_prefix    TEXT NOT NULL,       -- first 8 chars of key for display
    key_hash      TEXT NOT NULL,       -- SHA-256 hash of full key
    last_used_at  INTEGER,
    expires_at    INTEGER,
    is_active     INTEGER NOT NULL DEFAULT 1,
    created_at    INTEGER NOT NULL DEFAULT (unixepoch())
);

-- ── Subscriptions (Stripe) ────────────────────────────────
-- Three plans: free, pro, business. Billed per-user.

CREATE TABLE subscriptions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    stripe_subscription_id TEXT,
    stripe_customer_id TEXT,
    plan   TEXT NOT NULL DEFAULT 'free'
           CHECK (plan IN ('free','pro','business')),
    status TEXT NOT NULL DEFAULT 'active'
           CHECK (status IN ('active','canceled','past_due','incomplete','trialing','unpaid')),
    current_period_start INTEGER,
    current_period_end   INTEGER,
    cancel_at_period_end INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE invoices (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    subscription_id TEXT REFERENCES subscriptions(id) ON DELETE SET NULL,
    stripe_invoice_id TEXT,
    amount INTEGER NOT NULL,
    currency TEXT NOT NULL DEFAULT 'usd',
    status TEXT NOT NULL DEFAULT 'paid',
    invoice_url TEXT,
    paid_at INTEGER,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

-- ── External Posts (imported feed items) ──────────────────

CREATE TABLE external_posts (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    platform_post_id TEXT NOT NULL,
    text TEXT NOT NULL DEFAULT '',
    author_name TEXT,
    author_handle TEXT,
    author_avatar TEXT,             -- migration 015
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    url TEXT,
    media TEXT NOT NULL DEFAULT '[]',
    metadata TEXT NOT NULL DEFAULT '{}',
    imported_at INTEGER NOT NULL DEFAULT (unixepoch()),
    hidden_at INTEGER,              -- migration 025: soft-hide from feed
    saved_at  INTEGER,              -- migration 025: bookmark
    UNIQUE(provider, platform_post_id)
);

-- ── Post Engagement Metrics ───────────────────────────────
-- Unified metrics for external posts across all platforms.
-- `impressions` was added by migration 037 and aliases `views`.

CREATE TABLE post_engagement (
    id TEXT PRIMARY KEY,
    post_id TEXT NOT NULL REFERENCES external_posts(id) ON DELETE CASCADE,

    -- Core metrics (all platforms normalize to these)
    likes    INTEGER NOT NULL DEFAULT 0,
    comments INTEGER NOT NULL DEFAULT 0,
    shares   INTEGER NOT NULL DEFAULT 0,
    views    INTEGER NOT NULL DEFAULT 0,

    -- Platform-specific engagement
    saves   INTEGER NOT NULL DEFAULT 0,   -- IG saves, X bookmarks
    quotes  INTEGER NOT NULL DEFAULT 0,   -- X quotes, Bluesky quotes
    reposts INTEGER NOT NULL DEFAULT 0,   -- X retweets, Mastodon reblogs
    replies INTEGER NOT NULL DEFAULT 0,   -- X replies, Reddit comments

    -- Reaction breakdown (Facebook: {"like": 42, "love": 7, ...})
    reactions TEXT NOT NULL DEFAULT '{}',

    -- Reddit-specific metrics
    upvotes      INTEGER NOT NULL DEFAULT 0,
    downvotes    INTEGER NOT NULL DEFAULT 0,
    upvote_ratio REAL,
    awards       INTEGER NOT NULL DEFAULT 0,

    -- Raw platform data for extensibility
    raw TEXT NOT NULL DEFAULT '{}',

    -- Timestamps
    fetched_at INTEGER NOT NULL DEFAULT (unixepoch()),
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    impressions INTEGER NOT NULL DEFAULT 0,

    UNIQUE(post_id)
);

-- ── Automation Rules ──────────────────────────────────────

CREATE TABLE automation_rules (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    integration_id TEXT NOT NULL REFERENCES integrations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    trigger_type TEXT NOT NULL CHECK (trigger_type IN ('comment','dm','mention','follow')),
    trigger_filter TEXT DEFAULT '{}',
    response_template TEXT NOT NULL,
    response_type TEXT NOT NULL CHECK (response_type IN ('ai_generated','template','fixed')),
    ai_model TEXT,
    is_active INTEGER DEFAULT 1,
    cooldown_minutes INTEGER DEFAULT 0,
    max_responses_per_hour INTEGER DEFAULT 10,
    created_at INTEGER DEFAULT (unixepoch()),
    updated_at INTEGER DEFAULT (unixepoch())
);

CREATE TABLE automation_logs (
    id TEXT PRIMARY KEY,
    rule_id TEXT NOT NULL REFERENCES automation_rules(id) ON DELETE CASCADE,
    trigger_id TEXT NOT NULL,
    trigger_type TEXT NOT NULL,
    response TEXT,
    status TEXT NOT NULL CHECK (status IN ('sent','failed','skipped_cooldown','skipped_limit')),
    error_message TEXT,
    created_at INTEGER DEFAULT (unixepoch())
);

-- ── Publish Attempt Audit Trail ───────────────────────────
-- One row per provider.publish() call, so the operator can see
-- "attempted at T1 with 429, at T2 with 429, at T3 with success".

CREATE TABLE publish_attempts (
    id              TEXT PRIMARY KEY,
    post_id         TEXT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    attempt_number  INTEGER NOT NULL,
    status          TEXT NOT NULL CHECK (status IN ('success','failed')),
    error_message   TEXT,
    started_at      INTEGER NOT NULL DEFAULT (unixepoch()),
    finished_at     INTEGER,
    created_at      INTEGER NOT NULL DEFAULT (unixepoch())
);

-- ── Post Publish Outbox ───────────────────────────────────
-- Best-effort durability log: after publish() succeeds the scheduler
-- writes here as well as to `posts`; the drain loop reconciles any
-- `posts` write that failed.

CREATE TABLE publish_outbox (
    id              TEXT PRIMARY KEY,
    post_id         TEXT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    idempotency_key TEXT NOT NULL,
    platform_post_id   TEXT,
    platform_post_url  TEXT,
    published_at       INTEGER,
    error_message      TEXT,
    attempts          INTEGER NOT NULL DEFAULT 0,
    next_attempt_at   INTEGER NOT NULL DEFAULT (unixepoch()),
    created_at        INTEGER NOT NULL DEFAULT (unixepoch()),
    completed_at      INTEGER
);

-- ── Post Sets / Templates ─────────────────────────────────
-- The full post payload (content, channels, media, settings) so a set
-- can be loaded into the composer in one click.

CREATE TABLE post_sets (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    description TEXT,
    content     TEXT NOT NULL DEFAULT '{}',
    created_at  INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at  INTEGER NOT NULL DEFAULT (unixepoch())
);

-- Was `post_sets.channel_ids UUID[]` in Postgres.
CREATE TABLE post_set_channels (
    post_set_id  TEXT NOT NULL REFERENCES post_sets(id) ON DELETE CASCADE,
    channel_id   TEXT NOT NULL,
    PRIMARY KEY (post_set_id, channel_id)
);

-- ── Post Plugs (outbound post-publish automations) ────────

CREATE TABLE post_plugs (
    id              TEXT PRIMARY KEY,
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    post_id         TEXT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    integration_id  TEXT NOT NULL REFERENCES integrations(id) ON DELETE CASCADE,
    plug_type       TEXT NOT NULL CHECK (plug_type IN ('auto_repost_after_likes','cross_post_from_secondary')),
    config          TEXT NOT NULL DEFAULT '{}',
    runs_so_far     INTEGER NOT NULL DEFAULT 0,
    max_runs        INTEGER NOT NULL DEFAULT 1,
    next_run_at     INTEGER NOT NULL DEFAULT (unixepoch()),
    fired_at        INTEGER,
    completed       INTEGER NOT NULL DEFAULT 0,
    created_at      INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at      INTEGER NOT NULL DEFAULT (unixepoch())
);

-- ── Resolved Comments ─────────────────────────────────────
-- Platform comment IDs are globally unique, so no provider column.

CREATE TABLE resolved_comments (
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    comment_id TEXT NOT NULL,
    resolved_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (user_id, comment_id)
);

-- ── Cached Comments ───────────────────────────────────────
-- Comments are pulled by the background feed refresher so the comments
-- list endpoint reads from cache instead of 50 live API calls.

CREATE TABLE cached_comments (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id       TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    comment_id    TEXT NOT NULL,
    post_id       TEXT NOT NULL REFERENCES external_posts(id) ON DELETE CASCADE,
    provider      TEXT NOT NULL,
    author_name   TEXT,
    author_handle TEXT,
    author_avatar TEXT,
    text          TEXT NOT NULL,
    created_at    INTEGER NOT NULL,
    fetched_at    INTEGER NOT NULL DEFAULT (unixepoch()),
    UNIQUE (user_id, comment_id)
);

-- ── Campaigns ─────────────────────────────────────────────
-- Kanban columns group by posts.state, optionally filtered by campaign.

CREATE TABLE campaigns (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    description TEXT,
    color       TEXT NOT NULL DEFAULT '#6366f1',
    start_date  TEXT,
    end_date    TEXT,
    goal        TEXT,
    status      TEXT NOT NULL DEFAULT 'active'
                CHECK (status IN ('active','paused','archived','completed')),
    progress_metric TEXT,            -- posts | engagement | reach | followers | custom
    progress_target INTEGER,
    audience_persona TEXT,           -- {age_range, location, interests, pain_points}
    content_pillars   TEXT,          -- [{title, description, tags}]
    budget_cents      INTEGER,
    kpi_targets       TEXT,          -- {min_engagement_rate, min_reach, target_clicks}
    deleted_at    INTEGER,
    sort_order    INTEGER NOT NULL DEFAULT 0,
    created_at    INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at    INTEGER NOT NULL DEFAULT (unixepoch())
);

-- ── Events Log ────────────────────────────────────────────
-- Append-only recent-activity feed behind the dashboard widget.

CREATE TABLE events_log (
    id         TEXT PRIMARY KEY,
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    payload    TEXT NOT NULL DEFAULT '{}',
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

-- ── Brand Profiles ────────────────────────────────────────
-- Single row per user; synced across devices and read as AiAssistant
-- context.

CREATE TABLE brand_profiles (
    user_id             TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    brand_name          TEXT,
    description         TEXT,
    tone_of_voice       TEXT,
    audience            TEXT,
    content_pillars     TEXT,   -- [{title, description}]
    keywords            TEXT,   -- ["keyword1", "keyword2"]
    hashtag_sets        TEXT,   -- [{name, tags: ["#tag1", "#tag2"]}]
    avoid_topics        TEXT,   -- ["topic1", "topic2"]
    posting_frequency   TEXT,   -- daily | weekly | 3x-weekly | ...
    posts_per_day_goal  REAL,   -- for the cadence widget
    created_at          INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at          INTEGER NOT NULL DEFAULT (unixepoch())
);

-- ── Indexes ───────────────────────────────────────────────

CREATE INDEX idx_posts_user_state      ON posts(user_id, state);
CREATE INDEX idx_posts_scheduled       ON posts(scheduled_at) WHERE state = 'queued';
CREATE INDEX idx_posts_group_id        ON posts(group_id);
CREATE INDEX idx_posts_not_deleted     ON posts(user_id, state) WHERE deleted_at IS NULL;
CREATE INDEX idx_posts_campaign        ON posts(campaign_id) WHERE campaign_id IS NOT NULL;
CREATE INDEX idx_posts_source_external_post_id
                                         ON posts(source_external_post_id)
                                         WHERE source_external_post_id IS NOT NULL;
CREATE INDEX idx_posts_idempotency_key ON posts(idempotency_key);
CREATE INDEX idx_posts_kanban_order    ON posts(user_id, state, kanban_sort_order)
                                         WHERE deleted_at IS NULL;
CREATE INDEX idx_posts_due_date        ON posts(user_id, due_date)
                                         WHERE due_date IS NOT NULL AND deleted_at IS NULL;

CREATE INDEX idx_integrations_user     ON integrations(user_id);
CREATE INDEX idx_integrations_provider ON integrations(provider_identifier);

CREATE INDEX idx_oauth_states_state    ON oauth_states(state);
CREATE INDEX idx_oauth_states_expires  ON oauth_states(expires_at);

CREATE INDEX idx_media_user            ON media(user_id);

CREATE INDEX idx_analytics_cache_user_provider ON analytics_cache(user_id, provider);
CREATE INDEX idx_analytics_cache_expires ON analytics_cache(expires_at);

CREATE INDEX idx_webhooks_user         ON webhooks(user_id);
CREATE INDEX idx_webhook_deliveries_webhook ON webhook_deliveries(webhook_id);
CREATE INDEX idx_webhook_deliveries_status  ON webhook_deliveries(status);

CREATE INDEX idx_notifications_user_read    ON notifications(user_id, is_read);
CREATE INDEX idx_notifications_user_created ON notifications(user_id, created_at DESC);

CREATE INDEX idx_rss_feeds_user_id    ON rss_feeds(user_id);
CREATE INDEX idx_rss_feeds_enabled    ON rss_feeds(enabled) WHERE enabled = 1;
CREATE INDEX idx_rss_posts_feed_id    ON rss_posts(feed_id);

CREATE INDEX idx_signatures_user_id   ON signatures(user_id);
CREATE UNIQUE INDEX idx_signatures_default_per_provider
                                     ON signatures(user_id, provider) WHERE is_default = 1;

CREATE INDEX idx_api_keys_user        ON api_keys(user_id);

CREATE UNIQUE INDEX idx_subscriptions_user_id ON subscriptions(user_id);
CREATE INDEX idx_invoices_user_id          ON invoices(user_id);
CREATE INDEX idx_invoices_subscription_id  ON invoices(subscription_id);

CREATE INDEX idx_external_posts_user_provider ON external_posts(user_id, provider);
CREATE INDEX idx_external_posts_imported_at  ON external_posts(imported_at DESC);

CREATE INDEX idx_post_engagement_fetched  ON post_engagement(fetched_at DESC);
CREATE INDEX idx_post_engagement_post_id  ON post_engagement(post_id);

CREATE INDEX idx_automation_rules_user        ON automation_rules(user_id);
CREATE INDEX idx_automation_rules_integration ON automation_rules(integration_id);
CREATE INDEX idx_automation_rules_active      ON automation_rules(is_active) WHERE is_active = 1;
CREATE INDEX idx_automation_logs_rule         ON automation_logs(rule_id);
CREATE INDEX idx_automation_logs_status       ON automation_logs(status);
CREATE INDEX idx_automation_logs_created      ON automation_logs(created_at);

CREATE INDEX idx_publish_attempts_post_id     ON publish_attempts(post_id, attempt_number);
CREATE INDEX idx_publish_attempts_status_started
                                              ON publish_attempts(status, started_at DESC);

CREATE INDEX idx_publish_outbox_pending      ON publish_outbox(next_attempt_at)
                                              WHERE completed_at IS NULL;
CREATE INDEX idx_publish_outbox_post_id      ON publish_outbox(post_id);
CREATE INDEX idx_publish_outbox_idempotency  ON publish_outbox(idempotency_key)
                                              WHERE platform_post_id IS NOT NULL;

CREATE INDEX idx_post_sets_user_id     ON post_sets(user_id);
CREATE INDEX idx_post_plugs_due        ON post_plugs(next_run_at) WHERE completed = 0;
CREATE INDEX idx_post_plugs_post       ON post_plugs(post_id);

CREATE INDEX idx_resolved_comments_user      ON resolved_comments(user_id);
CREATE INDEX idx_cached_comments_user_created ON cached_comments(user_id, created_at DESC);
CREATE INDEX idx_cached_comments_user_provider ON cached_comments(user_id, provider);
CREATE INDEX idx_cached_comments_post_id    ON cached_comments(post_id);

CREATE INDEX idx_campaigns_user          ON campaigns(user_id);
CREATE INDEX idx_campaigns_status        ON campaigns(user_id, status) WHERE deleted_at IS NULL;
CREATE INDEX idx_campaigns_not_deleted   ON campaigns(user_id) WHERE deleted_at IS NULL;

CREATE INDEX idx_events_log_user_created ON events_log(user_id, created_at DESC);
CREATE INDEX idx_events_log_created      ON events_log(created_at);