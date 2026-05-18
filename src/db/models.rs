// ─── Database Models ──────────────────────────────────────────
// Rust structs matching the PostgreSQL schema.
// Used for serialization/deserialization with sqlx and serde.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── User ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password: String,
    pub name: String,
    pub timezone: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Public user info (no password, no internal fields)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UserPublic {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub timezone: i32,
}

impl From<User> for UserPublic {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            email: u.email,
            name: u.name,
            timezone: u.timezone,
        }
    }
}

// ── Integration (Social Channel) ────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Integration {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider_identifier: String,
    pub provider_name: String,
    pub internal_id: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub profile_name: Option<String>,
    pub profile_picture: Option<String>,
    pub profile_url: Option<String>,
    pub disabled: bool,
    pub refresh_needed: bool,
    pub root_internal_id: Option<String>,
    pub posting_times: serde_json::Value,
    pub auth_method: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IntegrationPublic {
    pub id: Uuid,
    pub provider_identifier: String,
    pub provider_name: String,
    pub internal_id: String,
    pub profile_name: Option<String>,
    pub profile_picture: Option<String>,
    pub profile_url: Option<String>,
    pub disabled: bool,
    pub refresh_needed: bool,
    pub root_internal_id: Option<String>,
    pub posting_times: serde_json::Value,
    pub auth_method: String,
}

impl From<Integration> for IntegrationPublic {
    fn from(i: Integration) -> Self {
        Self {
            id: i.id,
            provider_identifier: i.provider_identifier,
            provider_name: i.provider_name,
            internal_id: i.internal_id,
            profile_name: i.profile_name,
            profile_picture: i.profile_picture,
            profile_url: i.profile_url,
            disabled: i.disabled,
            refresh_needed: i.refresh_needed,
            root_internal_id: i.root_internal_id,
            posting_times: i.posting_times,
            auth_method: i.auth_method,
        }
    }
}

// ── Post ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "post_state")]
#[sqlx(rename_all = "lowercase")]
pub enum PostState {
    Draft,
    Queued,
    Published,
    Error,
}

impl std::fmt::Display for PostState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PostState::Draft => write!(f, "draft"),
            PostState::Queued => write!(f, "queued"),
            PostState::Published => write!(f, "published"),
            PostState::Error => write!(f, "error"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Post {
    pub id: Uuid,
    pub user_id: Uuid,
    pub integration_id: Uuid,
    pub state: PostState,
    pub content: String,
    pub title: Option<String>,
    pub media: serde_json::Value,
    pub settings: serde_json::Value,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
    pub platform_post_id: Option<String>,
    pub platform_post_url: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub repeat_interval_days: Option<i32>,
    pub repeat_end_date: Option<DateTime<Utc>>,
    pub group_id: Option<Uuid>,
    pub first_comment: Option<String>,
    pub sequence: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PostPublic {
    pub id: Uuid,
    pub integration_id: Uuid,
    pub integration_name: String,
    pub state: String,
    pub content: String,
    pub title: Option<String>,
    pub media: serde_json::Value,
    pub scheduled_at: Option<String>,
    pub published_at: Option<String>,
    pub platform_post_url: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub repeat_interval_days: Option<i32>,
    pub repeat_end_date: Option<String>,
    pub group_id: Option<Uuid>,
    pub first_comment: Option<String>,
    pub sequence: i32,
}

impl From<Post> for PostPublic {
    fn from(p: Post) -> Self {
        Self {
            id: p.id,
            integration_id: p.integration_id,
            integration_name: String::new(),
            state: p.state.to_string(),
            content: p.content,
            title: p.title,
            media: p.media,
            scheduled_at: p.scheduled_at.map(|d| d.to_rfc3339()),
            published_at: p.published_at.map(|d| d.to_rfc3339()),
            platform_post_url: p.platform_post_url,
            error_message: p.error_message,
            created_at: p.created_at.to_rfc3339(),
            repeat_interval_days: p.repeat_interval_days,
            repeat_end_date: p.repeat_end_date.map(|d| d.to_rfc3339()),
            group_id: p.group_id,
            first_comment: p.first_comment,
            sequence: p.sequence,
        }
    }
}

// ── Post with Integration Details (for publishing) ──────────

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PostWithIntegration {
    pub id: Uuid,
    pub user_id: Uuid,
    pub integration_id: Uuid,
    pub state: PostState,
    pub content: String,
    pub title: Option<String>,
    pub media: serde_json::Value,
    pub settings: serde_json::Value,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
    pub platform_post_id: Option<String>,
    pub platform_post_url: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub repeat_interval_days: Option<i32>,
    pub repeat_end_date: Option<DateTime<Utc>>,
    pub group_id: Option<Uuid>,
    pub first_comment: Option<String>,
    pub sequence: i32,
    // Joined from integrations
    pub provider_identifier: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub integration_disabled: bool,
    pub integration_refresh_needed: bool,
}

// ── Media ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MediaEntry {
    pub id: Uuid,
    pub user_id: Uuid,
    pub original_name: String,
    pub storage_path: String,
    pub mime_type: String,
    pub file_size: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MediaPublic {
    pub id: Uuid,
    pub original_name: String,
    pub url: String,
    pub mime_type: String,
    pub file_size: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

impl From<MediaEntry> for MediaPublic {
    fn from(m: MediaEntry) -> Self {
        Self {
            id: m.id,
            original_name: m.original_name,
            url: format!("/api/media/{}", m.id),
            mime_type: m.mime_type,
            file_size: m.file_size,
            width: m.width,
            height: m.height,
        }
    }
}

// ── OAuth State ─────────────────────────────────────────────

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OAuthState {
    pub id: Uuid,
    pub state: String,
    pub provider: String,
    pub code_verifier: String,
    pub redirect_uri: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

// ── Notification ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub body: String,
    pub notification_type: String,
    pub reference_type: Option<String>,
    pub reference_id: Option<String>,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NotificationPublic {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    pub notification_type: String,
    pub reference_type: Option<String>,
    pub reference_id: Option<String>,
    pub is_read: bool,
    pub created_at: String,
}

impl From<Notification> for NotificationPublic {
    fn from(n: Notification) -> Self {
        Self {
            id: n.id,
            title: n.title,
            body: n.body,
            notification_type: n.notification_type,
            reference_type: n.reference_type,
            reference_id: n.reference_id,
            is_read: n.is_read,
            created_at: n.created_at.to_rfc3339(),
        }
    }
}

// ── Rss Feed ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RssFeed {
    pub id: Uuid,
    pub user_id: Uuid,
    pub feed_url: String,
    pub integration_id: Uuid,
    pub title: String,
    pub last_polled_at: Option<DateTime<Utc>>,
    pub poll_interval_min: i32,
    pub enabled: bool,
    pub use_ai_summary: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RssPost {
    pub id: Uuid,
    pub feed_id: Uuid,
    pub post_id: Option<Uuid>,
    pub guid: String,
    pub title: String,
    pub url: String,
    pub published_at: Option<DateTime<Utc>>,
    pub content_hash: String,
    pub is_imported: bool,
    pub created_at: DateTime<Utc>,
}

// ── Signature ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Signature {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub content: String,
    pub provider: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

use schemars::JsonSchema;
