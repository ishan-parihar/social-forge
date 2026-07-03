// ─── MCP DM Tools ──────────────────────────────────────────────
// Generic DM tools that work across platforms.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SendDmInput {
    pub integration_id: String,
    pub recipient: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SendDmOutput {
    pub message_id: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListDmInput {
    pub integration_id: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DmConversationInfo {
    pub id: String,
    pub participant: String,
    pub participant_name: Option<String>,
    pub last_message: Option<String>,
    pub last_message_at: Option<String>,
    pub unread_count: u32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListDmOutput {
    pub conversations: Vec<DmConversationInfo>,
    pub total: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetDmInput {
    pub integration_id: String,
    pub conversation_id: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DmMessageInfo {
    pub id: String,
    pub sender: String,
    pub sender_name: Option<String>,
    pub content: String,
    pub created_at: String,
    pub read: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetDmOutput {
    pub messages: Vec<DmMessageInfo>,
    pub total: usize,
}

pub async fn send_dm(
    state: &AppState,
    input: &SendDmInput,
) -> Result<Json<SendDmOutput>, String> {
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

    let result = provider.send_dm(&token, &input.recipient, &post_content)
        .await
        .map_err(|e| format!("Failed to send DM: {e}"))?;

    Ok(Json(SendDmOutput {
        message_id: result.platform_post_id,
        status: result.status,
    }))
}

pub async fn list_dm_conversations(
    state: &AppState,
    input: &ListDmInput,
) -> Result<Json<ListDmOutput>, String> {
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

    let conversations = provider.get_dm_conversations(&token, limit)
        .await
        .map_err(|e| format!("Failed to list DM conversations: {e}"))?;

    let conv_infos: Vec<DmConversationInfo> = conversations.into_iter().map(|c| DmConversationInfo {
        id: c.id,
        participant: c.participant,
        participant_name: c.participant_name,
        last_message: c.last_message,
        last_message_at: c.last_message_at.map(|dt| dt.to_rfc3339()),
        unread_count: c.unread_count,
    }).collect();

    let total = conv_infos.len();

    Ok(Json(ListDmOutput { conversations: conv_infos, total }))
}

pub async fn get_dm_messages(
    state: &AppState,
    input: &GetDmInput,
) -> Result<Json<GetDmOutput>, String> {
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

    let messages = provider.get_dm_messages(&token, &input.conversation_id, limit)
        .await
        .map_err(|e| format!("Failed to get DM messages: {e}"))?;

    let msg_infos: Vec<DmMessageInfo> = messages.into_iter().map(|m| DmMessageInfo {
        id: m.id,
        sender: m.sender,
        sender_name: m.sender_name,
        content: m.content,
        created_at: m.created_at.to_rfc3339(),
        read: m.read,
    }).collect();

    let total = msg_infos.len();

    Ok(Json(GetDmOutput { messages: msg_infos, total }))
}

use super::auth::resolve_first_user;
