use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};

use crate::api::AppState;
use crate::services::telegram_daemon::TelegramDaemon;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TuAuthStatusOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TuSendMessageInput {
    pub peer: String,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TuSendMessageOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TuListDialogsInput {
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TuListDialogsOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TuListContactsInput {
    pub query: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TuListContactsOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TuSearchInput {
    pub query: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TuSearchOutput {
    pub data: serde_json::Value,
}

async fn find_telegram_daemon() -> Result<Box<TelegramDaemon>, String> {
    TelegramDaemon::start()
        .map_err(|e| format!("Failed to start Telegram daemon: {}", e))
}

pub async fn handle_tu_auth_status(
    _state: &AppState,
) -> Result<Json<TuAuthStatusOutput>, String> {
    let daemon = find_telegram_daemon().await?;
    let result = daemon
        .auth_status()
        .map_err(|e| format!("Telegram auth status failed: {}", e))?;
    Ok(Json(TuAuthStatusOutput { data: result }))
}

pub async fn handle_tu_send_message(
    _state: &AppState,
    input: &TuSendMessageInput,
) -> Result<Json<TuSendMessageOutput>, String> {
    let daemon = find_telegram_daemon().await?;
    let result = daemon
        .send_message(&input.peer, &input.text)
        .map_err(|e| format!("Telegram send failed: {}", e))?;
    Ok(Json(TuSendMessageOutput { data: result }))
}

pub async fn handle_tu_list_dialogs(
    _state: &AppState,
    _input: &TuListDialogsInput,
) -> Result<Json<TuListDialogsOutput>, String> {
    let daemon = find_telegram_daemon().await?;
    let result = daemon
        .list_dialogs()
        .map_err(|e| format!("Telegram list dialogs failed: {}", e))?;
    Ok(Json(TuListDialogsOutput { data: result }))
}

pub async fn handle_tu_list_contacts(
    _state: &AppState,
    _input: &TuListContactsInput,
) -> Result<Json<TuListContactsOutput>, String> {
    let daemon = find_telegram_daemon().await?;
    let result = daemon
        .list_contacts()
        .map_err(|e| format!("Telegram list contacts failed: {}", e))?;
    Ok(Json(TuListContactsOutput { data: result }))
}

pub async fn handle_tu_search(
    _state: &AppState,
    input: &TuSearchInput,
) -> Result<Json<TuSearchOutput>, String> {
    let daemon = find_telegram_daemon().await?;
    let result = daemon
        .search(&input.query)
        .map_err(|e| format!("Telegram search failed: {}", e))?;
    Ok(Json(TuSearchOutput { data: result }))
}