// ─── Social Provider Trait ─────────────────────────────────────
// Abstract interface for all social media platforms.
// Each platform implements this trait for OAuth + publishing.

pub mod archive;
pub mod bluesky;
pub mod browser_cookies;
pub mod calendar;
pub mod common;
pub mod devto;
pub mod discord;
pub mod drive;
pub mod facebook;
pub mod github;
pub mod gmail;
pub mod google;
pub mod google_my_business;
pub mod hashnode;
pub mod instagram;
pub mod instagram_standalone;
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
pub mod tier;
pub mod tiktok;
pub mod whatsapp;
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

// ── Error classification (v25 §2 row 9) ────────────────────────

/// Coarse bucket a provider error response falls into.
///
/// Per-provider `map_error` impls turn this into user-facing copy, so the
/// body-sniffing heuristics live in one place instead of being
/// re-implemented (and drifting) across every provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    RateLimited,
    Duplicate,
    MediaRejected,
    AuthExpired,
    Forbidden,
    NotFound,
    InvalidRequest,
}

/// Classify an error response body + HTTP status into an [`ErrorKind`].
///
/// `status` is authoritative when it says something specific (401/403/404/429/
/// 400/422); the body is only sniffed for the cases a status cannot express
/// (duplicate submissions, rejected media, expired grants). Returns `None`
/// when nothing useful can be said — callers then fall back to the raw body.
pub fn classify_error(body: &str, status: u16) -> Option<ErrorKind> {
    let lower = body.to_ascii_lowercase();

    // Body signals are checked before the status code because they are more
    // specific: a 403 carrying "rate limit exceeded" is a rate limit, not a
    // permission problem.
    if lower.contains("\"code\":88")
        || lower.contains("rate limit exceeded")
        || lower.contains("too many requests")
        || lower.contains("ratelimit")
    {
        return Some(ErrorKind::RateLimited);
    }
    if lower.contains("duplicate") || lower.contains("already exists") {
        return Some(ErrorKind::Duplicate);
    }
    if lower.contains("expired")
        || lower.contains("invalid_grant")
        || lower.contains("unauthorized")
        || lower.contains("not authorized")
        || lower.contains("invalid token")
        || lower.contains("bad authentication")
    {
        return Some(ErrorKind::AuthExpired);
    }
    let is_media = ["media", "image", "video", "blob", "upload", "thumbnail"]
        .iter()
        .any(|t| lower.contains(t));
    let is_media_failure = [
        "rejected",
        "invalid",
        "unsupported",
        "not allowed",
        "could not be uploaded",
        "failed to upload",
        "too large",
    ]
    .iter()
    .any(|t| lower.contains(t));
    if is_media && is_media_failure {
        return Some(ErrorKind::MediaRejected);
    }

    match status {
        401 => Some(ErrorKind::AuthExpired),
        403 => Some(ErrorKind::Forbidden),
        404 => Some(ErrorKind::NotFound),
        429 => Some(ErrorKind::RateLimited),
        400 | 422 => Some(ErrorKind::InvalidRequest),
        _ => None,
    }
}

/// Prefix a raw provider error with the user-friendly copy returned by
/// `map_error`, keeping the raw text so debugging stays possible. Falls back
/// to the raw text alone when the provider has no mapping for this error.
pub fn friendly_error(friendly: Option<String>, raw: &str) -> String {
    match friendly {
        Some(msg) => format!("{msg} (raw: {raw})"),
        None => raw.to_string(),
    }
}

// ── Engagement Data Parser ────────────────────────────────────

// ── Meta Graph insights helpers ───────────────────────────────
// Facebook, Instagram, Instagram-standalone and Threads all serve metrics
// through the same Graph insights envelope, so one parser serves all four.

/// Read a numeric JSON field, accepting both integer and float encodings.
fn json_i64(v: Option<&serde_json::Value>) -> Option<i64> {
    match v? {
        serde_json::Value::Number(n) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)),
        serde_json::Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

