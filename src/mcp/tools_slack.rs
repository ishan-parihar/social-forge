// ─── MCP Slack Tools ──────────────────────────────────────────
// Slack Web API tools for sending messages, listing channels, reading history, and listing users.
// Uses OAuth user tokens stored in integrations table.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::api::AppState;
use crate::social::slack::SlackProvider;
use crate::social::SocialProvider;

// ── Input Types ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SlSendMessageInput {
    /// Channel ID (e.g. "C01234ABCD") or channel name (e.g. "#general")
    pub channel: String,
    /// Message text content (up to 40,000 characters, supports mrkdwn)
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SlListChannelsInput {
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SlChannelHistoryInput {
    /// Channel ID (e.g. "C01234ABCD")
    pub channel: String,
    /// Maximum number of messages to return (1-200, default 50)
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SlListUsersInput {
}

// ── Helpers ──────────────────────────────────────────────────

/// Find a Slack integration for the given user and return its access token.
async fn find_slack_token(
    state: &AppState,
    user_id: Uuid,
) -> Result<String, String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let integration = integrations
        .iter()
        .find(|i| i.provider_identifier == "slack")
        .ok_or_else(|| {
            "Slack not connected. Connect Slack first via integrations_connect.".to_string()
        })?;

    let tok = crate::crypto::maybe_decrypt_token(&integration.access_token, state.token_key.as_ref());
    Ok(tok)
}

fn create_slack_provider(state: &AppState) -> SlackProvider {
    SlackProvider::new(&state.config)
}

// ── Tool Implementations ─────────────────────────────────────

pub async fn handle_sl_send_message(
    state: &AppState,
    input: &SlSendMessageInput,
) -> Result<Json<Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let stored_token = find_slack_token(state, user_id).await?;
    let token = stored_token;

    let provider = create_slack_provider(state);

    let post = crate::social::PostContent {
        content: input.content.clone(),
        media: vec![],
        settings: json!({
            "channel": input.channel,
        }),
    in_reply_to: None,
    };

    let result = provider
        .publish(&token, &post)
        .await
        .map_err(|e| format!("Slack send message failed: {e}"))?;

    Ok(Json(json!(result)))
}

pub async fn handle_sl_list_channels(
    state: &AppState,
    input: &SlListChannelsInput,
) -> Result<Json<Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let stored_token = find_slack_token(state, user_id).await?;
    let token = stored_token;

    let provider = create_slack_provider(state);
    let result = provider
        .get_channel_list(&token)
        .await
        .map_err(|e| format!("Slack list channels failed: {e}"))?;

    Ok(Json(json!({ "data": result })))
}

pub async fn handle_sl_channel_history(
    state: &AppState,
    input: &SlChannelHistoryInput,
) -> Result<Json<Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let stored_token = find_slack_token(state, user_id).await?;
    let token = stored_token;

    let provider = create_slack_provider(state);
    let limit = input.limit.unwrap_or(50);
    let result = provider
        .get_conversation_history(&token, &input.channel, limit)
        .await
        .map_err(|e| format!("Slack conversation history failed: {e}"))?;

    Ok(Json(json!({ "data": result })))
}

pub async fn handle_sl_list_users(
    state: &AppState,
    input: &SlListUsersInput,
) -> Result<Json<Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let stored_token = find_slack_token(state, user_id).await?;
    let token = stored_token;

    let provider = create_slack_provider(state);
    let result = provider
        .get_user_list(&token)
        .await
        .map_err(|e| format!("Slack list users failed: {e}"))?;

    Ok(Json(json!({ "data": result })))
}
