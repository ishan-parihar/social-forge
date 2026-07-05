// ─── DMs API Routes ──────────────────────────────────────────
// Direct message operations: list conversations, read messages, send DMs.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::queries;
use crate::error::AppError;
use crate::social::{MediaAttachment, PostContent};

use super::AppState;

// ── Request Types ───────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListConversationsQuery {
    pub integration_id: Uuid,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

#[derive(Debug, Deserialize)]
pub struct GetMessagesQuery {
    #[serde(default = "default_limit")]
    pub limit: u32,
}

#[derive(Debug, Deserialize)]
pub struct SendDmRequest {
    pub integration_id: Uuid,
    pub recipient: String,
    pub content: String,
    /// Optional media attachments
    #[serde(default)]
    pub media: Vec<MediaAttachment>,
}

fn default_limit() -> u32 {
    50
}

// ── Response Types ──────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ConversationResponse {
    pub id: String,
    pub participant: String,
    pub participant_name: Option<String>,
    pub participant_avatar: Option<String>,
    pub last_message: Option<String>,
    pub last_message_at: Option<String>,
    pub unread_count: u32,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub id: String,
    pub conversation_id: String,
    pub sender: String,
    pub sender_name: Option<String>,
    pub content: String,
    pub media: Vec<MediaAttachment>,
    pub created_at: String,
    pub read: bool,
}

#[derive(Debug, Serialize)]
pub struct ListConversationsResponse {
    pub conversations: Vec<ConversationResponse>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct GetMessagesResponse {
    pub messages: Vec<MessageResponse>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct SendDmResponse {
    pub message_id: String,
    pub status: String,
}

// ── Handlers ────────────────────────────────────────────────

/// GET /api/dms/conversations?integration_id=X&limit=50
pub async fn list_conversations(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<ListConversationsQuery>,
) -> Result<Json<ListConversationsResponse>, AppError> {
    let integration = queries::get_integration(&state.db, query.integration_id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Integration not found".into()))?;

    let provider = state
        .providers
        .get(&integration.provider_identifier)
        .ok_or_else(|| {
            AppError::BadRequest(format!("Provider {} not found", integration.provider_identifier))
        })?;

    let access_token = resolve_access_token(&state, &integration.access_token);

    let conversations = provider
        .get_dm_conversations(&access_token, query.limit)
        .await
        .map_err(AppError::from)?;

    let conv_responses: Vec<ConversationResponse> = conversations
        .into_iter()
        .map(|c| ConversationResponse {
            id: c.id,
            participant: c.participant,
            participant_name: c.participant_name,
            participant_avatar: c.participant_avatar,
            last_message: c.last_message,
            last_message_at: c.last_message_at.map(|dt| dt.to_rfc3339()),
            unread_count: c.unread_count,
        })
        .collect();

    let total = conv_responses.len();

    Ok(Json(ListConversationsResponse {
        conversations: conv_responses,
        total,
    }))
}

/// GET /api/dms/{conversation_id}/messages?limit=50
pub async fn get_messages(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(conversation_id): Path<String>,
    Query(query): Query<GetMessagesQuery>,
) -> Result<Json<GetMessagesResponse>, AppError> {
    // Find any integration for this user (DMs are provider-specific, but we need a token)
    let integrations = queries::list_integrations(&state.db, auth.user_id).await?;
    let integration = integrations
        .into_iter()
        .find(|i| !i.disabled)
        .ok_or_else(|| AppError::NotFound("No active integration found".into()))?;

    let provider = state
        .providers
        .get(&integration.provider_identifier)
        .ok_or_else(|| {
            AppError::BadRequest(format!("Provider {} not found", integration.provider_identifier))
        })?;

    let access_token = resolve_access_token(&state, &integration.access_token);

    let messages = provider
        .get_dm_messages(&access_token, &conversation_id, query.limit)
        .await
        .map_err(AppError::from)?;

    let msg_responses: Vec<MessageResponse> = messages
        .into_iter()
        .map(|m| MessageResponse {
            id: m.id,
            conversation_id: m.conversation_id,
            sender: m.sender,
            sender_name: m.sender_name,
            content: m.content,
            media: m.media,
            created_at: m.created_at.to_rfc3339(),
            read: m.read,
        })
        .collect();

    let total = msg_responses.len();

    Ok(Json(GetMessagesResponse {
        messages: msg_responses,
        total,
    }))
}

/// POST /api/dms/send
pub async fn send_dm(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(request): Json<SendDmRequest>,
) -> Result<Json<SendDmResponse>, AppError> {
    let integration = queries::get_integration(&state.db, request.integration_id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Integration not found".into()))?;

    let provider = state
        .providers
        .get(&integration.provider_identifier)
        .ok_or_else(|| {
            AppError::BadRequest(format!("Provider {} not found", integration.provider_identifier))
        })?;

    let access_token = resolve_access_token(&state, &integration.access_token);

    let content = PostContent {
        content: request.content,
        media: request.media,
        settings: serde_json::json!({}),
    in_reply_to: None,
    };

    let result = provider
        .send_dm(&access_token, &request.recipient, &content)
        .await
        .map_err(AppError::from)?;

    Ok(Json(SendDmResponse {
        message_id: result.platform_post_id,
        status: result.status,
    }))
}

// ── Helpers ─────────────────────────────────────────────────

/// Resolve access token: try decryption with token_key, fall back to raw.
fn resolve_access_token(state: &AppState, token: &str) -> String {
    state
        .token_key
        .as_ref()
        .and_then(|key| crate::crypto::decrypt_string(token, key).ok())
        .unwrap_or_else(|| token.to_string())
}
