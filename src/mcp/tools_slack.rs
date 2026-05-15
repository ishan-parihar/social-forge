// ─── MCP Slack Tools ──────────────────────────────────────────
// Slack Web API tools for sending messages, listing channels, reading history, and listing users.
// Uses OAuth user tokens stored in integrations table.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::api::AppState;
use crate::crypto;
use crate::social::slack::SlackProvider;
use crate::social::SocialProvider;

// ── Input Types ─────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SlSendMessageInput {
    /// Slack access token (optional — will use stored integration token if empty)
    pub token: String,
    /// Channel ID (e.g. "C01234ABCD") or channel name (e.g. "#general")
    pub channel: String,
    /// Message text content (up to 40,000 characters, supports mrkdwn)
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SlListChannelsInput {
    /// Slack access token (optional — will use stored integration token if empty)
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SlChannelHistoryInput {
    /// Slack access token (optional — will use stored integration token if empty)
    pub token: String,
    /// Channel ID (e.g. "C01234ABCD")
    pub channel: String,
    /// Maximum number of messages to return (1-200, default 50)
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SlListUsersInput {
    /// Slack access token (optional — will use stored integration token if empty)
    pub token: String,
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

    let __tok = integration.access_token.clone();
    let __tok = state.token_key.as_ref()
        .and_then(|k| crate::crypto::decrypt_string(&__tok, k).ok())
        .unwrap_or(__tok);
    Ok(__tok)
}

fn create_slack_provider(state: &AppState) -> SlackProvider {
    SlackProvider::new(&state.config)
}

/// Resolve the effective access token: prefer the user-provided token if non-empty,
/// otherwise fall back to the stored integration token.
fn resolve_token(
    provided: &str,
    stored_token: &str,
) -> String {
    if provided.is_empty() {
        stored_token.to_string()
    } else {
        provided.to_string()
    }
}

// ── Tool Implementations ─────────────────────────────────────

pub async fn handle_sl_send_message(
    state: &AppState,
    input: &SlSendMessageInput,
) -> Result<Json<Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let stored_token = find_slack_token(state, user_id).await?;
    let token = resolve_token(&input.token, &stored_token);

    let provider = create_slack_provider(state);

    let post = crate::social::PostContent {
        content: input.content.clone(),
        media: vec![],
        settings: json!({
            "channel": input.channel,
        }),
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
    let token = resolve_token(&input.token, &stored_token);

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
    let token = resolve_token(&input.token, &stored_token);

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
    let token = resolve_token(&input.token, &stored_token);

    let provider = create_slack_provider(state);
    let result = provider
        .get_user_list(&token)
        .await
        .map_err(|e| format!("Slack list users failed: {e}"))?;

    Ok(Json(json!({ "data": result })))
}
