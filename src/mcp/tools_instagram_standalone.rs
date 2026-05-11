// ─── MCP Instagram Standalone Tools ──────────────────────────────
// Instagram Basic Display API tools via graph.instagram.com/v21.0.
// Follows the same pattern as tools_instagram.rs.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::social::instagram_standalone::InstagramStandaloneProvider;

// ── Input/Output Types ──────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IasGetMediaInput {
    pub ig_id: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IasGetMediaDetailInput {
    pub ig_id: String,
    pub media_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IasGetCommentsInput {
    pub ig_id: String,
    pub media_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IasReplyToCommentInput {
    pub ig_id: String,
    pub comment_id: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IasCreateContainerInput {
    pub ig_id: String,
    pub media_type: String,
    pub media_url: String,
    pub caption: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IasPublishContainerInput {
    pub ig_id: String,
    pub creation_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IasPollContainerInput {
    pub ig_id: String,
    pub creation_id: String,
}

// ── Helpers ──────────────────────────────────────────────────

async fn find_ias_token(state: &AppState, user_id: Uuid, ig_id: &str) -> Result<String, String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let ig = integrations
        .iter()
        .find(|i| i.provider_identifier == "instagram-standalone" && i.internal_id == ig_id)
        .ok_or_else(|| {
            format!(
                "Instagram Standalone account '{}' not connected. Connect it via the onboarding page first.",
                ig_id
            )
        })?;

    Ok(ig.access_token.clone())
}

fn create_provider(state: &AppState) -> InstagramStandaloneProvider {
    InstagramStandaloneProvider::new(&state.config)
}

// ── Tool Implementations ─────────────────────────────────────

pub async fn handle_ias_get_media(
    state: &AppState,
    input: &IasGetMediaInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_ias_token(state, user_id, &input.ig_id).await?;
    let provider = create_provider(state);
    let limit = input.limit.unwrap_or(20).min(100);
    let result = provider
        .get_media(&token, &input.ig_id, limit)
        .await
        .map_err(|e| format!("Instagram Standalone get media failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_ias_get_media_detail(
    state: &AppState,
    input: &IasGetMediaDetailInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_ias_token(state, user_id, &input.ig_id).await?;
    let provider = create_provider(state);
    let result = provider
        .get_media_detail(&token, &input.media_id)
        .await
        .map_err(|e| format!("Instagram Standalone get media detail failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_ias_get_comments(
    state: &AppState,
    input: &IasGetCommentsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_ias_token(state, user_id, &input.ig_id).await?;
    let provider = create_provider(state);
    let result = provider
        .get_media_comments(&token, &input.media_id)
        .await
        .map_err(|e| format!("Instagram Standalone get comments failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_ias_reply_to_comment(
    state: &AppState,
    input: &IasReplyToCommentInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_ias_token(state, user_id, &input.ig_id).await?;
    let provider = create_provider(state);
    let result = provider
        .reply_to_comment(&token, &input.comment_id, &input.message)
        .await
        .map_err(|e| format!("Instagram Standalone reply to comment failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_ias_create_container(
    state: &AppState,
    input: &IasCreateContainerInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_ias_token(state, user_id, &input.ig_id).await?;
    let provider = create_provider(state);
    let result = provider
        .create_container(&token, &input.ig_id, &input.media_url, &input.caption, &input.media_type)
        .await
        .map_err(|e| format!("Instagram Standalone create container failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_ias_publish_container(
    state: &AppState,
    input: &IasPublishContainerInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_ias_token(state, user_id, &input.ig_id).await?;
    let provider = create_provider(state);
    let result = provider
        .publish_container(&token, &input.ig_id, &input.creation_id)
        .await
        .map_err(|e| format!("Instagram Standalone publish container failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_ias_poll_container(
    state: &AppState,
    input: &IasPollContainerInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_ias_token(state, user_id, &input.ig_id).await?;
    let provider = create_provider(state);
    let result = provider
        .poll_container_status(&token, &input.creation_id)
        .await
        .map_err(|e| format!("Instagram Standalone poll container failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}