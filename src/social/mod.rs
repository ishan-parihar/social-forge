// ─── Social Provider Trait ─────────────────────────────────────
// Abstract interface for all social media platforms.
// Each platform implements this trait for OAuth + publishing.

pub mod bluesky;
pub mod common;
pub mod facebook;
pub mod instagram;
pub mod linkedin;
pub mod registry;
pub mod x;

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

    /// Generate the OAuth authorization URL
    /// `code_verifier` is the PKCE code verifier (S256 challenge will be derived from it).
    /// Providers that don't use PKCE can ignore this parameter.
    async fn generate_auth_url(
        &self,
        state: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthUrlResponse, ProviderError>;

    /// Check if this provider uses OAuth (vs direct API key / app password).
    /// Non-OAuth providers (like Bluesky) need a different connection flow.
    fn uses_oauth(&self) -> bool {
        true
    }

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

    /// Validate content against platform-specific limits before publishing.
    /// Returns Ok(()) or Err with a user-friendly message.
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
