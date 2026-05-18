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

// ─── Additional WhatsApp MCP Tools ───────────────────────────────

use crate::wa::groups;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaEditMessageInput {
    pub to: String,
    pub message_id: String,
    pub new_text: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaEditMessageOutput {
    pub message_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaRevokeMessageInput {
    pub to: String,
    pub message_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaRevokeMessageOutput {
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaListGroupsOutput {
    pub groups: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaCreateGroupInput {
    pub subject: String,
    pub participants: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaCreateGroupOutput {
    pub group_jid: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaGroupInviteLinkInput {
    pub group_jid: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaGroupInviteLinkOutput {
    pub invite_link: String,
}

pub async fn handle_wa_edit_message(
    state: &AppState,
    input: &WaEditMessageInput,
) -> Result<Json<WaEditMessageOutput>, String> {
    let client = get_wa_client(state)?;
    let jid = wa_rs::Jid::pn(&input.to);
    let msg_id = messages::edit_message(client, &jid, &input.message_id, &input.new_text)
        .await
        .map_err(|e| format!("WhatsApp edit failed: {e}"))?;
    Ok(Json(WaEditMessageOutput { message_id: msg_id }))
}

pub async fn handle_wa_revoke_message(
    state: &AppState,
    input: &WaRevokeMessageInput,
) -> Result<Json<WaRevokeMessageOutput>, String> {
    let client = get_wa_client(state)?;
    let jid = wa_rs::Jid::pn(&input.to);
    messages::revoke_message(client, &jid, &input.message_id)
        .await
        .map_err(|e| format!("WhatsApp revoke failed: {e}"))?;
    Ok(Json(WaRevokeMessageOutput { success: true }))
}

pub async fn handle_wa_list_groups(
    state: &AppState,
) -> Result<Json<WaListGroupsOutput>, String> {
    let client = get_wa_client(state)?;
    let result = groups::list_groups(client)
        .await
        .map_err(|e| format!("WhatsApp list groups failed: {e}"))?;
    Ok(Json(WaListGroupsOutput { groups: result }))
}

pub async fn handle_wa_create_group(
    state: &AppState,
    input: &WaCreateGroupInput,
) -> Result<Json<WaCreateGroupOutput>, String> {
    let client = get_wa_client(state)?;
    let result = groups::create_group(client, &input.subject, &input.participants)
        .await
        .map_err(|e| format!("WhatsApp create group failed: {e}"))?;
    Ok(Json(WaCreateGroupOutput { group_jid: result.gid.to_string() }))
}

pub async fn handle_wa_group_invite_link(
    state: &AppState,
    input: &WaGroupInviteLinkInput,
) -> Result<Json<WaGroupInviteLinkOutput>, String> {
    let client = get_wa_client(state)?;
    let jid: wa_rs::Jid = input.group_jid.parse()
        .map_err(|e| format!("Invalid group JID: {e}"))?;
    let link = groups::get_group_invite_link(client, &jid)
        .await
        .map_err(|e| format!("WhatsApp invite link failed: {e}"))?;
    Ok(Json(WaGroupInviteLinkOutput { invite_link: link }))
}
