// ─── MCP LinkedIn Personal Tools ─────────────────────────────────
// LinkedIn personal profile tools via LinkedIn API v2.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::social::linkedin::LinkedInProvider;
use crate::social::SocialProvider;

// ── Input/Output Types ──────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LiGetProfileInput {
    pub user_id: Uuid,
    pub li_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LiGetPostsInput {
    pub user_id: Uuid,
    pub li_id: String,
    pub author_urn: String,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_limit() -> u32 {
    10
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LiGetPostDetailInput {
    pub user_id: Uuid,
    pub li_id: String,
    pub post_urn: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LiGetCommentsInput {
    pub user_id: Uuid,
    pub li_id: String,
    pub post_urn: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LiCreateCommentInput {
    pub user_id: Uuid,
    pub li_id: String,
    pub post_urn: String,
    pub message: String,
    pub actor_urn: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LiCreatePostInput {
    pub user_id: Uuid,
    pub li_id: String,
    pub content: String,
}

// ── Helpers ──────────────────────────────────────────────────

async fn find_linkedin_token(
    state: &AppState,
    user_id: Uuid,
    li_id: &str,
) -> Result<String, String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let li = integrations
        .iter()
        .find(|i| i.provider_identifier == "linkedin" && i.internal_id == li_id)
        .ok_or_else(|| {
            format!(
                "LinkedIn account '{li_id}' not connected. Connect it via the onboarding page first."
            )
        })?;

    let tok = crate::crypto::maybe_decrypt_token(&li.access_token, state.token_key.as_ref());
    Ok(tok)
}

fn create_provider(state: &AppState) -> LinkedInProvider {
    LinkedInProvider::new(&state.config)
}

// ── Tool Implementations ─────────────────────────────────────

pub async fn handle_li_get_profile(
    state: &AppState,
    input: &LiGetProfileInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_linkedin_token(state, user_id, &input.li_id).await?;
    let provider = create_provider(state);
    let result = provider
        .get_profile(&token)
        .await
        .map_err(|e| format!("LinkedIn get profile failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_li_get_posts(
    state: &AppState,
    input: &LiGetPostsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_linkedin_token(state, user_id, &input.li_id).await?;
    let provider = create_provider(state);
    let limit = input.limit.min(100);
    let result = provider
        .get_posts(&token, &input.author_urn, limit)
        .await
        .map_err(|e| format!("LinkedIn get posts failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_li_get_post_detail(
    state: &AppState,
    input: &LiGetPostDetailInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_linkedin_token(state, user_id, &input.li_id).await?;
    let provider = create_provider(state);
    let result = provider
        .get_post_detail(&token, &input.post_urn)
        .await
        .map_err(|e| format!("LinkedIn get post detail failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_li_get_comments(
    state: &AppState,
    input: &LiGetCommentsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_linkedin_token(state, user_id, &input.li_id).await?;
    let provider = create_provider(state);
    let result = provider
        .get_post_comments(&token, &input.post_urn)
        .await
        .map_err(|e| format!("LinkedIn get comments failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_li_create_comment(
    state: &AppState,
    input: &LiCreateCommentInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_linkedin_token(state, user_id, &input.li_id).await?;
    let provider = create_provider(state);
    let result = provider
        .create_comment(&token, &input.post_urn, &input.actor_urn, &input.message)
        .await
        .map_err(|e| format!("LinkedIn create comment failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_li_create_post(
    state: &AppState,
    input: &LiCreatePostInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_linkedin_token(state, user_id, &input.li_id).await?;
    let provider = create_provider(state);

    let post = crate::social::PostContent {
        content: input.content.clone(),
        media: vec![],
        settings: serde_json::Value::Object(serde_json::Map::new()),
    in_reply_to: None,
    idempotency_key: None,
    };

    let result = provider
        .publish(&token, &post)
        .await
        .map_err(|e| format!("LinkedIn create post failed: {e}"))?;

    Ok(Json(serde_json::json!({
        "data": {
            "id": result.platform_post_id,
            "url": result.platform_post_url,
            "status": result.status,
        }
    })))
}

// ─── LinkedIn Personal Delete Post ──────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LiDeletePostInput {
    pub user_id: Uuid,
    pub li_id: String,
    /// The post URN (e.g., urn:li:share:123456 or urn:li:ugcPost:123456)
    pub post_urn: String,
}

pub async fn handle_li_delete_post(
    state: &AppState,
    input: &LiDeletePostInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_linkedin_token(state, user_id, &input.li_id).await?;
    let provider = create_provider(state);
    provider.delete_post(&token, &input.post_urn).await
        .map_err(|e| format!("LinkedIn delete failed: {e}"))?;
    Ok(Json(serde_json::json!({"deleted": true, "post_urn": input.post_urn})))
}

// ─── LinkedIn Personal Get Reactions ────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LiGetReactionsInput {
    pub user_id: Uuid,
    pub li_id: String,
    pub post_urn: String,
}

pub async fn handle_li_get_reactions(
    state: &AppState,
    input: &LiGetReactionsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_linkedin_token(state, user_id, &input.li_id).await?;
    let provider = create_provider(state);
    let result = provider.get_reactions(&token, &input.post_urn).await
        .map_err(|e| format!("LinkedIn get reactions failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

// ─── LinkedIn Personal Get Shares ───────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LiGetSharesInput {
    pub user_id: Uuid,
    pub li_id: String,
    pub post_urn: String,
}

pub async fn handle_li_get_shares(
    state: &AppState,
    input: &LiGetSharesInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_linkedin_token(state, user_id, &input.li_id).await?;
    let provider = create_provider(state);
    let result = provider.get_shares(&token, &input.post_urn).await
        .map_err(|e| format!("LinkedIn get shares failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

// ─── LinkedIn Personal Analytics ────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LiGetAnalyticsInput {
    pub user_id: Uuid,
    pub li_id: String,
}

pub async fn handle_li_get_analytics(
    state: &AppState,
    input: &LiGetAnalyticsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_linkedin_token(state, user_id, &input.li_id).await?;
    let provider = create_provider(state);
    let data = provider.analytics(&token, &input.li_id, 30).await
        .map_err(|e| format!("LinkedIn analytics failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": data })))
}

// ─── LinkedIn Personal Post Analytics ───────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LiGetPostAnalyticsInput {
    pub user_id: Uuid,
    pub li_id: String,
    pub post_urn: String,
}

pub async fn handle_li_get_post_analytics(
    state: &AppState,
    input: &LiGetPostAnalyticsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_linkedin_token(state, user_id, &input.li_id).await?;
    let provider = create_provider(state);
    let data = provider.post_analytics(&token, &input.post_urn).await
        .map_err(|e| format!("LinkedIn post analytics failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": data })))
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LiReplyCommentInput {
    pub user_id: Uuid,
    pub li_id: String,
    pub comment_id: String,
    pub content: String,
}

pub async fn handle_li_reply_comment(
    state: &AppState,
    input: &LiReplyCommentInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_linkedin_token(state, user_id, &input.li_id).await?;
    let provider = create_provider(state);
    let post = crate::social::PostContent {
        content: input.content.clone(),
        media: vec![],
        settings: serde_json::json!({}),
    in_reply_to: None,
    idempotency_key: None,
    };
    let result = provider.reply_to_comment(&token, &input.comment_id, &post).await
        .map_err(|e| format!("LinkedIn reply comment failed: {e}"))?;
    Ok(Json(serde_json::json!({
        "data": {
            "id": result.platform_post_id,
            "status": result.status,
        }
    })))
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LiSendDmInput {
    pub user_id: Uuid,
    pub li_id: String,
    pub recipient_urn: String,
    pub content: String,
}

pub async fn handle_li_send_dm(
    state: &AppState,
    input: &LiSendDmInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_linkedin_token(state, user_id, &input.li_id).await?;
    let provider = create_provider(state);
    let post = crate::social::PostContent {
        content: input.content.clone(),
        media: vec![],
        settings: serde_json::json!({}),
    in_reply_to: None,
    idempotency_key: None,
    };
    let result = provider.send_dm(&token, &input.recipient_urn, &post).await
        .map_err(|e| format!("LinkedIn send DM failed: {e}"))?;
    Ok(Json(serde_json::json!({
        "data": {
            "id": result.platform_post_id,
            "status": result.status,
        }
    })))
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LiListConversationsInput {
    pub user_id: Uuid,
    pub li_id: String,
    pub limit: Option<u32>,
}

pub async fn handle_li_list_conversations(
    state: &AppState,
    input: &LiListConversationsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_linkedin_token(state, user_id, &input.li_id).await?;
    let provider = create_provider(state);
    let limit = input.limit.unwrap_or(20);
    let conversations = provider.get_dm_conversations(&token, limit).await
        .map_err(|e| format!("LinkedIn list conversations failed: {e}"))?;
    let conv_values: Vec<serde_json::Value> = conversations.into_iter().map(|c| {
        serde_json::json!({
            "id": c.id,
            "participant": c.participant,
            "last_message": c.last_message,
            "last_message_at": c.last_message_at.map(|dt| dt.to_rfc3339()),
        })
    }).collect();
    Ok(Json(serde_json::json!({ "data": conv_values })))
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LiGetMessagesInput {
    pub user_id: Uuid,
    pub li_id: String,
    pub conversation_id: String,
    pub limit: Option<u32>,
}

pub async fn handle_li_get_messages(
    state: &AppState,
    input: &LiGetMessagesInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_linkedin_token(state, user_id, &input.li_id).await?;
    let provider = create_provider(state);
    let limit = input.limit.unwrap_or(20);
    let messages = provider.get_dm_messages(&token, &input.conversation_id, limit).await
        .map_err(|e| format!("LinkedIn get messages failed: {e}"))?;
    let msg_values: Vec<serde_json::Value> = messages.into_iter().map(|m| {
        serde_json::json!({
            "id": m.id,
            "sender": m.sender,
            "content": m.content,
            "created_at": m.created_at.to_rfc3339(),
        })
    }).collect();
    Ok(Json(serde_json::json!({ "data": msg_values })))
}
