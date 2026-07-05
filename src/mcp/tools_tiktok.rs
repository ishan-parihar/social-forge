// ─── MCP TikTok Tools ───────────────────────────────────────────
// TikTok Content Posting API tools via the TikTokProvider.
// Follows the same pattern as tools_youtube.rs.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::social::tiktok::TikTokProvider;
use crate::social::SocialProvider;

// ── Input Types ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TtProfileInput {
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TtCreatePostInput {
    /// Caption / description for the video (max 150 chars)
    pub text: String,
    /// Base64-encoded video data (alternative to video_url)
    pub video_data: Option<String>,
    /// URL of the video to download and upload
    pub video_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TtListVideosInput {
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

// ── Tool Handlers ───────────────────────────────────────────

pub async fn handle_tt_profile(
    state: &AppState,
    input: &TtProfileInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let (stored_token, _) = find_tiktok_integration(state, user_id).await?;
    let token = stored_token;

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
    let token = stored_token;

    // Build media from video_url or video_data
    let mut media = Vec::new();
    if let Some(url) = &input.video_url {
        if !url.is_empty() {
            media.push(crate::social::MediaAttachment {
                url: url.clone(),
                mime_type: "video/mp4".into(),
                alt: Some(input.text.clone()),
                poster_url: None,
            });
        }
    }

    let post = crate::social::PostContent {
        content: input.text.clone(),
        media,
        settings: serde_json::json!({}),
    in_reply_to: None,
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
    let token = stored_token;

    let provider = create_tiktok_provider(state);
    let max_count = input.max_count.unwrap_or(20);
    let result = provider
        .list_videos(&token, max_count)
        .await
        .map_err(|e| format!("TikTok list videos failed: {e}"))?;

    Ok(Json(serde_json::json!({ "data": result })))
}

// ── Analytics ────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TtGetAnalyticsInput {
    /// Number of days of analytics (default 30)
    #[serde(default = "default_days")]
    pub days: u32,
}

fn default_days() -> u32 { 30 }

/// Get account-level analytics for the authenticated TikTok user
/// (follower count, following count, likes, video count).
pub async fn handle_tt_get_analytics(
    state: &AppState,
    input: &TtGetAnalyticsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let (token, _internal_id) = find_tiktok_integration(state, user_id).await?;
    let provider = create_tiktok_provider(state);

    let user_info = provider
        .get_user_info(&token)
        .await
        .map_err(|e| format!("TikTok analytics failed: {e}"))?;

    // Also fetch video list for engagement stats
    let videos = provider
        .list_videos(&token, 20)
        .await
        .map_err(|e| format!("TikTok video list for analytics failed: {e}"))?;

    let total_views: i64 = videos
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v["view_count"].as_i64())
                .sum()
        })
        .unwrap_or(0);
    let total_likes: i64 = videos
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v["like_count"].as_i64())
                .sum()
        })
        .unwrap_or(0);
    let total_comments: i64 = videos
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v["comment_count"].as_i64())
                .sum()
        })
        .unwrap_or(0);
    let total_shares: i64 = videos
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v["share_count"].as_i64())
                .sum()
        })
        .unwrap_or(0);

    Ok(Json(serde_json::json!({
        "account": user_info,
        "recent_videos": {
            "count": videos.as_array().map(|a| a.len()).unwrap_or(0),
            "total_views": total_views,
            "total_likes": total_likes,
            "total_comments": total_comments,
            "total_shares": total_shares,
        },
        "days": input.days,
    })))
}
