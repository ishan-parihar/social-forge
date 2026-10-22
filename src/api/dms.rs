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
    /// Required: which integration's token to use for fetching messages.
    /// DMs are provider-specific, so we must use the user-selected integration's
    /// access token — not just any active integration's.
    pub integration_id: Uuid,
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
    pub platform: String,
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

/// X's DM API (`api.twitter.com/2/dm_conversations`) is
/// BEARER-ONLY: it authenticates with an app-scoped OAuth 2.0 bearer
/// token. A cookie-auth X channel (auth_token + ct0) has no bearer,
/// so the call is made with a string that is not a bearer token - and
/// X answers 404 "Not Found", which the error classifier maps to
/// "post or user not found, it may be deleted or protected". That is
/// a MISDIAGNOSIS the founder cannot act on: the channel's cookies
/// are fine, the DMs surface just does not exist for cookie auth.
/// Refuse with the real reason instead.
fn x_cookie_auth_dm_refusal(integration: &crate::db::models::Integration) -> Option<AppError> {
    if integration.provider_identifier == "x" && integration.auth_method == "cookie" {
        Some(AppError::BadRequest(
            "X direct messages need an OAuth-connected channel. Cookie-auth channels \
             (auth_token + ct0) cannot use X's DM API."
                .into(),
        ))
    } else {
        None
    }
}

/// GET /api/dms/conversations?integration_id=X&limit=50
pub async fn list_conversations(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<ListConversationsQuery>,
) -> Result<Json<ListConversationsResponse>, AppError> {
    let integration = queries::get_integration(&state.db, query.integration_id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Integration not found".into()))?;

    if let Some(refusal) = x_cookie_auth_dm_refusal(&integration) {
        return Err(refusal);
    }

    if let Some(refusal) = x_cookie_auth_dm_refusal(&integration) {
        return Err(refusal);
    }

    let provider = state
        .providers
        .get(&integration.provider_identifier)
        .ok_or_else(|| {
            AppError::BadRequest(format!(
                "Provider {} not found",
                integration.provider_identifier
            ))
        })?;

    let access_token =
        crate::crypto::maybe_decrypt_token(&integration.access_token, state.token_key.as_ref());

    let conversations = provider
        .get_dm_conversations(&access_token, query.limit)
        .await
        .map_err(AppError::from)?;

    let conv_responses: Vec<ConversationResponse> = conversations
        .into_iter()
        .map(|c| ConversationResponse {
            id: c.id,
            platform: integration.provider_identifier.clone(),
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

/// GET /api/dms/{conversation_id}/messages?integration_id=X&limit=50
pub async fn get_messages(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(conversation_id): Path<String>,
    Query(query): Query<GetMessagesQuery>,
) -> Result<Json<GetMessagesResponse>, AppError> {
    // Use the user-selected integration's token — DMs are provider-specific
    // and using "any" integration (as the previous code did) would leak
    // messages from a different platform's account.
    let integration = queries::get_integration(&state.db, query.integration_id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Integration not found".into()))?;

    let provider = state
        .providers
        .get(&integration.provider_identifier)
        .ok_or_else(|| {
            AppError::BadRequest(format!(
                "Provider {} not found",
                integration.provider_identifier
            ))
        })?;

    let access_token =
        crate::crypto::maybe_decrypt_token(&integration.access_token, state.token_key.as_ref());

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

    if let Some(refusal) = x_cookie_auth_dm_refusal(&integration) {
        return Err(refusal);
    }

    let provider = state
        .providers
        .get(&integration.provider_identifier)
        .ok_or_else(|| {
            AppError::BadRequest(format!(
                "Provider {} not found",
                integration.provider_identifier
            ))
        })?;

    let access_token =
        crate::crypto::maybe_decrypt_token(&integration.access_token, state.token_key.as_ref());

    let content = PostContent {
        content: request.content,
        media: request.media,
        settings: serde_json::json!({}),
        ..Default::default()
    };

    let result = provider
        .send_dm(&access_token, &request.recipient, &content)
        .await
        .map_err(AppError::from)?;

    // Broadcast so the /dms view (and any other open tab) reloads the
    // conversation list / thread after a message goes out.
    //
    // Only the send path broadcasts. DMs are fetched live from the provider
    // on every GET (there is no DM cache table and no background DM poller),
    // so emitting from `list_conversations` / `get_messages` would loop: the
    // /dms page's `dm_received` handler re-fetches exactly those endpoints.
    // `send_dm` is a one-shot, non-idempotent state change — the only place a
    // DM event can originate without that feedback loop.
    state.broadcast.send(
        "dm_received",
        &serde_json::json!({ "id": result.platform_post_id }),
    );

    Ok(Json(SendDmResponse {
        message_id: result.platform_post_id,
        status: result.status,
    }))
}