/// Look up one metric in a Graph insights envelope or a flat media object.
///
/// Accepts both shapes the Graph API returns:
/// - multi-point: `{"data":[{"name":"views","values":[{"value":12,"end_time":"…"}]}]}`
/// - single-value: `{"data":[{"name":"reach","value":30}]}`
/// - flat: `{"like_count":42}`
///
/// Returns `None` when the metric is absent, so callers can fall back to the
/// next candidate metric without mistaking a real zero for a missing field.
pub fn insight_value(raw: &serde_json::Value, metric: &str) -> Option<i64> {
    if let Some(v) = json_i64(raw.get(metric)) {
        return Some(v);
    }
    let entry = raw
        .get("data")
        .and_then(|d| d.as_array())?
        .iter()
        .find(|e| e.get("name").and_then(|n| n.as_str()) == Some(metric))?;

    match entry.get("values").and_then(|v| v.as_array()) {
        // Multi-point envelope: the first bucket is the closest-to-now total
        // for lifetime metrics, and the only one for period-scoped metrics.
        Some(values) => json_i64(values.first().and_then(|v| v.get("value"))),
        None => json_i64(entry.get("value")),
    }
}

/// Parse a Graph insights response into dashboard-shaped `AnalyticsData`.
///
/// One `AnalyticsData` per metric, labelled with the metric name, one data point
/// per bucket. Metrics without a `values` array become a single data point.
/// `percentage_change` is left at 0.0 — no consumer reads it today.
pub fn parse_insights_data(raw: &serde_json::Value) -> Vec<AnalyticsData> {
    raw.get("data")
        .and_then(|d| d.as_array())
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    let label = entry.get("name").and_then(|n| n.as_str())?.to_string();
                    let data = match entry.get("values").and_then(|v| v.as_array()) {
                        Some(values) => values
                            .iter()
                            .map(|v| AnalyticsDataPoint {
                                total: json_i64(v.get("value")).unwrap_or(0).to_string(),
                                date: v
                                    .get("end_time")
                                    .and_then(|d| d.as_str())
                                    .unwrap_or_default()
                                    .to_string(),
                            })
                            .collect(),
                        // Single-value metric (e.g. IG `reach`, `follower_count`)
                        None => vec![AnalyticsDataPoint {
                            total: json_i64(entry.get("value")).unwrap_or(0).to_string(),
                            date: entry
                                .get("end_time")
                                .and_then(|d| d.as_str())
                                .unwrap_or_default()
                                .to_string(),
                        }],
                    };
                    Some(AnalyticsData {
                        label,
                        data,
                        percentage_change: 0.0,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

// ── Provider metric readers (v25 §2 rows 3–4) ─────────────────

/// Read one count out of a JSON object, tolerating the shapes the analytics
/// endpoints use: YouTube Data API sends string-encoded numbers
/// (`"viewCount": "1200"`), Pinterest uses SCREAMING_SNAKE keys
/// (`"IMPRESSION"`), TikTok uses snake_case integers. Key lookup is
/// case-insensitive so both key styles match.
pub(crate) fn count(source: &serde_json::Value, key: &str) -> Option<i32> {
    let obj = source.as_object()?;
    let v = obj.get(key).or_else(|| {
        obj.iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v)
    })?;
    // Saturate instead of `as i32`: an out-of-range count is a bad reading,
    // not a negative one, and must never be written to the DB as such.
    let n = v
        .as_i64()
        .or_else(|| v.as_f64().map(|f| f as i64))
        .or_else(|| v.as_str()?.trim().parse().ok())?;
    Some(i32::try_from(n).unwrap_or(i32::MAX))
}

/// Build single-date `AnalyticsData` series from a flat metrics object.
/// `keys` are `(label, json_key)` pairs; absent or non-numeric keys are skipped
/// so a provider that omits a metric reports the rest rather than a bogus zero.
pub(crate) fn metrics_series(
    source: &serde_json::Value,
    date: &str,
    keys: &[(&str, &str)],
) -> Vec<AnalyticsData> {
    keys.iter()
        .filter_map(|(label, key)| {
            count(source, key).map(|v| AnalyticsData {
                label: (*label).to_string(),
                data: vec![AnalyticsDataPoint {
                    total: v.to_string(),
                    date: date.to_string(),
                }],
                percentage_change: 0.0,
            })
        })
        .collect()
}

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

        // Instagram (Facebook Graph + graph.instagram.com), flat or insights-envelope:
        // { "like_count": 42, "comments_count": 12, "saved": 5, "reach": 300, "views": 420 }
        "instagram" | "instagram_standalone" | "instagram-standalone" => {
            let g = |m: &str| insight_value(&raw, m).unwrap_or(0) as i32;
            e.likes = g("like_count");
            e.comments = g("comments_count");
            e.saves = match insight_value(&raw, "saved") {
                Some(v) => v as i32,
                None => g("saved_count"),
            };
            e.views = ["views", "plays", "reach", "impressions"]
                .iter()
                .find_map(|m| insight_value(&raw, m))
                .unwrap_or(0) as i32;
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
        // The Data API encodes these as strings, and callers may hand us either
        // the normalized flat object or the raw `statistics` sub-object.
        "youtube" => {
            let m = raw.get("statistics").unwrap_or(&raw);
            e.views = count(m, "viewCount").unwrap_or(0);
            e.likes = count(m, "likeCount").unwrap_or(0);
            e.comments = count(m, "commentCount").unwrap_or(0);
        }

        // Mastodon: { "favourites_count": 42, "reblogs_count": 8, "replies_count": 3 }
        "mastodon" => {
            e.likes = raw.get("favourites_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.reposts = raw.get("reblogs_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            e.replies = raw.get("replies_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        }

        // TikTok: { "like_count": 42, "comment_count": 12, "share_count": 5, "view_count": 1200 }
        // The provider flattens the Display API envelope to the single video
        // object before calling this, so the flat shape is the contract.
        "tiktok" => {
            e.likes = count(&raw, "like_count").unwrap_or(0);
            e.comments = count(&raw, "comment_count").unwrap_or(0);
            e.shares = count(&raw, "share_count").unwrap_or(0);
            e.views = count(&raw, "view_count").unwrap_or(0);
        }

        // Pinterest: { "all": { "summary_metrics": { "IMPRESSION": 1200, "PIN_CLICK": 42,
        //                          "OUTBOUND_CLICK": 12, "SAVE": 7 } } }
        // Also accepts a flat metrics object, and a body-level `summary_metrics`.
        // Pinterest has no like metric, so pin clicks stand in as the positive
        // engagement signal (the same treatment Reddit's score gets above);
        // saves and impressions map literally, and OUTBOUND_CLICK stays in `raw`.
        "pinterest" => {
            let m = raw
                .get("all")
                .and_then(|a| a.get("summary_metrics"))
                .or_else(|| raw.get("summary_metrics"))
                .unwrap_or(&raw);
            e.views = count(m, "IMPRESSION").unwrap_or(0);
            e.saves = count(m, "SAVE").unwrap_or(0);
            e.likes = count(m, "PIN_CLICK").unwrap_or(0);
        }

        // Threads: { "like_count": 42, "reply_count": 12, "repost_count": 5, "quote_count": 1 }
        "threads" => {
            let g = |m: &str| insight_value(&raw, m).unwrap_or(0) as i32;
            e.likes = g("like_count");
            e.replies = g("reply_count");
            e.reposts = g("repost_count");
            e.quotes = g("quote_count");
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

#[cfg(test)]
/// Minimal offline `Config` for provider unit tests.
///
/// Lives here so every provider's `#[cfg(test)]` module shares one literal
/// instead of each carrying its own ~60-line copy. No credentials, no DB.
pub(crate) fn test_config() -> crate::config::Config {
    crate::config::Config {
        database_url: "postgres://test:test@localhost:5432/test".into(),
        jwt_secret: "test".into(),
        app_password: "test".into(),
        app_url: "http://localhost:3000".into(),
        frontend_url: "http://localhost:4200".into(),
        x_client_id: None,
        x_client_secret: None,
        x_auth_token: None,
        x_ct0: None,
        linkedin_client_id: Some("test_linkedin_id".into()),
        linkedin_client_secret: Some("test_linkedin_secret".into()),
        bluesky_handle: Some("test.bsky.social".into()),
        bluesky_app_password: Some("test-app-password".into()),
        facebook_client_id: None,
        facebook_client_secret: None,
        instagram_client_id: None,
        instagram_client_secret: None,
        threads_app_id: None,
        threads_app_secret: None,
        youtube_client_id: None,
        youtube_client_secret: None,
        reddit_client_id: Some("test_reddit_id".into()),
        reddit_client_secret: Some("test_reddit_secret".into()),
        reddit_username: Some("test_reddit_user".into()),
        reddit_password: Some("test_reddit_pass".into()),
        reddit_access_token: None,
        reddit_refresh_token: None,
        discord_client_id: None,
        discord_client_secret: None,
        discord_bot_token: None,
        telegram_bot_tokens: None,
        telegram_session_dir: None,
        telegram_api_id: None,
        telegram_api_hash: None,
        tiktok_client_id: None,
        tiktok_client_secret: None,
        medium_access_token: None,
        devto_api_key: None,
        pinterest_client_id: None,
        pinterest_client_secret: None,
        whatsapp_store_dir: None,
        slack_client_id: None,
        slack_client_secret: None,
        instagram_app_id: None,
        instagram_app_secret: None,
        mastodon_client_id: None,
        mastodon_client_secret: None,
        mastodon_instance_url: None,
        hashnode_api_key: None,
        github_token: None,
        neynar_api_key: None,
        token_encryption_key: None,
        media_dir: "./uploads".into(),
        stripe_secret_key: None,
        stripe_webhook_secret: None,
        stripe_price_free: None,
        stripe_price_pro_monthly: None,
        stripe_price_pro_annual: None,
        stripe_price_business_monthly: None,
        stripe_price_business_annual: None,
        llm_endpoint: None,
        llm_model: None,
        dub_co_api_key: None,
        dub_co_workspace: None,
        strip_links_from_x: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── classify_error ───────────────────────────────────────

    #[test]
    fn classify_error_detects_rate_limit_from_code_88() {
        assert_eq!(
            classify_error(r#"{"errors":[{"code":88}]}"#, 400),
            Some(ErrorKind::RateLimited)
        );
    }

    #[test]
    fn classify_error_detects_rate_limit_from_429_status() {
        assert_eq!(
            classify_error("too many requests", 429),
            Some(ErrorKind::RateLimited)
        );
    }

    #[test]
    fn classify_error_prefers_body_rate_limit_over_403() {
        assert_eq!(
            classify_error("Rate limit exceeded for this method", 403),
            Some(ErrorKind::RateLimited)
        );
    }

    #[test]
    fn classify_error_detects_duplicate() {
        assert_eq!(
            classify_error("Status is a duplicate.", 403),
            Some(ErrorKind::Duplicate)
        );
    }

    #[test]
    fn classify_error_detects_media_rejection() {
        assert_eq!(
            classify_error("The media upload was rejected by the platform", 400),
            Some(ErrorKind::MediaRejected)
        );
    }

    #[test]
    fn classify_error_detects_expired_grant() {
        assert_eq!(
            classify_error(r#"{"error":"invalid_grant"}"#, 400),
            Some(ErrorKind::AuthExpired)
        );
    }

    #[test]
    fn classify_error_falls_back_to_status_codes() {
        assert_eq!(classify_error("", 401), Some(ErrorKind::AuthExpired));
        assert_eq!(classify_error("", 403), Some(ErrorKind::Forbidden));
        assert_eq!(classify_error("", 404), Some(ErrorKind::NotFound));
        assert_eq!(classify_error("", 400), Some(ErrorKind::InvalidRequest));
        assert_eq!(classify_error("", 422), Some(ErrorKind::InvalidRequest));
    }

    #[test]
    fn classify_error_returns_none_for_uninformative_server_error() {
        assert_eq!(classify_error("Internal Server Error", 500), None);
    }

    // ── parse_engagement_data: X ─────────────────────────────

    #[test]
    fn parse_engagement_data_maps_x_v2_public_metrics() {
        let raw = serde_json::json!({
            "public_metrics": {
                "like_count": 42, "retweet_count": 8, "reply_count": 3,
                "quote_count": 1, "impression_count": 1200, "bookmark_count": 5
            }
        });
        let e = parse_engagement_data("x", raw);
        assert_eq!(e.likes, 42);
        assert_eq!(e.reposts, 8);
        assert_eq!(e.replies, 3);
        assert_eq!(e.quotes, 1);
        assert_eq!(e.views, 1200);
        assert_eq!(e.saves, 5);
    }

    // ── parse_engagement_data: Reddit ────────────────────────

    #[test]
    fn parse_engagement_data_maps_reddit_submission_metrics() {
        let raw = serde_json::json!({
            "score": 120, "num_comments": 12, "upvote_ratio": 0.95,
            "ups": 123, "downs": 3, "total_awards_received": 2
        });
        let e = parse_engagement_data("reddit", raw);
        assert_eq!(e.likes, 120);
        assert_eq!(e.comments, 12);
        assert_eq!(e.upvote_ratio, Some(0.95));
        assert_eq!(e.upvotes, 123);
        assert_eq!(e.downvotes, 3);
        assert_eq!(e.awards, 2);
    }

    #[test]
    fn parse_engagement_data_treats_null_reddit_ups_as_zero() {
        // Reddit omits ups/downs for non-authors — must not panic or read as NaN.
        let raw = serde_json::json!({ "score": 7, "num_comments": 1, "ups": null, "downs": null });
        let e = parse_engagement_data("reddit", raw);
        assert_eq!(e.upvotes, 0);
        assert_eq!(e.downvotes, 0);
        assert_eq!(e.likes, 7);
    }

    // ── parse_engagement_data: Bluesky ───────────────────────

    #[test]
    fn parse_engagement_data_maps_bluesky_post_metrics() {
        let raw = serde_json::json!({
            "likeCount": 42, "repostCount": 8, "replyCount": 3, "quoteCount": 1
        });
        let e = parse_engagement_data("bluesky", raw);
        assert_eq!(e.likes, 42);
        assert_eq!(e.reposts, 8);
        assert_eq!(e.replies, 3);
        assert_eq!(e.quotes, 1);
    }

    // ── parse_engagement_data: LinkedIn (member + page) ──────

    #[test]
    fn parse_engagement_data_maps_linkedin_social_actions() {
        let raw = serde_json::json!({
            "likeCount": 42, "commentCount": 12, "shareCount": 5, "impressionCount": 900
        });
        for provider in ["linkedin", "linkedin_page", "linkedin-page"] {
            let e = parse_engagement_data(provider, raw.clone());
            assert_eq!(e.likes, 42, "{provider} likes");
            assert_eq!(e.comments, 12, "{provider} comments");
            assert_eq!(e.shares, 5, "{provider} shares");
            assert_eq!(e.views, 900, "{provider} views");
        }
    }

    #[test]
    fn parse_engagement_data_unknown_provider_yields_zeroes() {
        let e = parse_engagement_data("not-a-provider", serde_json::json!({ "likeCount": 5 }));
        assert_eq!(e.likes, 0);
    }
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

#[cfg(test)]
mod insights_tests {
    use super::*;

    #[test]
    fn parse_should_read_flat_metric() {
        let raw = serde_json::json!({ "like_count": 42 });
        assert_eq!(insight_value(&raw, "like_count"), Some(42));
    }

    #[test]
    fn parse_should_read_metric_from_values_envelope() {
        let raw = serde_json::json!({
            "data": [
                { "name": "reach", "period": "lifetime", "values": [{ "value": 300, "end_time": "2026-09-29T00:00:00+0000" }] }
            ]
        });
        assert_eq!(insight_value(&raw, "reach"), Some(300));
    }

    #[test]
    fn parse_should_read_metric_from_single_value_envelope() {
        let raw = serde_json::json!({ "data": [{ "name": "follower_count", "value": 1200 }] });
        assert_eq!(insight_value(&raw, "follower_count"), Some(1200));
    }

    #[test]
    fn parse_should_return_none_for_absent_metric() {
        let raw = serde_json::json!({ "data": [{ "name": "reach", "value": 1 }] });
        assert_eq!(insight_value(&raw, "views"), None);
    }

    #[test]
    fn parse_should_expand_daily_insights_into_dashboard_series() {
        let raw = serde_json::json!({
            "data": [
                {
                    "name": "views",
                    "period": "day",
                    "values": [
                        { "value": 10, "end_time": "2026-09-28T00:00:00+0000" },
                        { "value": 25, "end_time": "2026-09-29T00:00:00+0000" }
                    ]
                },
                { "name": "follower_count", "value": 1200 }
            ]
        });
        let series = parse_insights_data(&raw);
        assert_eq!(series.len(), 2);
        assert_eq!(series[0].label, "views");
        assert_eq!(series[0].data.len(), 2);
        assert_eq!(series[0].data[1].total, "25");
        assert_eq!(series[0].data[1].date, "2026-09-29T00:00:00+0000");
        assert_eq!(series[1].label, "follower_count");
        assert_eq!(series[1].data[0].total, "1200");
    }

    #[test]
    fn parse_should_map_facebook_reaction_comment_share_shape() {
        let raw = serde_json::json!({
            "id": "123_456",
            "reactions": {
                "summary": { "total_count": 42 },
                "data": [
                    { "type": "LIKE", "total_count": 30 },
                    { "type": "LOVE", "total_count": 7 },
                    { "type": "LIKE", "total_count": 5 }
                ]
            },
            "comments": { "summary": { "total_count": 12 } },
            "shares": { "count": 5 }
        });
        let e = parse_engagement_data("facebook", raw);
        assert_eq!(e.likes, 42);
        assert_eq!(e.comments, 12);
        assert_eq!(e.shares, 5);
        // Repeated reaction types are aggregated into the breakdown map.
        assert_eq!(e.reactions, Some(serde_json::json!({ "like": 2, "love": 1 })));
    }

    #[test]
    fn parse_should_map_instagram_graph_media_shape() {
        let raw = serde_json::json!({
            "id": "17900000000000000",
            "like_count": 42,
            "comments_count": 12,
            "saved": 7,
            "reach": 300,
            "views": 420
        });
        let e = parse_engagement_data("instagram", raw);
        assert_eq!(e.likes, 42);
        assert_eq!(e.comments, 12);
        assert_eq!(e.saves, 7);
        assert_eq!(e.views, 420);
    }

    #[test]
    fn parse_should_map_instagram_standalone_insights_envelope_shape() {
        let raw = serde_json::json!({
            "id": "17900000000000000",
            "like_count": 8,
            "comments_count": 2,
            "data": [
                { "name": "reach", "values": [{ "value": 90, "end_time": "2026-09-29T00:00:00+0000" }] },
                { "name": "saved", "values": [{ "value": 3, "end_time": "2026-09-29T00:00:00+0000" }] }
            ]
        });
        let e = parse_engagement_data("instagram-standalone", raw);
        assert_eq!(e.likes, 8);
        assert_eq!(e.comments, 2);
        assert_eq!(e.saves, 3);
        assert_eq!(e.views, 90);
    }

    #[test]
    fn parse_should_map_threads_reaction_shape() {
        let raw = serde_json::json!({
            "id": "8888888888888888888",
            "like_count": 42,
            "reply_count": 12,
            "repost_count": 5,
            "quote_count": 1
        });
        let e = parse_engagement_data("threads", raw);
        assert_eq!(e.likes, 42);
        assert_eq!(e.replies, 12);
        assert_eq!(e.reposts, 5);
        assert_eq!(e.quotes, 1);
    }

    #[test]
    fn parse_should_keep_raw_payload_for_engagement_row() {
        let raw = serde_json::json!({ "like_count": 1 });
        let e = parse_engagement_data("instagram", raw.clone());
        assert_eq!(e.raw, Some(raw));
    }
}
