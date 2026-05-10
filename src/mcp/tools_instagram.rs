// ─── MCP Instagram Tools ───────────────────────────────────────────
// Instagram-specific read/write tools via Instagram Graph API.
// Follows the same pattern as tools_reddit.rs and tools_x.rs.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::social::instagram::InstagramProvider;

// ── Input/Output Types ──────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IgGetMediaInput {
    pub ig_id: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IgGetMediaDetailInput {
    pub ig_id: String,
    pub media_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IgGetCommentsInput {
    pub ig_id: String,
    pub media_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IgSearchHashtagInput {
    pub ig_id: String,
    pub query: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IgGetHashtagMediaInput {
    pub ig_id: String,
    pub hashtag_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IgGetInsightsInput {
    pub ig_id: String,
    pub metric: String,
    pub period: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IgGetTaggedInput {
    pub ig_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IgCreateContainerInput {
    pub ig_id: String,
    pub media_type: String,
    pub media_url: String,
    pub caption: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IgPublishContainerInput {
    pub ig_id: String,
    pub creation_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IgReplyToCommentInput {
    pub ig_id: String,
    pub comment_id: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IgGetReelsInput {
    pub ig_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IgGetStoriesInput {
    pub ig_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IgGetFollowersInput {
    pub ig_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IgBusinessDiscoveryInput {
    pub ig_id: String,
    pub target_username: String,
}


#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IgGetInsightsAudienceInput {
    pub ig_id: String,
}

// ── Helpers ──────────────────────────────────────────────────

async fn find_instagram_token(state: &AppState, user_id: Uuid, ig_id: &str) -> Result<String, String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let ig = integrations
        .iter()
        .find(|i| i.provider_identifier == "instagram" && i.internal_id == ig_id)
        .or_else(|| {
            integrations.iter().find(|i| i.provider_identifier == "instagram-standalone" && i.internal_id == ig_id)
        })
        .ok_or_else(|| {
            format!("Instagram account '{ig_id}' not connected. Connect it via the onboarding page first.")
        })?;

    Ok(ig.access_token.clone())
}

fn create_provider(state: &AppState) -> InstagramProvider {
    InstagramProvider::new(&state.config)
}

// ── Tool Implementations ─────────────────────────────────────

pub async fn handle_ig_get_media(
    state: &AppState,
    input: &IgGetMediaInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_instagram_token(state, user_id, &input.ig_id).await?;
    let provider = create_provider(state);
    let limit = input.limit.unwrap_or(20).min(100);
    let result = provider
        .get_ig_media(&token, &input.ig_id, limit)
        .await
        .map_err(|e| format!("Instagram get media failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_ig_get_media_detail(
    state: &AppState,
    input: &IgGetMediaDetailInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_instagram_token(state, user_id, &input.ig_id).await?;
    let provider = create_provider(state);
    let result = provider
        .get_ig_media_detail(&token, &input.media_id)
        .await
        .map_err(|e| format!("Instagram get media detail failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_ig_get_comments(
    state: &AppState,
    input: &IgGetCommentsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_instagram_token(state, user_id, &input.ig_id).await?;
    let provider = create_provider(state);
    let result = provider
        .get_ig_media_comments(&token, &input.media_id)
        .await
        .map_err(|e| format!("Instagram get comments failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_ig_search_hashtag(
    state: &AppState,
    input: &IgSearchHashtagInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_instagram_token(state, user_id, &input.ig_id).await?;
    let provider = create_provider(state);
    let result = provider
        .search_hashtag(&token, &input.ig_id, &input.query)
        .await
        .map_err(|e| {
            if e.to_string().contains("permission") || e.to_string().contains("OAuth") {
                format!("Instagram search hashtag failed: {e}. This tool requires 'Instagram Public Content Access' permission via Meta App Review.")
            } else {
                format!("Instagram search hashtag failed: {e}")
            }
        })?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_ig_get_hashtag_media(
    state: &AppState,
    input: &IgGetHashtagMediaInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_instagram_token(state, user_id, &input.ig_id).await?;
    let provider = create_provider(state);
    let result = provider
        .get_hashtag_media(&token, &input.hashtag_id)
        .await
        .map_err(|e| {
            if e.to_string().contains("permission") || e.to_string().contains("OAuth") {
                format!("Instagram get hashtag media failed: {e}. This tool requires 'Instagram Public Content Access' permission via Meta App Review.")
            } else {
                format!("Instagram get hashtag media failed: {e}")
            }
        })?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_ig_get_insights(
    state: &AppState,
    input: &IgGetInsightsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_instagram_token(state, user_id, &input.ig_id).await?;
    let provider = create_provider(state);
    let resolved_metric = match input.metric.trim() {
        "all" | "overview" => "reach,follower_count",
        "engagement" => "accounts_engaged,total_interactions",
        s if s.is_empty() => "reach,follower_count",
        s => s,
    };
    let valid_metrics: std::collections::HashSet<&str> = [
        "reach", "follower_count", "website_clicks", "profile_views",
        "online_followers", "accounts_engaged", "total_interactions",
        "likes", "comments", "shares", "saves", "replies",
        "engaged_audience_demographics", "reached_audience_demographics",
        "follower_demographics", "follows_and_unfollows",
        "profile_links_taps", "views", "threads_likes", "threads_replies",
        "reposts", "quotes", "threads_followers", "threads_follower_demographics",
        "content_views", "threads_views", "threads_clicks", "threads_reposts",
    ].into_iter().collect();
    let requested: Vec<&str> = resolved_metric.split(',').map(|s| s.trim()).collect();
    let invalid: Vec<_> = requested.iter().filter(|m| !valid_metrics.contains(*m)).collect();
    if !invalid.is_empty() {
        let invalid_str: String = invalid.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", ");
        return Err(format!(
            "Invalid metric(s): {}. Valid: reach, follower_count, website_clicks, \
             profile_views, online_followers, accounts_engaged, total_interactions, \
             likes, comments, shares, saves, replies, engaged/reached_audience_demographics, \
             follower_demographics, follows_and_unfollows, profile_links_taps, views, \
             threads_likes, threads_replies, reposts, quotes, threads_followers, \
             threads_follower_demographics, content_views, threads_views, threads_clicks, \
             threads_reposts. Tip: use 'impressions,reach,follower_count' for overview.",
            invalid_str
        ));
    }
    
    let mut all_results = serde_json::Map::new();
    for metric in requested {
        let period = if metric == "follower_count" {
            "day"
        } else {
            input.period.as_deref().unwrap_or("day")
        };
        
        let result = provider
            .get_ig_insights(&token, &input.ig_id, metric, period)
            .await
            .map_err(|e| format!("Instagram get insights failed for metric {metric}: {e}"))?;
        
        all_results.insert(metric.to_string(), result);
    }
    
    Ok(Json(serde_json::json!({ "data": all_results })))
}

pub async fn handle_ig_get_tagged(
    state: &AppState,
    input: &IgGetTaggedInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_instagram_token(state, user_id, &input.ig_id).await?;
    let provider = create_provider(state);
    let result = provider
        .get_ig_tagged(&token, &input.ig_id)
        .await
        .map_err(|e| format!("Instagram get tagged failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_ig_create_container(
    state: &AppState,
    input: &IgCreateContainerInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_instagram_token(state, user_id, &input.ig_id).await?;
    let provider = create_provider(state);
    let result = provider
        .create_ig_container(&token, &input.ig_id, &input.media_type, &input.media_url, &input.caption)
        .await
        .map_err(|e| format!("Instagram create container failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_ig_publish_container(
    state: &AppState,
    input: &IgPublishContainerInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_instagram_token(state, user_id, &input.ig_id).await?;
    let provider = create_provider(state);
    let result = provider
        .publish_ig_container(&token, &input.ig_id, &input.creation_id)
        .await
        .map_err(|e| format!("Instagram publish container failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_ig_reply_to_comment(
    state: &AppState,
    input: &IgReplyToCommentInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_instagram_token(state, user_id, &input.ig_id).await?;
    let provider = create_provider(state);
    let result = provider
        .reply_to_ig_comment(&token, &input.comment_id, &input.message)
        .await
        .map_err(|e| format!("Instagram reply to comment failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_ig_get_reels(
    state: &AppState,
    input: &IgGetReelsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_instagram_token(state, user_id, &input.ig_id).await?;
    let provider = create_provider(state);
    let result = provider
        .get_ig_reels(&token, &input.ig_id)
        .await
        .map_err(|e| format!("Instagram get reels failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_ig_get_stories(
    state: &AppState,
    input: &IgGetStoriesInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_instagram_token(state, user_id, &input.ig_id).await?;
    let provider = create_provider(state);
    let result = provider
        .get_ig_stories(&token, &input.ig_id)
        .await
        .map_err(|e| format!("Instagram get stories failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_ig_get_followers(
    state: &AppState,
    input: &IgGetFollowersInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_instagram_token(state, user_id, &input.ig_id).await?;
    let provider = create_provider(state);
    let result = provider
        .get_ig_followers(&token, &input.ig_id)
        .await
        .map_err(|e| format!("Instagram get followers failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_ig_business_discovery(
    state: &AppState,
    input: &IgBusinessDiscoveryInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_instagram_token(state, user_id, &input.ig_id).await?;
    let provider = create_provider(state);
    let result = provider
        .get_ig_business_discovery(&token, &input.ig_id, &input.target_username)
        .await
        .map_err(|e| format!("Instagram business discovery failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}


pub async fn handle_ig_get_insights_audience(
    state: &AppState,
    input: &IgGetInsightsAudienceInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_instagram_token(state, user_id, &input.ig_id).await?;
    let provider = create_provider(state);
    let result = provider
        .get_ig_insights_audience(&token, &input.ig_id)
        .await
        .map_err(|e| format!("Instagram get insights audience failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}
