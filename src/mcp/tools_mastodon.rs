// ─── MCP Mastodon Tools ───────────────────────────────────────
// Content posting, timeline reading, and search API tools
// via the MastodonProvider.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::social::mastodon::MastodonProvider;
use crate::social::SocialProvider;

// ── Input Types ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MsCreatePostInput {
    /// Content of the toot/post (max 500 characters)
    pub content: String,
    /// Visibility: "public", "unlisted", "private", "direct" (default: "public")
    pub visibility: Option<String>,
    /// Local ID of the status being replied to
    pub in_reply_to_id: Option<String>,
    /// IDs of media attachments to include (uploaded via media_ids or via media_urls)
    pub media_ids: Option<Vec<String>>,
    /// URLs of media to upload and attach (alternative to media_ids)
    pub media_urls: Option<Vec<String>>,
    /// Spoiler/content warning text
    pub spoiler_text: Option<String>,
    /// Mark the post as sensitive
    pub sensitive: Option<bool>,
    /// ISO 639-1 language code (e.g. "en", "fr")
    pub language: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MsGetTimelineInput {
    /// Timeline type: "home", "local", "trending", "public" (default: "home")
    pub timeline_type: Option<String>,
    /// Maximum number of posts to return (default: 20)
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MsGetPostInput {
    /// ID of the post/status to retrieve
    pub post_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MsSearchInput {
    /// Search query
    pub query: String,
    /// Maximum number of results to return (default: 20)
    pub limit: Option<i32>,
}

// ── Helpers ──────────────────────────────────────────────────

/// Find a Mastodon integration for the given user and return access_token.
async fn find_mastodon_integration(
    state: &AppState,
    user_id: Uuid,
) -> Result<String, String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let integration = integrations
        .iter()
        .find(|i| i.provider_identifier == "mastodon")
        .ok_or_else(|| {
            "Mastodon not connected. Connect it via the integrations page first.".to_string()
        })?;

    let tok = crate::crypto::maybe_decrypt_token(&integration.access_token, state.token_key.as_ref());
    Ok(tok)
}

fn create_mastodon_provider(state: &AppState) -> MastodonProvider {
    MastodonProvider::new(&state.config)
}

// ── Tool Handlers ───────────────────────────────────────────

pub async fn handle_ms_create_post(
    state: &AppState,
    input: &MsCreatePostInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let stored_token = find_mastodon_integration(state, user_id).await?;
    let token = stored_token;

    // Build media attachments from media_urls if provided
    let mut media = Vec::new();
    if let Some(urls) = &input.media_urls {
        for url in urls {
            if !url.is_empty() {
                media.push(crate::social::MediaAttachment {
                    url: url.clone(),
                    mime_type: "image/jpeg".into(),
                    alt: None,
                    poster_url: None,
                });
            }
        }
    }

    // Build settings
    let mut settings = serde_json::json!({});
    if let Some(vis) = &input.visibility {
        settings["visibility"] = serde_json::json!(vis);
    }
    if let Some(reply_to) = &input.in_reply_to_id {
        settings["in_reply_to_id"] = serde_json::json!(reply_to);
    }
    if let Some(media_ids) = &input.media_ids {
        settings["media_ids"] = serde_json::json!(media_ids);
    }
    if let Some(spoiler) = &input.spoiler_text {
        settings["spoiler_text"] = serde_json::json!(spoiler);
    }
    if let Some(sensitive) = input.sensitive {
        settings["sensitive"] = serde_json::json!(sensitive);
    }
    if let Some(lang) = &input.language {
        settings["language"] = serde_json::json!(lang);
    }

    let post = crate::social::PostContent {
        content: input.content.clone(),
        media,
        settings,
    };

    let provider = create_mastodon_provider(state);
    let result = provider
        .publish(&token, &post)
        .await
        .map_err(|e| format!("Mastodon create post failed: {e}"))?;

    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_ms_get_timeline(
    state: &AppState,
    input: &MsGetTimelineInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let stored_token = find_mastodon_integration(state, user_id).await?;
    let token = stored_token;

    let tl_type = input
        .timeline_type
        .as_deref()
        .unwrap_or("home");
    let limit = input.limit.unwrap_or(20);

    let provider = create_mastodon_provider(state);
    let result = provider
        .get_timeline(&token, tl_type, limit)
        .await
        .map_err(|e| format!("Mastodon timeline failed: {e}"))?;

    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_ms_get_post(
    state: &AppState,
    input: &MsGetPostInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let stored_token = find_mastodon_integration(state, user_id).await?;
    let token = stored_token;

    let provider = create_mastodon_provider(state);
    let result = provider
        .get_post(&token, &input.post_id)
        .await
        .map_err(|e| format!("Mastodon get post failed: {e}"))?;

    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_ms_search(
    state: &AppState,
    input: &MsSearchInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let stored_token = find_mastodon_integration(state, user_id).await?;
    let token = stored_token;

    let limit = input.limit.unwrap_or(20);

    let provider = create_mastodon_provider(state);
    let result = provider
        .search(&token, &input.query, limit)
        .await
        .map_err(|e| format!("Mastodon search failed: {e}"))?;

    Ok(Json(serde_json::json!({ "data": result })))
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MsReplyInput {
    pub status_id: String,
    pub content: String,
}

pub async fn handle_ms_reply(
    state: &AppState,
    input: &MsReplyInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let stored_token = find_mastodon_integration(state, user_id).await?;
    let token = stored_token;

    let provider = create_mastodon_provider(state);
    let post = crate::social::PostContent {
        content: input.content.clone(),
        media: vec![],
        settings: serde_json::json!({}),
    };
    let result = provider
        .reply_to_comment(&token, &input.status_id, &post)
        .await
        .map_err(|e| format!("Mastodon reply failed: {e}"))?;

    Ok(Json(serde_json::json!({
        "data": {
            "id": result.platform_post_id,
            "url": result.platform_post_url,
            "status": result.status,
        }
    })))
}

// ── Analytics ────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MsGetAnalyticsInput {
    /// Number of days of analytics (default 30)
    #[serde(default = "default_days")]
    pub days: u32,
}

fn default_days() -> u32 { 30 }

/// Get account-level analytics for the authenticated Mastodon user
/// (followers, following, statuses count, and recent engagement).
pub async fn handle_ms_get_analytics(
    state: &AppState,
    input: &MsGetAnalyticsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_mastodon_integration(state, user_id).await?;
    let provider = create_mastodon_provider(state);

    // Get user profile (includes follower/following/status counts)
    let user_info = provider
        .get_user_info(&token)
        .await
        .map_err(|e| format!("Mastodon analytics failed: {e}"))?;

    // Get recent timeline posts for engagement metrics
    let timeline = provider
        .get_timeline(&token, "home", 30)
        .await
        .map_err(|e| format!("Mastodon timeline for analytics failed: {e}"))?;

    let posts = timeline.as_array().cloned().unwrap_or_default();
    let total_replies: i64 = posts.iter()
        .filter_map(|p| p["replies_count"].as_i64())
        .sum();
    let total_reblogs: i64 = posts.iter()
        .filter_map(|p| p["reblogs_count"].as_i64())
        .sum();
    let total_favourites: i64 = posts.iter()
        .filter_map(|p| p["favourites_count"].as_i64())
        .sum();

    Ok(Json(serde_json::json!({
        "account": user_info,
        "recent_engagement": {
            "posts_analyzed": posts.len(),
            "total_replies": total_replies,
            "total_reblogs": total_reblogs,
            "total_favourites": total_favourites,
        },
        "days": input.days,
    })))
}
