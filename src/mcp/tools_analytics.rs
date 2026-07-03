// ─── MCP Analytics Tools ───────────────────────────────────────
// Exposes provider analytics and per-post analytics as MCP tools.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};

use crate::api::AppState;

// ── Input Types ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AnalyticsGetInput {
    /// JWT auth token
    pub token: String,
    /// Provider identifier (e.g. "instagram", "facebook", "threads")
    pub provider: String,
    /// Number of days of analytics (default 7)
    pub days: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AnalyticsPostInput {
    /// JWT auth token
    pub token: String,
    /// Post ID (UUID from our system, not platform)
    pub post_id: String,
}

// ── Handlers ─────────────────────────────────────────────────

/// Get analytics for a connected social provider
pub async fn handle_analytics_get(
    state: &AppState,
    input: &AnalyticsGetInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let days = input.days.unwrap_or(7).max(1) as u32;

    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let integration = integrations
        .iter()
        .find(|i| i.provider_identifier == input.provider)
        .ok_or_else(|| {
            format!(
                "Provider '{}' not connected. Connect it via the onboarding page first.",
                input.provider
            )
        })?;

    let provider = state
        .providers
        .get(&input.provider)
        .ok_or_else(|| format!("Provider '{}' not found", input.provider))?;

    let tok = crate::crypto::maybe_decrypt_token(&integration.access_token, state.token_key.as_ref());
    let analytics = provider
        .analytics(&tok, &integration.internal_id, days)
        .await
        .map_err(|e| format!("Analytics request failed: {e}"))?;

    Ok(Json(serde_json::json!({ "data": analytics })))
}

/// Get analytics for a specific published post
pub async fn handle_analytics_get_post(
    state: &AppState,
    input: &AnalyticsPostInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let post_id = uuid::Uuid::parse_str(&input.post_id)
        .map_err(|_| format!("Invalid post ID: {}", input.post_id))?;

    let post = crate::db::queries::get_post(&state.db, post_id, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?
        .ok_or_else(|| "Post not found".to_string())?;

    let platform_post_id = post
        .platform_post_id
        .ok_or_else(|| "Post has not been published yet".to_string())?;

    let integration = crate::db::queries::get_integration(
        &state.db,
        post.integration_id,
        user_id,
    )
    .await
    .map_err(|e| format!("DB error: {e}"))?
    .ok_or_else(|| "Integration not found".to_string())?;

    let provider = state
        .providers
        .get(&integration.provider_identifier)
        .ok_or_else(|| format!("Provider '{}' not found", integration.provider_identifier))?;

    let tok = crate::crypto::maybe_decrypt_token(&integration.access_token, state.token_key.as_ref());
    let analytics = provider
        .post_analytics(&tok, &platform_post_id)
        .await
        .map_err(|e| format!("Post analytics request failed: {e}"))?;

    Ok(Json(serde_json::json!({ "data": analytics })))
}
