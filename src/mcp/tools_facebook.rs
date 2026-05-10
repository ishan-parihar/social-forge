// ─── MCP Facebook Tools ──────────────────────────────────────────
// Facebook Page management tools via Meta Graph API.
// Uses page-scoped tokens resolved from connected Facebook integrations.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::social::facebook::FacebookProvider;

// ── Input Types ───────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FbGetFeedInput {
    pub page_id: String,
    pub limit: Option<u32>,
    pub since: Option<String>,
    pub until: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FbGetPostInput {
    pub page_id: String,
    pub post_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FbGetCommentsInput {
    pub page_id: String,
    pub post_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FbCreatePostInput {
    pub page_id: String,
    pub message: String,
    pub link: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FbCreatePhotoInput {
    pub page_id: String,
    pub url: String,
    pub caption: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FbCreateVideoInput {
    pub page_id: String,
    pub file_url: String,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FbDeletePostInput {
    pub page_id: String,
    pub post_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FbCommentInput {
    pub page_id: String,
    pub post_id: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FbReactInput {
    pub page_id: String,
    pub post_id: String,
    pub reaction_type: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FbPageInsightsInput {
    pub page_id: String,
    pub metric: String,
    pub period: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FbConversationsInput {
    pub page_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FbConversationMsgsInput {
    pub page_id: String,
    pub conversation_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FbSendMessageInput {
    pub page_id: String,
    pub conversation_id: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FbSearchPagesInput {
    pub query: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FbAlbumsInput {
    pub page_id: String,
}

// ── Helpers ──────────────────────────────────────────────────

/// Find the Facebook user-level integration (parent, no root_internal_id set).
async fn find_facebook_token(state: &AppState, user_id: Uuid) -> Result<String, String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let fb = integrations
        .iter()
        .find(|i| i.provider_identifier == "facebook" && i.root_internal_id.is_none())
        .ok_or_else(|| {
            "No Facebook account connected. Use the onboarding page first.".to_string()
        })?;

    Ok(fb.access_token.clone())
}

/// Find a page-scoped Facebook token by page_id.
async fn find_page_token(state: &AppState, user_id: Uuid, page_id: &str) -> Result<String, String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let page = integrations
        .iter()
        .find(|i| i.provider_identifier == "facebook" && i.internal_id == page_id)
        .ok_or_else(|| {
            format!(
                "Facebook page '{page_id}' not connected. Use available-pages to connect it first."
            )
        })?;

    Ok(page.access_token.clone())
}

fn create_provider(state: &AppState) -> FacebookProvider {
    FacebookProvider::new(&state.config)
}

// ── Tool Implementations ─────────────────────────────────────

pub async fn handle_fb_get_feed(
    state: &AppState,
    input: &FbGetFeedInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_page_token(state, user_id, &input.page_id).await?;
    let provider = create_provider(state);
    let result = provider
        .get_page_feed(
            &token,
            &input.page_id,
            input.limit.unwrap_or(20),
            input.since.as_deref(),
            input.until.as_deref(),
        )
        .await
        .map_err(|e| format!("Facebook get feed failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_fb_get_post(
    state: &AppState,
    input: &FbGetPostInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_page_token(state, user_id, &input.page_id).await?;
    let provider = create_provider(state);
    let result = provider
        .get_page_post(&token, &input.post_id)
        .await
        .map_err(|e| format!("Facebook get post failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_fb_get_comments(
    state: &AppState,
    input: &FbGetCommentsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_page_token(state, user_id, &input.page_id).await?;
    let provider = create_provider(state);
    let result = provider
        .get_post_comments(&token, &input.post_id)
        .await
        .map_err(|e| format!("Facebook get comments failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_fb_create_post(
    state: &AppState,
    input: &FbCreatePostInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_page_token(state, user_id, &input.page_id).await?;
    let provider = create_provider(state);
    let result = provider
        .create_post(&token, &input.page_id, &input.message, input.link.as_deref())
        .await
        .map_err(|e| format!("Facebook create post failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_fb_create_photo(
    state: &AppState,
    input: &FbCreatePhotoInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_page_token(state, user_id, &input.page_id).await?;
    let provider = create_provider(state);
    let result = provider
        .create_photo_post(&token, &input.page_id, &input.url, &input.caption.as_deref().unwrap_or(""))
        .await
        .map_err(|e| format!("Facebook create photo failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_fb_create_video(
    state: &AppState,
    input: &FbCreateVideoInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_page_token(state, user_id, &input.page_id).await?;
    let provider = create_provider(state);
    let result = provider
        .create_video_post(
            &token,
            &input.page_id,
            &input.file_url,
            input.title.as_deref().unwrap_or(""),
            input.description.as_deref().unwrap_or(""),
        )
        .await
        .map_err(|e| format!("Facebook create video failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_fb_delete_post(
    state: &AppState,
    input: &FbDeletePostInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_page_token(state, user_id, &input.page_id).await?;
    let provider = create_provider(state);
    let result = provider
        .delete_post(&token, &input.post_id)
        .await
        .map_err(|e| format!("Facebook delete post failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_fb_comment(
    state: &AppState,
    input: &FbCommentInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_page_token(state, user_id, &input.page_id).await?;
    let provider = create_provider(state);
    let result = provider
        .comment_on_post(&token, &input.post_id, &input.message)
        .await
        .map_err(|e| format!("Facebook comment failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_fb_react(
    state: &AppState,
    input: &FbReactInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_page_token(state, user_id, &input.page_id).await?;
    let provider = create_provider(state);
    let result = provider
        .react_to_post(&token, &input.post_id, &input.reaction_type)
        .await
        .map_err(|e| format!("Facebook react failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_fb_page_insights(
    state: &AppState,
    input: &FbPageInsightsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_page_token(state, user_id, &input.page_id).await?;
    let provider = create_provider(state);
    
    let metrics_str = if input.metric.is_empty() {
        "page_post_engagements"
    } else {
        &input.metric
    };
    
    let metrics: Vec<&str> = metrics_str.split(',').map(|s| s.trim()).collect();
    let mut all_results = serde_json::Map::new();
    
    for metric in metrics {
        if metric.is_empty() { continue; }
        let result = provider
            .get_page_insights(
                &token,
                &input.page_id,
                metric,
                input.period.as_deref().unwrap_or("week"),
                input.since.as_deref(),
                input.until.as_deref(),
            )
            .await
            .map_err(|e| format!("Facebook page insights failed for metric {metric}: {e}"))?;
        
        all_results.insert(metric.to_string(), result);
    }
    
    Ok(Json(serde_json::json!({ "data": all_results })))
}

pub async fn handle_fb_conversations(
    state: &AppState,
    input: &FbConversationsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_page_token(state, user_id, &input.page_id).await?;
    let provider = create_provider(state);
    let result = provider
        .get_page_conversations(&token, &input.page_id)
        .await
        .map_err(|e| format!("Facebook conversations failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_fb_conversation_msgs(
    state: &AppState,
    input: &FbConversationMsgsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_page_token(state, user_id, &input.page_id).await?;
    let provider = create_provider(state);
    let result = provider
        .get_conversation_messages(&token, &input.conversation_id)
        .await
        .map_err(|e| format!("Facebook conversation messages failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_fb_send_message(
    state: &AppState,
    input: &FbSendMessageInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_page_token(state, user_id, &input.page_id).await?;
    let provider = create_provider(state);
    let result = provider
        .send_message(&token, &input.conversation_id, &input.message)
        .await
        .map_err(|e| format!("Facebook send message failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_fb_search_pages(
    state: &AppState,
    input: &FbSearchPagesInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_facebook_token(state, user_id).await?;
    let provider = create_provider(state);
    let result = provider
        .search_pages(&token, &input.query)
        .await
        .map_err(|e| format!("Facebook search pages failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_fb_albums(
    state: &AppState,
    input: &FbAlbumsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_page_token(state, user_id, &input.page_id).await?;
    let provider = create_provider(state);
    let result = provider
        .get_page_albums(&token, &input.page_id)
        .await
        .map_err(|e| format!("Facebook albums failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}
