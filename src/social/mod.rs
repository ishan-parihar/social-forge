// ─── Social Provider Trait ─────────────────────────────────────
// Abstract interface for all social media platforms.
// Each platform implements this trait for OAuth + publishing.

pub mod bluesky;
pub mod calendar;
pub mod common;
pub mod devto;
pub mod discord;
pub mod farcaster;
pub mod nostr;
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
pub mod mewe;
pub mod moltbook;
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
pub mod twitch;
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
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MediaAttachment {
    pub url: String,
    pub mime_type: String,
    pub alt: Option<String>,
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

impl ProviderError {
    pub fn is_token_expired(&self) -> bool {
        matches!(self, ProviderError::TokenExpired)
    }

    pub fn is_rate_limited(&self) -> bool {
        matches!(self, ProviderError::RateLimited(_))
    }
}
