// ─── MCP Google Drive Tools ───────────────────────────────────
// Google Drive API v3 tools via DriveProvider.
// Uses Google OAuth (reusing YOUTUBE_CLIENT_ID credentials).

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::crypto;
use crate::social::drive::DriveProvider;

// ── Input Types ───────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DrListFilesInput {
    /// Max results (default 20, max 100)
    pub max_results: Option<u32>,
    /// Optional MIME type filter (e.g. "application/pdf", "text/plain")
    pub mime_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DrGetFileInput {
    pub file_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DrSearchFilesInput {
    /// Search query (e.g. "name contains 'report'")
    pub query: String,
    /// Max results (default 20, max 100)
    pub max_results: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DrListFoldersInput {
    /// Max results (default 50)
    pub max_results: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DrGetFileMetadataInput {
    pub file_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DrExportFileInput {
    pub file_id: String,
    /// Target MIME type (e.g. "application/pdf", "text/plain")
    pub mime_type: String,
}

// ── Helpers ───────────────────────────────────────────────────

async fn find_dr_token(state: &AppState, user_id: Uuid) -> Result<String, String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let dr = integrations
        .iter()
        .find(|i| i.provider_identifier == "drive")
        .ok_or_else(|| "Google Drive not connected. Connect it via the onboarding page first.".to_string())?;

    let tok = dr.access_token.clone();
    let tok = state.token_key.as_ref()
        .and_then(|k| crypto::decrypt_string(&tok, k).ok())
        .unwrap_or(tok);
    Ok(tok)
}

fn create_dr_provider(state: &AppState) -> DriveProvider {
    DriveProvider::new(&state.config)
}

// ── Handlers ──────────────────────────────────────────────────

pub async fn handle_dr_list_files(
    state: &AppState,
    input: &DrListFilesInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_dr_token(state, user_id).await?;
    let provider = create_dr_provider(state);
    let max_results = input.max_results.unwrap_or(20);
    let result = provider
        .list_files(&token, max_results, input.mime_type.as_deref())
        .await
        .map_err(|e| format!("Drive list files failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_dr_get_file(
    state: &AppState,
    input: &DrGetFileInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_dr_token(state, user_id).await?;
    let provider = create_dr_provider(state);
    let result = provider
        .get_file(&token, &input.file_id)
        .await
        .map_err(|e| format!("Drive get file failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_dr_search_files(
    state: &AppState,
    input: &DrSearchFilesInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_dr_token(state, user_id).await?;
    let provider = create_dr_provider(state);
    let max_results = input.max_results.unwrap_or(20);
    let result = provider
        .search_files(&token, &input.query, max_results)
        .await
        .map_err(|e| format!("Drive search files failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_dr_list_folders(
    state: &AppState,
    input: &DrListFoldersInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_dr_token(state, user_id).await?;
    let provider = create_dr_provider(state);
    let max_results = input.max_results.unwrap_or(50);
    let result = provider
        .list_folders(&token, max_results)
        .await
        .map_err(|e| format!("Drive list folders failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_dr_get_file_metadata(
    state: &AppState,
    input: &DrGetFileMetadataInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_dr_token(state, user_id).await?;
    let provider = create_dr_provider(state);
    let result = provider
        .get_file_metadata(&token, &input.file_id)
        .await
        .map_err(|e| format!("Drive get file metadata failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_dr_export_file(
    state: &AppState,
    input: &DrExportFileInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_dr_token(state, user_id).await?;
    let provider = create_dr_provider(state);
    let result = provider
        .export_file(&token, &input.file_id, &input.mime_type)
        .await
        .map_err(|e| format!("Drive export file failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}
