use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::api::AppState;
use crate::wa::{WhaClient, chats, messages};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaAuthStatusOutput {
    pub authenticated: bool,
    pub jid: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaSendTextInput {
    pub to: String,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaSendTextOutput {
    pub message_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaChatsInput {
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaChatsOutput {
    pub data: Vec<chats::ChatSummary>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaContactsInput {
    pub query: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaContactsOutput {
    pub data: Vec<chats::ContactEntry>,
}

/// Get the native WhaClient guard, or error.
fn get_wa_client<'a>(
    state: &'a AppState,
) -> Result<&'a Arc<Mutex<WhaClient>>, String> {
    state
        .wa_client
        .as_ref()
        .ok_or_else(|| "WhatsApp client not configured (set WHATSAPP_STORE_DIR)".to_string())
}

pub async fn handle_wa_auth_status(
    state: &AppState,
) -> Result<Json<WaAuthStatusOutput>, String> {
    let client = get_wa_client(state)?;
    let locked = client.lock().await;
    let authenticated = locked.is_authenticated();
    let jid = locked
        .inner()
        .get_pn()
        .await
        .map(|j| j.to_string());
    Ok(Json(WaAuthStatusOutput { authenticated, jid }))
}

pub async fn handle_wa_send_text(
    state: &AppState,
    input: &WaSendTextInput,
) -> Result<Json<WaSendTextOutput>, String> {
    let client = get_wa_client(state)?;
    let jid = wa_rs::Jid::pn(&input.to);
    let msg_id = messages::send_text(client, &jid, &input.text)
        .await
        .map_err(|e| format!("WhatsApp send failed: {e}"))?;
    Ok(Json(WaSendTextOutput { message_id: msg_id }))
}

pub async fn handle_wa_chats(
    state: &AppState,
    input: &WaChatsInput,
) -> Result<Json<WaChatsOutput>, String> {
    let client = get_wa_client(state)?;
    let result = chats::list_chats(client, input.limit)
        .await
        .map_err(|e| format!("WhatsApp list chats failed: {e}"))?;
    Ok(Json(WaChatsOutput { data: result }))
}

pub async fn handle_wa_contacts(
    state: &AppState,
    input: &WaContactsInput,
) -> Result<Json<WaContactsOutput>, String> {
    let client = get_wa_client(state)?;
    let result = chats::list_contacts(client, input.limit)
        .await
        .map_err(|e| format!("WhatsApp list contacts failed: {e}"))?;
    Ok(Json(WaContactsOutput { data: result }))
}
