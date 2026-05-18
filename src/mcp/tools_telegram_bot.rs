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

// ─── Additional Telegram Bot Tools ──────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TbTokenInput { pub token_index: usize }

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TbChatInput { pub token_index: usize, pub chat_id: String }

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TbSendPhotoInput { pub token_index: usize, pub chat_id: String, pub photo_url: String, pub caption: Option<String> }

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TbSendDocumentInput { pub token_index: usize, pub chat_id: String, pub document_url: String, pub caption: Option<String> }

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TbForwardInput { pub token_index: usize, pub chat_id: String, pub from_chat_id: String, pub message_id: i64 }

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TbPinInput { pub token_index: usize, pub chat_id: String, pub message_id: i64 }

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TbApiOutput { pub data: serde_json::Value }

async fn tb_call(state: &AppState, token_index: usize, method: &str, body: serde_json::Value) -> Result<Json<TbApiOutput>, String> {
    let tokens = get_telegram_bot_tokens(state);
    let token = tokens.get(token_index)
        .ok_or_else(|| format!("Token index {} out of range ({} bots)", token_index, tokens.len()))?;
    let resp = reqwest::Client::new()
        .post(format!("https://api.telegram.org/bot{token}/{method}"))
        .json(&body).send().await
        .map_err(|e| format!("Request failed: {e}"))?;
    let json: serde_json::Value = resp.json().await.map_err(|e| format!("Parse failed: {e}"))?;
    if json["ok"].as_bool().unwrap_or(false) {
        Ok(Json(TbApiOutput { data: json }))
    } else {
        Err(format!("Telegram API: {}", json["description"].as_str().unwrap_or("unknown error")))
    }
}

pub async fn handle_tb_get_me(state: &AppState, input: &TbTokenInput) -> Result<Json<TbApiOutput>, String> {
    tb_call(state, input.token_index, "getMe", serde_json::json!({})).await
}

pub async fn handle_tb_get_chat(state: &AppState, input: &TbChatInput) -> Result<Json<TbApiOutput>, String> {
    tb_call(state, input.token_index, "getChat", serde_json::json!({"chat_id": input.chat_id})).await
}

pub async fn handle_tb_get_chat_member_count(state: &AppState, input: &TbChatInput) -> Result<Json<TbApiOutput>, String> {
    tb_call(state, input.token_index, "getChatMemberCount", serde_json::json!({"chat_id": input.chat_id})).await
}

pub async fn handle_tb_send_photo(state: &AppState, input: &TbSendPhotoInput) -> Result<Json<TbApiOutput>, String> {
    let mut body = serde_json::json!({"chat_id": input.chat_id, "photo": input.photo_url});
    if let Some(c) = &input.caption { body["caption"] = serde_json::json!(c); }
    tb_call(state, input.token_index, "sendPhoto", body).await
}

pub async fn handle_tb_send_document(state: &AppState, input: &TbSendDocumentInput) -> Result<Json<TbApiOutput>, String> {
    let mut body = serde_json::json!({"chat_id": input.chat_id, "document": input.document_url});
    if let Some(c) = &input.caption { body["caption"] = serde_json::json!(c); }
    tb_call(state, input.token_index, "sendDocument", body).await
}

pub async fn handle_tb_forward_message(state: &AppState, input: &TbForwardInput) -> Result<Json<TbApiOutput>, String> {
    tb_call(state, input.token_index, "forwardMessage", serde_json::json!({
        "chat_id": input.chat_id, "from_chat_id": input.from_chat_id, "message_id": input.message_id
    })).await
}

pub async fn handle_tb_pin_message(state: &AppState, input: &TbPinInput) -> Result<Json<TbApiOutput>, String> {
    tb_call(state, input.token_index, "pinChatMessage", serde_json::json!({
        "chat_id": input.chat_id, "message_id": input.message_id
    })).await
}

pub async fn handle_tb_unpin_message(state: &AppState, input: &TbPinInput) -> Result<Json<TbApiOutput>, String> {
    tb_call(state, input.token_index, "unpinChatMessage", serde_json::json!({
        "chat_id": input.chat_id, "message_id": input.message_id
    })).await
}
