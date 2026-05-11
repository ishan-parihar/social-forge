use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};

use crate::api::AppState;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaAuthStatusOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaSendTextInput {
    pub to: String,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaSendTextOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaChatsInput {
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaChatsOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaContactsInput {
    pub query: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaContactsOutput {
    pub data: serde_json::Value,
}

async fn find_whatsapp_daemon(state: &AppState) -> Result<std::sync::Arc<crate::services::whatsapp_daemon::WhatsAppDaemon>, String> {
    let store_dir = state
        .config
        .whatsapp_store_dir
        .clone()
        .unwrap_or_else(|| "./data/whatsapp".into());
    crate::services::whatsapp_daemon::WhatsAppDaemon::start(std::path::PathBuf::from(store_dir))
}

pub async fn handle_wa_auth_status(
    state: &AppState,
) -> Result<Json<WaAuthStatusOutput>, String> {
    let daemon = find_whatsapp_daemon(state).await?;
    let result = daemon
        .auth_status()
        .map_err(|e| format!("WhatsApp auth status failed: {e}"))?;
    Ok(Json(WaAuthStatusOutput { data: result }))
}

pub async fn handle_wa_send_text(
    state: &AppState,
    input: &WaSendTextInput,
) -> Result<Json<WaSendTextOutput>, String> {
    let daemon = find_whatsapp_daemon(state).await?;
    let result = daemon
        .send_text(&input.to, &input.text)
        .map_err(|e| format!("WhatsApp send failed: {e}"))?;
    Ok(Json(WaSendTextOutput { data: result }))
}

pub async fn handle_wa_chats(
    state: &AppState,
    input: &WaChatsInput,
) -> Result<Json<WaChatsOutput>, String> {
    let daemon = find_whatsapp_daemon(state).await?;
    let result = daemon
        .list_chats(input.limit.map(|l| l as u64), None)
        .map_err(|e| format!("WhatsApp list chats failed: {e}"))?;
    Ok(Json(WaChatsOutput { data: result }))
}

pub async fn handle_wa_contacts(
    state: &AppState,
    input: &WaContactsInput,
) -> Result<Json<WaContactsOutput>, String> {
    let daemon = find_whatsapp_daemon(state).await?;
    let result = daemon
        .list_contacts(input.limit.map(|l| l as u64), input.query.clone())
        .map_err(|e| format!("WhatsApp list contacts failed: {e}"))?;
    Ok(Json(WaContactsOutput { data: result }))
}
