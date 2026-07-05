// ─── MCP Pinterest Tools ──────────────────────────────────────────
// Pinterest API v5 tools for boards, pins, and analytics.
// Follows the same pattern as tools_instagram_standalone.rs.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::social::pinterest::PinterestProvider;

// ── Input/Output Types ──────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PiGetUserAccountInput {
    pub board_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PiGetBoardInput {
    pub board_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PiGetBoardPinsInput {
    pub board_id: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PiGetPinInput {
    pub board_id: String,
    pub pin_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PiGetBoardAnalyticsInput {
    pub board_id: String,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PiGetPinAnalyticsInput {
    pub board_id: String,
    pub pin_id: String,
    pub start_date: String,
    pub end_date: String,
}

// ── Helpers ──────────────────────────────────────────────────

async fn find_pi_token(state: &AppState, user_id: Uuid, board_id: &str) -> Result<String, String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    integrations
        .iter()
        .find(|i| i.provider_identifier == "pinterest" && i.internal_id == board_id)
        .map(|i| i.access_token.clone())
        .ok_or_else(|| {
            format!(
                "Pinterest board '{}' not connected. Connect it via the onboarding page first.",
                board_id
            )
        })
}

/// Find any Pinterest integration token (for create_pin where we don't
/// need a specific board_id to look up the token).
async fn find_pi_token_any(state: &AppState, user_id: Uuid) -> Result<String, String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;
    integrations
        .iter()
        .find(|i| i.provider_identifier == "pinterest")
        .map(|i| crate::crypto::maybe_decrypt_token(&i.access_token, state.token_key.as_ref()))
        .ok_or_else(|| "Pinterest account not connected. Connect it via the onboarding page first.".to_string())
}

fn create_pi_provider(state: &AppState) -> PinterestProvider {
    PinterestProvider::new(&state.config)
}

// ── Tool Implementations ─────────────────────────────────────

pub async fn handle_pi_get_user_account(
    state: &AppState,
    input: &PiGetUserAccountInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_pi_token(state, user_id, &input.board_id).await?;
    let provider = create_pi_provider(state);
    let result = provider
        .get_user_account(&token)
        .await
        .map_err(|e| format!("Pinterest get user account failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_pi_get_board(
    state: &AppState,
    input: &PiGetBoardInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_pi_token(state, user_id, &input.board_id).await?;
    let provider = create_pi_provider(state);
    let result = provider
        .get_board(&token, &input.board_id)
        .await
        .map_err(|e| format!("Pinterest get board failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_pi_get_board_pins(
    state: &AppState,
    input: &PiGetBoardPinsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_pi_token(state, user_id, &input.board_id).await?;
    let provider = create_pi_provider(state);
    let limit = input.limit.unwrap_or(25).min(100);
    let result = provider
        .get_board_pins(&token, &input.board_id, limit)
        .await
        .map_err(|e| format!("Pinterest get board pins failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_pi_get_pin(
    state: &AppState,
    input: &PiGetPinInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_pi_token(state, user_id, &input.board_id).await?;
    let provider = create_pi_provider(state);
    let result = provider
        .get_pin(&token, &input.pin_id)
        .await
        .map_err(|e| format!("Pinterest get pin failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_pi_get_board_analytics(
    state: &AppState,
    input: &PiGetBoardAnalyticsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_pi_token(state, user_id, &input.board_id).await?;
    let provider = create_pi_provider(state);
    let result = provider
        .get_board_analytics(&token, &input.board_id, &input.start_date, &input.end_date)
        .await
        .map_err(|e| format!("Pinterest get board analytics failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_pi_get_pin_analytics(
    state: &AppState,
    input: &PiGetPinAnalyticsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_pi_token(state, user_id, &input.board_id).await?;
    let provider = create_pi_provider(state);
    let result = provider
        .get_pin_analytics(&token, &input.pin_id, &input.start_date, &input.end_date)
        .await
        .map_err(|e| format!("Pinterest get pin analytics failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

// ── Search Pins ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PiSearchPinsInput {
    pub query: String,
    pub limit: Option<u32>,
}

pub async fn handle_pi_search_pins(
    state: &AppState,
    input: &PiSearchPinsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;
    let token = integrations
        .iter()
        .find(|i| i.provider_identifier == "pinterest")
        .ok_or_else(|| "No Pinterest integration found".to_string())?;
    let provider = create_pi_provider(state);
    let result = provider
        .search_pins(&token.access_token, &input.query, input.limit)
        .await
        .map_err(|e| format!("Pinterest search pins failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

// ── Create Pin ───────────────────────────────────────────────

use super::auth::resolve_first_user;
use crate::social::SocialProvider;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PiCreatePinInput {
    /// Board ID to pin to
    pub board_id: String,
    /// Pin title
    pub title: String,
    /// Pin description/text content
    pub content: String,
    /// Image URL(s) for the pin. At least 1 required. If the URL ends in .mp4, a video pin is created.
    #[serde(default)]
    pub media_urls: Vec<String>,
    /// Optional destination link
    pub link: Option<String>,
}

/// Create and publish a new Pinterest pin immediately. Supports image pins
/// (single or multiple images) and video pins (auto-detected from .mp4 URL).
pub async fn handle_pi_create_pin(
    state: &AppState,
    input: &PiCreatePinInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_first_user(state).await?;
    let token = find_pi_token_any(state, user_id).await?;
    let provider = create_pi_provider(state);

    let media: Vec<crate::social::MediaAttachment> = input
        .media_urls
        .iter()
        .map(|url| crate::social::MediaAttachment {
            url: url.clone(),
            mime_type: if url.ends_with(".mp4") { "video/mp4".into() } else { "image/jpeg".into() },
            alt: Some(input.title.clone()),
            poster_url: None,
        })
        .collect();

    let mut settings = serde_json::json!({
        "board": input.board_id,
        "title": input.title,
    });
    if let Some(ref link) = input.link {
        settings["link"] = serde_json::json!(link);
    }

    let post = crate::social::PostContent {
        content: input.content.clone(),
        media,
        settings,
    in_reply_to: None,
    };

    let result = provider
        .publish(&token, &post)
        .await
        .map_err(|e| format!("Pinterest create pin failed: {e}"))?;

    Ok(Json(serde_json::json!({
        "pin_id": result.platform_post_id,
        "url": result.platform_post_url,
        "status": result.status,
    })))
}
