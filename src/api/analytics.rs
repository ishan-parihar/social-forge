// ─── Analytics API Routes ─────────────────────────────────────
// Dashboard analytics for connected social providers.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::error::AppError;

use super::AppState;

// ── Query Types ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub provider: String,
    pub days: Option<i32>,
}

// ── Handlers ─────────────────────────────────────────────────

/// GET /api/analytics
pub async fn get(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let days = query.days.unwrap_or(7).max(1) as u32;

    let integrations = crate::db::queries::list_integrations(&state.db, auth.user_id)
        .await?;

    let integration = integrations
        .iter()
        .find(|i| i.provider_identifier == query.provider)
        .ok_or_else(|| AppError::NotFound(format!("Provider '{}' not connected", query.provider)))?;

    let provider = state
        .providers
        .get(&query.provider)
        .ok_or_else(|| AppError::NotFound(format!("Provider '{}' not found", query.provider)))?;

    let analytics = provider
        .analytics(&integration.access_token, &integration.internal_id, days)
        .await
        .map_err(AppError::from)?;

    Ok(Json(serde_json::json!({ "data": analytics })))
}

/// GET /api/analytics/post/{id}
pub async fn get_post(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let post = crate::db::queries::get_post(&state.db, id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".into()))?;

    let platform_post_id = post
        .platform_post_id
        .ok_or_else(|| AppError::BadRequest("Post has not been published yet".into()))?;

    let integration = crate::db::queries::get_integration(
        &state.db,
        post.integration_id,
        auth.user_id,
    )
    .await?
    .ok_or_else(|| AppError::NotFound("Integration not found".into()))?;

    let provider = state
        .providers
        .get(&integration.provider_identifier)
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "Provider '{}' not found",
                integration.provider_identifier
            ))
        })?;

    let analytics = provider
        .post_analytics(&integration.access_token, &platform_post_id)
        .await
        .map_err(AppError::from)?;

    Ok(Json(serde_json::json!({ "data": analytics })))
}
