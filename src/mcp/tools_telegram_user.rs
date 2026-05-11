use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};

use crate::api::AppState;

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

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TuRequestCodeInput {
    pub phone: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TuRequestCodeOutput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TuSignInInput {
    pub phone: String,
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TuSignInOutput {
    pub data: serde_json::Value,
}

fn client_manager(state: &AppState) -> Result<&crate::services::telegram_client::TelegramClientManager, String> {
    state
        .telegram_client_manager
        .as_ref()
        .map(|arc| arc.as_ref())
        .ok_or_else(|| {
            "Telegram user client not configured. Set TELEGRAM_API_ID and TELEGRAM_API_HASH.".to_string()
        })
}

pub async fn handle_tu_auth_status(
    state: &AppState,
) -> Result<Json<TuAuthStatusOutput>, String> {
    let mgr = client_manager(state)?;
    let is_auth = mgr.is_authenticated().await?;
    let user = if is_auth {
        mgr.user_info().await.ok()
    } else {
        None
    };
    Ok(Json(TuAuthStatusOutput {
        data: serde_json::json!({
            "authenticated": is_auth,
            "user": user,
        }),
    }))
}

pub async fn handle_tu_send_message(
    state: &AppState,
    input: &TuSendMessageInput,
) -> Result<Json<TuSendMessageOutput>, String> {
    let mgr = client_manager(state)?;
    let result = mgr.send_message(&input.peer, &input.text).await?;
    Ok(Json(TuSendMessageOutput { data: result }))
}

pub async fn handle_tu_list_dialogs(
    state: &AppState,
    _input: &TuListDialogsInput,
) -> Result<Json<TuListDialogsOutput>, String> {
    let mgr = client_manager(state)?;
    let result = mgr.list_dialogs().await?;
    Ok(Json(TuListDialogsOutput { data: result }))
}

pub async fn handle_tu_list_contacts(
    state: &AppState,
    _input: &TuListContactsInput,
) -> Result<Json<TuListContactsOutput>, String> {
    let mgr = client_manager(state)?;
    let result = mgr.list_contacts().await?;
    Ok(Json(TuListContactsOutput { data: result }))
}

pub async fn handle_tu_search(
    state: &AppState,
    input: &TuSearchInput,
) -> Result<Json<TuSearchOutput>, String> {
    let mgr = client_manager(state)?;
    let result = mgr.search(&input.query).await?;
    Ok(Json(TuSearchOutput { data: result }))
}

pub async fn handle_tu_request_code(
    state: &AppState,
    input: &TuRequestCodeInput,
) -> Result<Json<TuRequestCodeOutput>, String> {
    let mgr = client_manager(state)?;
    let result = mgr.request_login_code(&input.phone).await?;
    Ok(Json(TuRequestCodeOutput { data: result }))
}

pub async fn handle_tu_sign_in(
    state: &AppState,
    input: &TuSignInInput,
) -> Result<Json<TuSignInOutput>, String> {
    let mgr = client_manager(state)?;
    let result = mgr.sign_in(&input.phone, &input.code).await?;
    Ok(Json(TuSignInOutput { data: result }))
}
