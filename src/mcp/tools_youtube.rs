// ─── MCP YouTube Tools ──────────────────────────────────────────
// YouTube Data API v3 tools via the YoutubeProvider.
// Follows the same pattern as tools_instagram_standalone.rs.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::social::youtube::YoutubeProvider;

// ── Input/Output Types ──────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct YtSearchVideosInput {
    pub channel_id: String,
    pub query: String,
    pub max_results: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct YtGetVideoInput {
    pub channel_id: String,
    pub video_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct YtListPlaylistsInput {
    pub channel_id: String,
    pub max_results: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct YtGetPlaylistItemsInput {
    pub channel_id: String,
    pub playlist_id: String,
    pub max_results: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct YtGetCommentsInput {
    pub channel_id: String,
    pub video_id: String,
    pub max_results: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct YtGetChannelStatsInput {
    pub channel_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct YtGetAnalyticsInput {
    pub channel_id: String,
    pub metrics: String,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct YtGetSubscriptionsInput {
    pub channel_id: String,
    pub max_results: Option<u32>,
}

// ── Helpers ──────────────────────────────────────────────────

async fn find_yt_token(state: &AppState, user_id: Uuid, channel_id: &str) -> Result<String, String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let yt = integrations
        .iter()
        .find(|i| i.provider_identifier == "youtube" && i.internal_id == channel_id)
        .ok_or_else(|| {
            format!(
                "YouTube channel '{}' not connected. Connect it via the onboarding page first.",
                channel_id
            )
        })?;

    let __tok = yt.access_token.clone();
    let __tok = state.token_key.as_ref()
        .and_then(|k| crate::crypto::decrypt_string(&__tok, k).ok())
        .unwrap_or(__tok);
    Ok(__tok)
}

fn create_yt_provider(state: &AppState) -> YoutubeProvider {
    YoutubeProvider::new(&state.config)
}

// ── Tool Implementations ─────────────────────────────────────

pub async fn handle_yt_search_videos(
    state: &AppState,
    input: &YtSearchVideosInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_yt_token(state, user_id, &input.channel_id).await?;
    let provider = create_yt_provider(state);
    let max_results = input.max_results.unwrap_or(10);
    let result = provider
        .search_videos(&token, &input.query, max_results)
        .await
        .map_err(|e| format!("YouTube search videos failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_yt_get_video(
    state: &AppState,
    input: &YtGetVideoInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_yt_token(state, user_id, &input.channel_id).await?;
    let provider = create_yt_provider(state);
    let result = provider
        .get_video(&token, &input.video_id)
        .await
        .map_err(|e| format!("YouTube get video failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_yt_list_playlists(
    state: &AppState,
    input: &YtListPlaylistsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_yt_token(state, user_id, &input.channel_id).await?;
    let provider = create_yt_provider(state);
    let max_results = input.max_results.unwrap_or(10);
    let result = provider
        .get_playlists(&token, &input.channel_id, max_results)
        .await
        .map_err(|e| format!("YouTube list playlists failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_yt_get_playlist_items(
    state: &AppState,
    input: &YtGetPlaylistItemsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_yt_token(state, user_id, &input.channel_id).await?;
    let provider = create_yt_provider(state);
    let max_results = input.max_results.unwrap_or(10);
    let result = provider
        .get_playlist_items(&token, &input.playlist_id, max_results)
        .await
        .map_err(|e| format!("YouTube get playlist items failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_yt_get_comments(
    state: &AppState,
    input: &YtGetCommentsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_yt_token(state, user_id, &input.channel_id).await?;
    let provider = create_yt_provider(state);
    let max_results = input.max_results.unwrap_or(10);
    let result = provider
        .get_comments(&token, &input.video_id, max_results)
        .await
        .map_err(|e| format!("YouTube get comments failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_yt_get_channel_stats(
    state: &AppState,
    input: &YtGetChannelStatsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_yt_token(state, user_id, &input.channel_id).await?;
    let provider = create_yt_provider(state);
    let result = provider
        .get_channel_stats(&token, &input.channel_id)
        .await
        .map_err(|e| format!("YouTube get channel stats failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_yt_get_analytics(
    state: &AppState,
    input: &YtGetAnalyticsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_yt_token(state, user_id, &input.channel_id).await?;
    let provider = create_yt_provider(state);
    let result = provider
        .get_analytics(&token, &input.channel_id, &input.metrics, &input.start_date, &input.end_date)
        .await
        .map_err(|e| format!("YouTube get analytics failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_yt_get_subscriptions(
    state: &AppState,
    input: &YtGetSubscriptionsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_yt_token(state, user_id, &input.channel_id).await?;
    let provider = create_yt_provider(state);
    let max_results = input.max_results.unwrap_or(10);
    let result = provider
        .get_subscriptions(&token, &input.channel_id, max_results)
        .await
        .map_err(|e| format!("YouTube get subscriptions failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

// ── Find Creators ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct YtFindCreatorsInput {
    /// Topic/query to search for creators
    pub query: String,
    /// Minimum subscriber count filter (optional)
    pub min_subscribers: Option<u32>,
    /// Maximum number of results (default 10, max 50)
    pub max_results: Option<u32>,
}

pub async fn handle_yt_find_creators(
    state: &AppState,
    input: &YtFindCreatorsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let provider = create_yt_provider(state);
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;
    let token = integrations
        .iter()
        .find(|i| i.provider_identifier == "youtube")
        .ok_or_else(|| "No YouTube integration found".to_string())?
        .access_token
        .clone();
    let result = provider
        .find_creators(&token, &input.query, input.min_subscribers, input.max_results)
        .await
        .map_err(|e| format!("YouTube find creators failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct YtReplyCommentInput {
    pub channel_id: String,
    pub comment_id: String,
    pub content: String,
}

pub async fn handle_yt_reply_comment(
    state: &AppState,
    input: &YtReplyCommentInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let provider = create_yt_provider(state);
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;
    let token = integrations
        .iter()
        .find(|i| i.provider_identifier == "youtube")
        .ok_or_else(|| "No YouTube integration found".to_string())?
        .access_token
        .clone();
    let post = crate::social::PostContent {
        content: input.content.clone(),
        media: vec![],
        settings: serde_json::json!({}),
    };
    let result = provider
        .reply_to_comment(&token, &input.comment_id, &post)
        .await
        .map_err(|e| format!("YouTube reply comment failed: {e}"))?;
    Ok(Json(serde_json::json!({
        "data": {
            "id": result.platform_post_id,
            "status": result.status,
        }
    })))
}
