// ─── Social Provider Trait ─────────────────────────────────────
// Abstract interface for all social media platforms.
// Each platform implements this trait for OAuth + publishing.

pub mod bluesky;
pub mod browser_cookies;
pub mod calendar;
pub mod common;
pub mod devto;
pub mod discord;
pub mod farcaster;
pub mod drive;
pub mod facebook;
pub mod github;
pub mod gmail;
pub mod google;
pub mod hashnode;
pub mod instagram;
pub mod instagram_standalone;
pub mod kick;
pub mod lemmy;
pub mod linkedin;
pub mod linkedin_page;
pub mod mastodon;
pub mod medium;
pub mod pinterest;
pub mod reddit;
pub mod reddit_cookies;
pub mod registry;
pub mod skool;
pub mod slack;
pub mod telegram_bot;
pub mod telegram_user;
pub mod threads;
pub mod tiktok;
pub mod vk;
pub mod google_my_business;
pub mod whatsapp;
pub mod whop;
pub mod wordpress;
pub mod x;
pub mod x_cookies;
pub mod youtube;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;


// ── Common Types ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AuthUrlResponse {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AuthToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u32>,
    pub provider_user_id: String,
    pub name: String,
    pub username: String,
    pub picture: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PostContent {
    pub content: String,
    pub media: Vec<MediaAttachment>,
    pub settings: serde_json::Value,
    /// Platform-specific post ID that this post is replying to (for threads).
    /// When the scheduler publishes a multi-part thread (posts sharing a
    /// `group_id`), it sets this to the previous part's `platform_post_id`
    /// so providers that support threading (X, Bluesky, Mastodon, Threads)
    /// can link them. Providers that don't support threading ignore this.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_reply_to: Option<String>,
    /// Phase v22: idempotency key for the publish attempt. Stable across
    /// retries (same post = same key). Providers that support idempotency
    /// (X, LinkedIn, etc.) deduplicate on this key — if the same key is
    /// sent twice, the second request is a no-op returning the original
    /// post's platform_post_id, preventing duplicate posts after
    /// crash-recovery retries.
    ///
    /// Providers that don't support idempotency simply ignore this field.
    /// The key is a UUID v4 string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    /// v24-5: delay (in minutes) before publishing this row of a thread.
    /// When the scheduler publishes a multi-part thread, it sleeps for
    /// `delay_minutes` minutes between rows. This allows the user to
    /// space out thread parts (e.g. "post part 1, wait 30min, post
    /// part 2"). Only applies to posts with sequence > 1 in a thread
    /// group. A value of 0 or None means no delay.
    ///
    /// Postiz uses a similar mechanism (per-row `delay` field in the
    /// composer store, slept in the Temporal workflow).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay_minutes: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MediaAttachment {
    pub url: String,
    pub mime_type: String,
    pub alt: Option<String>,
    /// Optional poster/thumbnail URL for videos
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poster_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PublishResult {
    pub platform_post_id: String,
    pub platform_post_url: Option<String>,
    pub status: String,
}

// ── Additional Common Types ─────────────────────────────────

/// Editor type for content creation
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub enum EditorType {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "markdown")]
    Markdown,
    #[serde(rename = "html")]
    Html,
}

/// Page/channel info for multi-step auth (isBetweenSteps providers)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PageInfo {
    pub id: String,
    pub name: String,
    pub access_token: Option<String>,
    pub picture: Option<String>,
    pub username: Option<String>,
}

/// Discoverable posting target (channel, group, subreddit, peer, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TargetInfo {
    pub id: String,
    pub name: String,
    pub target_type: String,
    pub picture: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Result of reconnecting after re-authentication
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReconnectResult {
    pub id: String,
    pub name: String,
    pub access_token: String,
    pub picture: Option<String>,
    pub username: Option<String>,
}

