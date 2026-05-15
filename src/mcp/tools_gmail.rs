// ─── MCP Gmail Tools ──────────────────────────────────────────
// Gmail API v1 tools via GmailProvider.
// Uses Google OAuth (reusing YOUTUBE_CLIENT_ID credentials).

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::crypto;
use crate::social::gmail::GmailProvider;

// ── Input Types ───────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GmListMessagesInput {
    /// Max results (default 20, max 500)
    pub max_results: Option<u32>,
    /// Gmail search query (optional, e.g. "from:someone@example.com")
    pub query: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GmGetMessageInput {
    /// Gmail message ID
    pub message_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GmSendMessageInput {
    /// Recipient email address
    pub to: String,
    pub subject: String,
    pub body: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GmGetThreadInput {
    /// Gmail thread ID
    pub thread_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GmSearchMessagesInput {
    /// Gmail search query
    pub query: String,
    /// Max results (default 20, max 500)
    pub max_results: Option<u32>,
}

// ── Helpers ───────────────────────────────────────────────────

async fn find_gm_token(state: &AppState, user_id: Uuid) -> Result<String, String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let gm = integrations
        .iter()
        .find(|i| i.provider_identifier == "gmail")
        .ok_or_else(|| "Gmail not connected. Connect it via the onboarding page first.".to_string())?;

    let tok = gm.access_token.clone();
    let tok = state.token_key.as_ref()
        .and_then(|k| crypto::decrypt_string(&tok, k).ok())
        .unwrap_or(tok);
    Ok(tok)
}

fn create_gm_provider(state: &AppState) -> GmailProvider {
    GmailProvider::new(&state.config)
}

// ── Handlers ──────────────────────────────────────────────────

pub async fn handle_gm_get_profile(
    state: &AppState,
    _input: &(),
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gm_token(state, user_id).await?;
    let provider = create_gm_provider(state);
    let result = provider
        .get_profile(&token)
        .await
        .map_err(|e| format!("Gmail get profile failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gm_list_messages(
    state: &AppState,
    input: &GmListMessagesInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gm_token(state, user_id).await?;
    let provider = create_gm_provider(state);
    let max_results = input.max_results.unwrap_or(20);
    let result = provider
        .list_messages(&token, max_results, input.query.as_deref())
        .await
        .map_err(|e| format!("Gmail list messages failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gm_get_message(
    state: &AppState,
    input: &GmGetMessageInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gm_token(state, user_id).await?;
    let provider = create_gm_provider(state);
    let result = provider
        .get_message(&token, &input.message_id)
        .await
        .map_err(|e| format!("Gmail get message failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gm_send_message(
    state: &AppState,
    input: &GmSendMessageInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gm_token(state, user_id).await?;
    let provider = create_gm_provider(state);
    let result = provider
        .send_message(&token, &input.to, &input.subject, &input.body)
        .await
        .map_err(|e| format!("Gmail send message failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gm_list_labels(
    state: &AppState,
    _input: &(),
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gm_token(state, user_id).await?;
    let provider = create_gm_provider(state);
    let result = provider
        .list_labels(&token)
        .await
        .map_err(|e| format!("Gmail list labels failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gm_get_thread(
    state: &AppState,
    input: &GmGetThreadInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gm_token(state, user_id).await?;
    let provider = create_gm_provider(state);
    let result = provider
        .get_thread(&token, &input.thread_id)
        .await
        .map_err(|e| format!("Gmail get thread failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gm_search_messages(
    state: &AppState,
    input: &GmSearchMessagesInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gm_token(state, user_id).await?;
    let provider = create_gm_provider(state);
    let max_results = input.max_results.unwrap_or(20);
    let result = provider
        .search_messages(&token, &input.query, max_results)
        .await
        .map_err(|e| format!("Gmail search messages failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}
