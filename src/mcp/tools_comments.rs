// ─── MCP Comment Tools ─────────────────────────────────────────
// Generic comment tools that work across platforms.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetCommentsInput {
    pub integration_id: String,
    pub post_id: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CommentInfo {
    pub id: String,
    pub author_name: Option<String>,
    pub text: String,
    pub created_at: String,
    pub like_count: i32,
    pub reply_count: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetCommentsOutput {
    pub comments: Vec<CommentInfo>,
    pub total: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReplyToCommentInput {
    pub integration_id: String,
    pub comment_id: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReplyToCommentOutput {
    pub post_id: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DeleteCommentInput {
    pub integration_id: String,
    pub comment_id: String,
}

pub async fn get_comments(
    state: &AppState,
    input: &GetCommentsInput,
) -> Result<Json<GetCommentsOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let integration_id = Uuid::parse_str(&input.integration_id)
        .map_err(|_| "Invalid integration_id format")?;

    let integration = crate::db::queries::get_integration(&state.db, integration_id, user_id)
        .await
        .map_err(|e| format!("Integration not found: {e}"))?
        .ok_or_else(|| "Integration not found".to_string())?;

    let provider = state.providers.get(&integration.provider_identifier)
        .ok_or_else(|| format!("Provider {} not found", integration.provider_identifier))?;

    let token = crate::crypto::maybe_decrypt_token(&integration.access_token, state.token_key.as_ref());
    let limit = input.limit.unwrap_or(50);

    let comments = provider.get_post_comments(&token, &input.post_id)
        .await
        .map_err(|e| format!("Failed to get comments: {e}"))?;

    let comment_infos: Vec<CommentInfo> = comments.into_iter().take(limit as usize).map(|c| CommentInfo {
        id: c.id,
        author_name: c.author_name,
        text: c.text,
        created_at: c.created_at.to_rfc3339(),
        like_count: c.like_count,
        reply_count: c.replies.len(),
    }).collect();

    let total = comment_infos.len();

    Ok(Json(GetCommentsOutput { comments: comment_infos, total }))
}

pub async fn reply_to_comment(
    state: &AppState,
    input: &ReplyToCommentInput,
) -> Result<Json<ReplyToCommentOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let integration_id = Uuid::parse_str(&input.integration_id)
        .map_err(|_| "Invalid integration_id format")?;

    let integration = crate::db::queries::get_integration(&state.db, integration_id, user_id)
        .await
        .map_err(|e| format!("Integration not found: {e}"))?
        .ok_or_else(|| "Integration not found".to_string())?;

    let provider = state.providers.get(&integration.provider_identifier)
        .ok_or_else(|| format!("Provider {} not found", integration.provider_identifier))?;

    let token = crate::crypto::maybe_decrypt_token(&integration.access_token, state.token_key.as_ref());

    let post_content = crate::social::PostContent {
        content: input.content.clone(),
        media: vec![],
        settings: serde_json::json!({}),
    };

    let result = provider.reply_to_comment(&token, &input.comment_id, &post_content)
        .await
        .map_err(|e| format!("Failed to reply to comment: {e}"))?;

    Ok(Json(ReplyToCommentOutput {
        post_id: result.platform_post_id,
        status: result.status,
    }))
}

pub async fn delete_comment(
    state: &AppState,
    input: &DeleteCommentInput,
) -> Result<Json<crate::mcp::McpJsonValue>, String> {
    let user_id = resolve_first_user(state).await?;
    let integration_id = Uuid::parse_str(&input.integration_id)
        .map_err(|_| "Invalid integration_id format")?;

    let integration = crate::db::queries::get_integration(&state.db, integration_id, user_id)
        .await
        .map_err(|e| format!("Integration not found: {e}"))?
        .ok_or_else(|| "Integration not found".to_string())?;

    let provider = state.providers.get(&integration.provider_identifier)
        .ok_or_else(|| format!("Provider {} not found", integration.provider_identifier))?;

    let token = crate::crypto::maybe_decrypt_token(&integration.access_token, state.token_key.as_ref());

    provider.delete_comment(&token, &input.comment_id)
        .await
        .map_err(|e| format!("Failed to delete comment: {e}"))?;

    Ok(Json(crate::mcp::McpJsonValue(serde_json::json!({"success": true, "message": "Comment deleted",}))))
}

use super::auth::resolve_first_user;
