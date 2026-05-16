// ─── Analytics API Routes ─────────────────────────────────────
// Dashboard analytics for connected social providers.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
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

// ── Aggregated Summary ────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AnalyticsSummaryQuery {
    pub days: Option<i32>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProviderCount {
    pub provider: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct DayCount {
    pub date: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsSummaryResponse {
    pub total_posts: i64,
    pub published: i64,
    pub failed: i64,
    pub draft: i64,
    pub queued: i64,
    pub best_provider: Option<ProviderCount>,
    pub posts_by_provider: Vec<ProviderCount>,
    pub posts_by_day: Vec<DayCount>,
}

/// GET /api/analytics/summary?days=30
pub async fn get_summary(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<AnalyticsSummaryQuery>,
) -> Result<Json<AnalyticsSummaryResponse>, AppError> {
    let days = query.days.unwrap_or(30).max(1);
    let cutoff = chrono::Utc::now() - chrono::Duration::days(days as i64);

    // Count posts by state
    let state_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT state::text, COUNT(*)::bigint FROM posts WHERE user_id = $1 AND created_at >= $2 GROUP BY state",
    )
    .bind(auth.user_id)
    .bind(cutoff)
    .fetch_all(&state.db)
    .await?;

    // Count posts by provider (JOIN integrations for provider_name)
    let provider_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT i.provider_name, COUNT(*)::bigint \
         FROM posts p JOIN integrations i ON p.integration_id = i.id \
         WHERE p.user_id = $1 AND p.created_at >= $2 \
         GROUP BY i.provider_name \
         ORDER BY COUNT(*)::bigint DESC",
    )
    .bind(auth.user_id)
    .bind(cutoff)
    .fetch_all(&state.db)
    .await?;

    // Count posts by day
    let day_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT DATE(p.created_at)::text, COUNT(*)::bigint \
         FROM posts p WHERE p.user_id = $1 AND p.created_at >= $2 \
         GROUP BY DATE(p.created_at) \
         ORDER BY DATE(p.created_at) ASC",
    )
    .bind(auth.user_id)
    .bind(cutoff)
    .fetch_all(&state.db)
    .await?;

    let mut published = 0i64;
    let mut failed = 0i64;
    let mut draft = 0i64;
    let mut queued = 0i64;

    for (s, count) in &state_rows {
        match s.as_str() {
            "published" => published = *count,
            "error" => failed = *count,
            "draft" => draft = *count,
            "queued" => queued = *count,
            _ => {}
        }
    }

    let total_posts = published + failed + draft + queued;

    let posts_by_provider: Vec<ProviderCount> = provider_rows
        .into_iter()
        .map(|(provider, count)| ProviderCount { provider, count })
        .collect();

    let best_provider = posts_by_provider.first().cloned();

    let posts_by_day: Vec<DayCount> = day_rows
        .into_iter()
        .map(|(date, count)| DayCount { date, count })
        .collect();

    Ok(Json(AnalyticsSummaryResponse {
        total_posts,
        published,
        failed,
        draft,
        queued,
        best_provider,
        posts_by_provider,
        posts_by_day,
    }))
}
