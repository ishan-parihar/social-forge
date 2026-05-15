// ─── MCP TikTok Tools ───────────────────────────────────────────
// TikTok Content Posting API tools via the TikTokProvider.
// Follows the same pattern as tools_youtube.rs.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::crypto;
use crate::social::tiktok::TikTokProvider;
use crate::social::SocialProvider;

// ── Input Types ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TtProfileInput {
    /// TikTok access token (optional — will use stored integration token if empty)
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TtCreatePostInput {
    /// TikTok access token (optional — will use stored integration token if empty)
    pub token: String,
    /// Caption / description for the video (max 150 chars)
    pub text: String,
    /// Base64-encoded video data (alternative to video_url)
    pub video_data: Option<String>,
    /// URL of the video to download and upload
    pub video_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TtListVideosInput {
    /// TikTok access token (optional — will use stored integration token if empty)
    pub token: String,
    /// Maximum number of videos to return (1-100, default 20)
    pub max_count: Option<u32>,
}

// ── Helpers ──────────────────────────────────────────────────

/// Find a TikTok integration for the given user and return (access_token, internal_id).
async fn find_tiktok_integration(
    state: &AppState,
    user_id: Uuid,
) -> Result<(String, String), String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let integration = integrations
        .iter()
        .find(|i| i.provider_identifier == "tiktok")
        .ok_or_else(|| {
            "TikTok not connected. Connect it via the onboarding page first.".to_string()
        })?;

    Ok((
        integration.access_token.clone(),
        integration.internal_id.clone(),
    ))
}

fn create_tiktok_provider(state: &AppState) -> TikTokProvider {
    TikTokProvider::new(&state.config)
}

/// Resolve the effective access token: prefer the user-provided token if non-empty,
/// otherwise fall back to the stored integration token.
fn resolve_token(
    provided: &str,
    stored_token: &str,
) -> String {
    if provided.is_empty() {
        stored_token.to_string()
    } else {
        provided.to_string()
    }
}

// ── Tool Handlers ───────────────────────────────────────────

pub async fn handle_tt_profile(
    state: &AppState,
    input: &TtProfileInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let (stored_token, _) = find_tiktok_integration(state, user_id).await?;
    let token = resolve_token(&input.token, &stored_token);

    let provider = create_tiktok_provider(state);
    let result = provider
        .get_user_info(&token)
        .await
        .map_err(|e| format!("TikTok profile failed: {e}"))?;

    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_tt_create_post(
    state: &AppState,
    input: &TtCreatePostInput,
) -> Result<Json<serde_json::value::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let (stored_token, _) = find_tiktok_integration(state, user_id).await?;
    let token = resolve_token(&input.token, &stored_token);

    // Build media from video_url or video_data
    let mut media = Vec::new();
    if let Some(url) = &input.video_url {
        if !url.is_empty() {
            media.push(crate::social::MediaAttachment {
                url: url.clone(),
                mime_type: "video/mp4".into(),
                alt: Some(input.text.clone()),
            });
        }
    }

    let post = crate::social::PostContent {
        content: input.text.clone(),
        media,
        settings: serde_json::json!({}),
    };

    let provider = create_tiktok_provider(state);
    let result = provider
        .publish(&token, &post)
        .await
        .map_err(|e| format!("TikTok create post failed: {e}"))?;

    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_tt_list_videos(
    state: &AppState,
    input: &TtListVideosInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let (stored_token, _) = find_tiktok_integration(state, user_id).await?;
    let token = resolve_token(&input.token, &stored_token);

    let provider = create_tiktok_provider(state);
    let max_count = input.max_count.unwrap_or(20);
    let result = provider
        .list_videos(&token, max_count)
        .await
        .map_err(|e| format!("TikTok list videos failed: {e}"))?;

    Ok(Json(serde_json::json!({ "data": result })))
}