/// Analytics data for dashboards
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalyticsData {
    pub label: String,
    pub data: Vec<AnalyticsDataPoint>,
    pub percentage_change: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalyticsDataPoint {
    pub total: String,
    pub date: String,
}

/// @mention autocomplete result
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MentionResult {
    pub id: String,
    pub label: String,
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub do_not_cache: Option<bool>,
}

/// Standardized engagement data for any social media post.
/// All platforms normalize their metrics into this unified schema.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EngagementData {
    /// Core metrics (all platforms)
    pub likes: i32,
    pub comments: i32,
    pub shares: i32,
    pub views: i32,
    /// Platform-specific
    pub saves: i32,
    pub quotes: i32,
    pub reposts: i32,
    pub replies: i32,
    /// Reaction breakdown (e.g., Facebook: {"like": 42, "love": 7, "haha": 3})
    pub reactions: Option<serde_json::Value>,
    /// Reddit-specific
    pub upvotes: i32,
    pub downvotes: i32,
    pub upvote_ratio: Option<f32>,
    pub awards: i32,
    /// Raw platform response for extensibility
    pub raw: Option<serde_json::Value>,
}

/// A single comment from a social media post
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CommentData {
    pub id: String,
    pub author_name: Option<String>,
    pub author_avatar: Option<String>,
    pub text: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub like_count: i32,
    pub replies: Vec<CommentData>,
}

/// A DM conversation
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DmConversation {
    pub id: String,
    pub participant: String,
    pub participant_name: Option<String>,
    pub participant_avatar: Option<String>,
    pub last_message: Option<String>,
    pub last_message_at: Option<chrono::DateTime<chrono::Utc>>,
    pub unread_count: u32,
}

/// A single DM message
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DmMessage {
    pub id: String,
    pub conversation_id: String,
    pub sender: String,
    pub sender_name: Option<String>,
    pub content: String,
    pub media: Vec<MediaAttachment>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub read: bool,
}

/// External post data for import (CLI command)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExternalPostData {
    pub platform_post_id: String,
    pub text: String,
    pub author_name: Option<String>,
    pub author_handle: Option<String>,
    pub author_avatar: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub url: Option<String>,
    pub media: Vec<MediaAttachment>,
    pub metadata: Option<serde_json::Value>,
}

/// Extra fields for provider OAuth config
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CustomField {
    pub key: String,
    pub label: String,
    pub default_value: Option<String>,
    pub validation: String,
    pub field_type: CustomFieldType,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub enum CustomFieldType {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "password")]
    Password,
}

// ── Trait ───────────────────────────────────────────────────

#[async_trait]
pub trait SocialProvider: Send + Sync {
    /// Unique provider identifier (e.g., "x", "linkedin", "bluesky")
    fn identifier(&self) -> &'static str;

    /// Human-readable provider name
    fn name(&self) -> &'static str;

    /// OAuth2 scopes required for this provider
    fn scopes(&self) -> Vec<String>;

    /// Max content length (characters)
    fn max_content_length(&self) -> usize;

    /// Tooltip shown in UI
    fn tooltip(&self) -> Option<&'static str> { None }

    /// Editor type for content creation
    fn editor_type(&self) -> EditorType { EditorType::Normal }

    /// Does this provider have multi-step auth (page selection after OAuth)?
    fn is_between_steps(&self) -> bool { false }

    /// Does this provider use a Chrome extension for auth?
    fn is_chrome_extension(&self) -> bool { false }

    /// Does this provider need proactive cron-based token refresh?
    fn needs_cron_refresh(&self) -> bool { false }

    /// Should we wait for refresh to complete before publishing?
    fn refresh_wait(&self) -> bool { false }

    /// Is this a one-time token provider?
    fn one_time_token(&self) -> bool { false }

    /// Custom fields for OAuth config
    async fn custom_fields(&self) -> Vec<CustomField> { vec![] }

    /// Extension cookies needed (for Chrome extension providers like Skool)
    fn extension_cookies(&self) -> Vec<(&'static str, &'static str)> { vec![] }

