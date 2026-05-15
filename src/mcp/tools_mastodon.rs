// ─── MCP Mastodon Tools ───────────────────────────────────────
// Content posting, timeline reading, and search API tools
// via the MastodonProvider.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::crypto;
use crate::social::mastodon::MastodonProvider;
use crate::social::SocialProvider;

// ── Input Types ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MsCreatePostInput {
    /// Mastodon access token (optional — will use stored integration token if empty)
    pub token: String,
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
    /// Mastodon access token (optional — will use stored integration token if empty)
    pub token: String,
    /// Timeline type: "home", "local", "trending", "public" (default: "home")
    pub timeline_type: Option<String>,
    /// Maximum number of posts to return (default: 20)
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MsGetPostInput {
    /// Mastodon access token (optional — will use stored integration token if empty)
    pub token: String,
    /// ID of the post/status to retrieve
    pub post_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MsSearchInput {
    /// Mastodon access token (optional — will use stored integration token if empty)
    pub token: String,
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

    let __tok = integration.access_token.clone();
    let __tok = state.token_key.as_ref()
        .and_then(|k| crate::crypto::decrypt_string(&__tok, k).ok())
        .unwrap_or(__tok);
    Ok(__tok)
}

fn create_mastodon_provider(state: &AppState) -> MastodonProvider {
    MastodonProvider::new(&state.config)
}

/// Resolve the effective access token: prefer the user-provided token if non-empty,
/// otherwise fall back to the stored integration token.
fn resolve_token(provided: &str, stored_token: &str) -> String {
    if provided.is_empty() {
        stored_token.to_string()
    } else {
        provided.to_string()
    }
}

// ── Tool Handlers ───────────────────────────────────────────

pub async fn handle_ms_create_post(
    state: &AppState,
    input: &MsCreatePostInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let stored_token = find_mastodon_integration(state, user_id).await?;
    let token = resolve_token(&input.token, &stored_token);

    // Build media attachments from media_urls if provided
    let mut media = Vec::new();
    if let Some(urls) = &input.media_urls {
        for url in urls {
            if !url.is_empty() {
                media.push(crate::social::MediaAttachment {
                    url: url.clone(),
                    mime_type: "image/jpeg".into(),
                    alt: None,
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
    let token = resolve_token(&input.token, &stored_token);

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
    let token = resolve_token(&input.token, &stored_token);

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
    let token = resolve_token(&input.token, &stored_token);

    let limit = input.limit.unwrap_or(20);

    let provider = create_mastodon_provider(state);
    let result = provider
        .search(&token, &input.query, limit)
        .await
        .map_err(|e| format!("Mastodon search failed: {e}"))?;

    Ok(Json(serde_json::json!({ "data": result })))
}
