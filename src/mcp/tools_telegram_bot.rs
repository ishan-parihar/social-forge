// ─── Telegram Bot MCP Tools ─────────────────────────────────────
// Multi-account Telegram Bot API tools via config tokens.
// Uses Bot API directly with token-based authentication.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};

use crate::api::AppState;

/// Input for sending a message via Telegram Bot
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TbSendMessageInput {
    pub token_index: usize,
    pub chat_id: String,
    pub text: String,
}

/// Output from sending a Telegram Bot message
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TbSendMessageOutput {
    pub data: serde_json::Value,
}

/// Input for getting Telegram Bot updates
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TbGetUpdatesInput {
    pub token_index: usize,
}

/// Output from getting Telegram Bot updates
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TbGetUpdatesOutput {
    pub data: serde_json::Value,
}

/// Get Telegram bot tokens from app config
fn get_telegram_bot_tokens(state: &AppState) -> Vec<String> {
    state
        .config
        .telegram_bot_tokens
        .as_ref()
        .map(|s| s.split(',').map(|t| t.trim().to_string()).collect())
        .unwrap_or_default()
}

/// Handler for sending a message via Telegram Bot API
pub async fn handle_tb_send_message(
    state: &AppState,
    input: &TbSendMessageInput,
) -> Result<Json<TbSendMessageOutput>, String> {
    let tokens = get_telegram_bot_tokens(state);

    if input.token_index >= tokens.len() {
        return Err(format!(
            "Token index {} out of range. Found {} Telegram bots.",
            input.token_index,
            tokens.len()
        ));
    }

    let token = &tokens[input.token_index];
    let http = reqwest::Client::new();

    let url = format!("https://api.telegram.org/bot{}/sendMessage", token);

    let response = http
        .post(&url)
        .json(&serde_json::json!({
            "chat_id": input.chat_id,
            "text": input.text,
            "parse_mode": "HTML"
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to send message: {}", e))?;

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if json.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        Ok(Json(TbSendMessageOutput { data: json }))
    } else {
        let error_msg = json
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        Err(format!("Telegram API error: {}", error_msg))
    }
}

/// Handler for getting updates from Telegram Bot API
pub async fn handle_tb_get_updates(
    state: &AppState,
    input: &TbGetUpdatesInput,
) -> Result<Json<TbGetUpdatesOutput>, String> {
    let tokens = get_telegram_bot_tokens(state);

    if input.token_index >= tokens.len() {
        return Err(format!(
            "Token index {} out of range. Found {} Telegram bots.",
            input.token_index,
            tokens.len()
        ));
    }

    let token = &tokens[input.token_index];
    let http = reqwest::Client::new();

    let url = format!("https://api.telegram.org/bot{}/getUpdates", token);

    let response = http
        .post(&url)
        .json(&serde_json::json!({
            "allowed_updates": ["message", "edited_message", "callback_query"]
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to get updates: {}", e))?;

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if json.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        Ok(Json(TbGetUpdatesOutput { data: json }))
    } else {
        let error_msg = json
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        Err(format!("Telegram API error: {}", error_msg))
    }
}