    /// Generate the OAuth authorization URL
    /// `code_verifier` is the PKCE code verifier (S256 challenge will be derived from it).
    async fn generate_auth_url(
        &self,
        state: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthUrlResponse, ProviderError>;

    /// Check if this provider uses OAuth (vs direct API key / app password).
    fn uses_oauth(&self) -> bool { true }

    /// Exchange authorization code for access token
    async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthToken, ProviderError>;

    /// Refresh an expired access token
    async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<AuthToken, ProviderError>;

    /// Publish content to the social platform
    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError>;

    /// Post a comment/reply to an existing post
    async fn comment(
        &self,
        _access_token: &str,
        _post_id: &str,
        _last_comment_id: Option<&str>,
        _post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        Err(ProviderError::Api("Comments not supported by this provider".into()))
    }

    /// Get analytics for a time range (dashboard)
    async fn analytics(
        &self,
        _access_token: &str,
        _internal_id: &str,
        _days: u32,
    ) -> Result<Vec<AnalyticsData>, ProviderError> {
        Ok(vec![])
    }

    /// Get per-post analytics
    async fn post_analytics(
        &self,
        _access_token: &str,
        _platform_post_id: &str,
    ) -> Result<Vec<AnalyticsData>, ProviderError> {
        Ok(vec![])
    }

    /// List manageable pages/channels (for isBetweenSteps providers)
    async fn pages(
        &self,
        _access_token: &str,
    ) -> Result<Vec<PageInfo>, ProviderError> {
        Ok(vec![])
    }

    /// Discover available posting targets (channels, groups, subreddits, peers, etc.)
    /// Returns empty vec for providers where target = integration (e.g., X, LinkedIn personal)
    async fn targets(
        &self,
        _access_token: &str,
    ) -> Result<Vec<TargetInfo>, ProviderError> {
        Ok(vec![])
    }

    /// Fetch page information by ID (for reConnect)
    async fn fetch_page_info(
        &self,
        access_token: &str,
        page_id: &str,
    ) -> Result<PageInfo, ProviderError>;

    /// Reconnect/re-bind after re-authentication
    async fn reconnect(
        &self,
        access_token: &str,
        _internal_id: &str,
        page_id: &str,
    ) -> Result<ReconnectResult, ProviderError> {
        let info = self.fetch_page_info(access_token, page_id).await?;
        Ok(ReconnectResult {
            id: info.id,
            name: info.name,
            access_token: info.access_token.unwrap_or_default(),
            picture: info.picture,
            username: info.username,
        })
    }

    /// Search for @mentions
    async fn search_mention(
        &self,
        _access_token: &str,
        _query: &str,
    ) -> Result<Vec<MentionResult>, ProviderError> {
        Ok(vec![])
    }

    /// Format an @mention string for this provider
    fn format_mention(&self, id_or_handle: &str, _name: &str) -> String {
        format!("@{}", id_or_handle)
    }

    /// Map provider API error body/status to user-friendly message
    fn map_error(&self, _body: &str, _status: u16) -> Option<String> { None }

    /// Import recent posts from this platform (for the External Post Import CLI).
    async fn get_recent_posts(
        &self,
        _access_token: &str,
        _internal_id: &str,
        _limit: u32,
    ) -> Result<Vec<ExternalPostData>, ProviderError> {
        Ok(vec![])
    }

    /// Fetch engagement data for a post (likes, comments, shares, etc.)
    /// Returns raw JSON to be parsed into EngagementData by the caller.
    async fn get_post_engagement(
        &self,
        _access_token: &str,
        _platform_post_id: &str,
    ) -> Result<Option<serde_json::Value>, ProviderError> {
        Ok(None)
    }

    /// Fetch comments for a post.
    /// Returns a flat or threaded list of comments from the platform.
    async fn get_post_comments(
        &self,
        _access_token: &str,
        _platform_post_id: &str,
    ) -> Result<Vec<CommentData>, ProviderError> {
        Ok(vec![])
    }

    /// Fetch normalized engagement data for a post.
    /// This is the canonical method used by the engagement sync engine.
    /// Returns None if the post has no engagement data or the API doesn't support it.
    /// Default implementation calls get_post_engagement() and parses it.
    async fn fetch_engagement(
        &self,
        access_token: &str,
        platform_post_id: &str,
    ) -> Result<Option<EngagementData>, ProviderError> {
        let raw = self.get_post_engagement(access_token, platform_post_id).await?;
        match raw {
            Some(value) => Ok(Some(parse_engagement_data(self.identifier(), value))),
            None => Ok(None),
        }
    }

    /// Validate content against platform-specific limits before publishing.
    fn validate_post(&self, post: &PostContent) -> Result<(), String> {
        if post.content.len() > self.max_content_length() {
            return Err(format!(
                "Content too long ({} chars). Maximum is {} chars for {}.",
                post.content.len(),
                self.max_content_length(),
                self.name()
            ));
        }
        Ok(())
    }

    /// Validate media attachments against platform-specific limits.
    /// Checks image/video counts, file sizes, and dimensions where applicable.
    /// Default implementation accepts all media (no restrictions).
    fn validate_media(&self, _post: &PostContent) -> Result<(), String> {
        Ok(())
    }

    /// Resolve a media attachment URL to a publicly accessible URL.
    /// Platforms like Instagram, Facebook, and Threads require publicly accessible URLs
    /// because they fetch media server-side. By default, returns the URL unchanged.
    /// Override in providers that need URL resolution (e.g., Meta Graph API providers).
    fn resolve_media_url(&self, attachment: &MediaAttachment, _app_url: &str) -> MediaAttachment {
        attachment.clone()
    }

    // ── DM Methods ───────────────────────────────────────────

    /// Send a direct message to a user
    async fn send_dm(
        &self,
        _access_token: &str,
        _recipient: &str,
        _content: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        Err(ProviderError::Api("DMs not supported by this provider".into()))
    }

    /// Get list of DM conversations
    async fn get_dm_conversations(
        &self,
        _access_token: &str,
        _limit: u32,
    ) -> Result<Vec<DmConversation>, ProviderError> {
        Ok(vec![])
    }

    /// Get messages in a DM conversation
    async fn get_dm_messages(
        &self,
        _access_token: &str,
        _conversation_id: &str,
        _limit: u32,
    ) -> Result<Vec<DmMessage>, ProviderError> {
        Ok(vec![])
    }

    // ── Comment Enhancement Methods ──────────────────────────

    /// Reply to a specific comment
    async fn reply_to_comment(
        &self,
        _access_token: &str,
        _comment_id: &str,
        _content: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        Err(ProviderError::Api("Comment replies not supported by this provider".into()))
    }

    /// Get a specific comment by ID
    async fn get_comment(
        &self,
        _access_token: &str,
        _comment_id: &str,
    ) -> Result<Option<CommentData>, ProviderError> {
        Ok(None)
    }

    /// Delete a comment
    async fn delete_comment(
        &self,
        _access_token: &str,
        _comment_id: &str,
    ) -> Result<(), ProviderError> {
        Err(ProviderError::Api("Comment deletion not supported by this provider".into()))
    }
}

/// Platform-specific media validation limits.
pub fn validate_media_limits(identifier: &str, post: &PostContent) -> Result<(), String> {
    let images: Vec<_> = post.media.iter().filter(|m| m.mime_type.starts_with("image/")).collect();
    let videos: Vec<_> = post.media.iter().filter(|m| m.mime_type.starts_with("video/")).collect();

    match identifier {
        "instagram" | "instagram_standalone" | "instagram-standalone" => {
            if images.len() > 10 {
                return Err(format!("Instagram allows max 10 images, got {}", images.len()));
            }
            if post.media.len() > 10 {
                return Err(format!("Instagram allows max 10 media items, got {}", post.media.len()));
            }
        }
        "x" => {
            if images.len() > 4 {
                return Err(format!("X/Twitter allows max 4 images, got {}", images.len()));
            }
            if post.media.len() > 4 {
                return Err(format!("X/Twitter allows max 4 media items, got {}", post.media.len()));
            }
        }
        "linkedin" | "linkedin_page" | "linkedin-page" => {
            if images.len() > 20 {
                return Err(format!("LinkedIn allows max 20 images, got {}", images.len()));
            }
            if post.media.len() > 20 {
                return Err(format!("LinkedIn allows max 20 media items, got {}", post.media.len()));
            }
        }
        "reddit" => {
            if images.len() > 20 {
                return Err(format!("Reddit allows max 20 images, got {}", images.len()));
            }
            if !videos.is_empty() {
                return Err("Reddit posts do not support video. Use a video link instead.".into());
            }
        }
        "facebook" => {
            if images.len() > 10 {
                return Err(format!("Facebook allows max 10 images, got {}", images.len()));
            }
            if post.media.len() > 10 {
                return Err(format!("Facebook allows max 10 media items, got {}", post.media.len()));
            }
        }
        "youtube" => {
            if !images.is_empty() && !videos.is_empty() {
                return Err("YouTube posts require video only, not images.".into());
            }
            if videos.is_empty() && images.is_empty() {
                return Err("YouTube posts require a video attachment.".into());
            }
        }
        "threads" => {
            if images.len() > 10 {
                return Err(format!("Threads allows max 10 images, got {}", images.len()));
            }
            if post.media.len() > 10 {
                return Err(format!("Threads allows max 10 media items, got {}", post.media.len()));
            }
        }
        "bluesky" => {
            if images.len() > 4 {
                return Err(format!("Bluesky allows max 4 images, got {}", images.len()));
            }
            if post.media.len() > 4 {
                return Err(format!("Bluesky allows max 4 media items, got {}", post.media.len()));
            }
        }
        "mastodon" => {
            if post.media.len() > 4 {
                return Err(format!("Mastodon allows max 4 media items, got {}", post.media.len()));
            }
        }
        _ => {}
    }
    Ok(())
}

// ── Error Types ─────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("API error: {0}")]
    Api(String),
    #[error("Token expired")]
    TokenExpired,
    #[error("Rate limited: {0}")]
    RateLimited(String),
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Auth error: {0}")]
    Auth(String),
}

// ── Engagement Data Parser ────────────────────────────────────

/// Parse a provider's raw engagement JSON into a normalized EngagementData struct.
/// Each provider returns a different JSON shape from get_post_engagement().
/// This function handles all known provider-specific formats.
pub fn parse_engagement_data(provider: &str, raw: serde_json::Value) -> EngagementData {
    let mut e = EngagementData {
        likes: 0, comments: 0, shares: 0, views: 0,
        saves: 0, quotes: 0, reposts: 0, replies: 0,
        reactions: None,
        upvotes: 0, downvotes: 0, upvote_ratio: None, awards: 0,
        raw: Some(raw.clone()),
    };

    match provider {
        // X/Twitter: { "public_metrics": { "like_count": 42, "retweet_count": 8, "reply_count": 3, "quote_count": 1, "impression_count": 1200, "bookmark_count": 5 } }
        "x" => {
            let pm = raw.get("public_metrics").or_else(|| raw.as_object().map(|_| &raw));
            if let Some(m) = pm {
                e.likes = m.get("like_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                e.replies = m.get("reply_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                e.reposts = m.get("retweet_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                e.quotes = m.get("quote_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                e.views = m.get("impression_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                e.saves = m.get("bookmark_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            }
        }

        // Reddit: { "score": 42, "num_comments": 12, "upvote_ratio": 0.95, "ups": 45, "downs": 3, "total_awards_received": 2 }
        "reddit" => {
            e.upvotes = raw.get("ups").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.downvotes = raw.get("downs").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.upvote_ratio = raw.get("upvote_ratio").and_then(|v| v.as_f64()).map(|v| v as f32);
            e.comments = raw.get("num_comments").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.awards = raw.get("total_awards_received").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            // Score as likes (positive engagement indicator)
            e.likes = raw.get("score").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        }

        // Bluesky: { "likeCount": 42, "repostCount": 8, "replyCount": 3, "quoteCount": 1 }
        "bluesky" => {
            e.likes = raw.get("likeCount").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.reposts = raw.get("repostCount").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.replies = raw.get("replyCount").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.quotes = raw.get("quoteCount").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        }

        // Instagram: { "like_count": 42, "comments_count": 12 } (from Graph API)
        "instagram" | "instagram_standalone" | "instagram-standalone" => {
            e.likes = raw.get("like_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.comments = raw.get("comments_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.saves = raw.get("saved_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.views = raw.get("reach").and_then(|v| v.as_i64()).or_else(|| raw.get("impressions").and_then(|v| v.as_i64())).unwrap_or(0) as i32;
        }

        // LinkedIn: { "likeCount": 42, "commentCount": 12, "shareCount": 5 }
        "linkedin" | "linkedin_page" | "linkedin-page" => {
            e.likes = raw.get("likeCount").or_else(|| raw.get("likes")).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.comments = raw.get("commentCount").or_else(|| raw.get("comments")).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.shares = raw.get("shareCount").or_else(|| raw.get("shares")).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.views = raw.get("impressionCount").or_else(|| raw.get("impressions")).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        }

        // Facebook: Graph API returns reactions.summary.total_count (not "likes")
        // { "reactions": { "summary": { "total_count": 42 } }, "comments": { "summary": { "total_count": 12 } }, "shares": { "count": 5 } }
        "facebook" => {
            e.likes = raw.get("reactions").and_then(|l| l.get("summary")).and_then(|s| s.get("total_count")).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.comments = raw.get("comments").and_then(|c| c.get("summary")).and_then(|s| s.get("total_count")).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.shares = raw.get("shares").and_then(|s| s.get("count")).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            // Reactions breakdown from data array (not the summary object)
            if let Some(data) = raw.get("reactions").and_then(|r| r.get("data")).and_then(|d| d.as_array()) {
                let mut rmap = serde_json::Map::new();
                for reaction in data {
                    if let Some(rtype) = reaction["type"].as_str() {
                        let key = rtype.to_lowercase();
                        let count = rmap.get(&key).and_then(|v| v.as_i64()).unwrap_or(0) + 1;
                        rmap.insert(key, serde_json::json!(count));
                    }
                }
                if !rmap.is_empty() {
                    e.reactions = Some(serde_json::Value::Object(rmap));
                }
            }
        }

        // YouTube: { "viewCount": 1200, "likeCount": 42, "dislikeCount": 2, "commentCount": 12 }
        "youtube" => {
            e.views = raw.get("viewCount").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.likes = raw.get("likeCount").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.comments = raw.get("commentCount").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        }

        // Mastodon: { "favourites_count": 42, "reblogs_count": 8, "replies_count": 3 }
        "mastodon" => {
            e.likes = raw.get("favourites_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.reposts = raw.get("reblogs_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.replies = raw.get("replies_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        }

        // TikTok: { "like_count": 42, "comment_count": 12, "share_count": 5, "view_count": 1200 }
        "tiktok" => {
            e.likes = raw.get("like_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.comments = raw.get("comment_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.shares = raw.get("share_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.views = raw.get("view_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        }

        // Threads: { "like_count": 42, "reply_count": 12, "repost_count": 5, "quote_count": 1 }
        "threads" => {
            e.likes = raw.get("like_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.replies = raw.get("reply_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.reposts = raw.get("repost_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.quotes = raw.get("quote_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        }

        _ => {}
    }

    e
}

/// Convert EngagementData into the DB row format for upsert (all numeric fields plus JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngagementRow {
    pub likes: i32,
    pub comments: i32,
    pub shares: i32,
    pub views: i32,
    pub saves: i32,
    pub quotes: i32,
    pub reposts: i32,
    pub replies: i32,
    pub reactions: serde_json::Value,
    pub upvotes: i32,
    pub downvotes: i32,
    pub upvote_ratio: Option<f32>,
    pub awards: i32,
    pub raw: serde_json::Value,
}

impl From<EngagementData> for EngagementRow {
    fn from(e: EngagementData) -> Self {
        EngagementRow {
            likes: e.likes,
            comments: e.comments,
            shares: e.shares,
            views: e.views,
            saves: e.saves,
            quotes: e.quotes,
            reposts: e.reposts,
            replies: e.replies,
            reactions: e.reactions.unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
            upvotes: e.upvotes,
            downvotes: e.downvotes,
            upvote_ratio: e.upvote_ratio,
            awards: e.awards,
            raw: e.raw.unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
        }
    }
}
