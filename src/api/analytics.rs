// ─── Analytics API Routes ─────────────────────────────────────
// Dashboard analytics for connected social providers.
// Cache-first: reads from analytics_cache, falls back to live-fetch.

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

/// GET /api/analytics?provider=X&days=N
///
/// Cache-first strategy:
/// 1. Check analytics_cache for (user_id, provider) where platform_post_id IS NULL
/// 2. If cached and not expired (expires_at > now), return cached data
/// 3. Otherwise live-fetch from provider, store in cache, return
pub async fn get(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let days = query.days.unwrap_or(7).max(1) as u32;

    // Check cache first
    let now = chrono::Utc::now();
    let cached = crate::db::queries::get_cached_analytics(
        &state.db,
        auth.user_id,
        &query.provider,
        now,
    )
    .await?;

    // If we have cached data, return it directly
    if let Some(entry) = cached.into_iter().next() {
        return Ok(Json(serde_json::json!({
            "data": entry.data,
            "cached": true,
            "cached_at": entry.cached_at.to_rfc3339(),
        })));
    }

    // Cache miss — live-fetch from provider
    let integrations = crate::db::queries::list_integrations(&state.db, auth.user_id).await?;

    let integration = integrations
        .iter()
        .find(|i| i.provider_identifier == query.provider)
        .ok_or_else(|| AppError::NotFound(format!("Provider '{}' not connected", query.provider)))?;

    let provider = state
        .providers
        .get(&query.provider)
        .ok_or_else(|| AppError::NotFound(format!("Provider '{}' not found", query.provider)))?;

    // Decrypt token if encryption is enabled.
    let tok = crate::crypto::maybe_decrypt_token(&integration.access_token, state.token_key.as_ref());

    let analytics = provider
        .analytics(&tok, &integration.internal_id, days)
        .await
        .map_err(AppError::from)?;

    // Store in cache (best-effort — cache miss is non-fatal)
    let data = serde_json::to_value(&analytics).unwrap_or(serde_json::Value::Null);
    if let Err(e) = crate::db::queries::upsert_analytics_cache(
        &state.db,
        auth.user_id,
        &query.provider,
        None,
        &data,
    )
    .await
    {
        tracing::warn!("Failed to upsert analytics cache: {e}");
    }

    Ok(Json(serde_json::json!({ "data": analytics })))
}

/// GET /api/analytics/post/{id}
///
/// Cache-first strategy:
/// 1. Get the post and its integration
/// 2. Check analytics_cache for (user_id, provider, platform_post_id)
/// 3. If cached and valid, return it
/// 4. Otherwise live-fetch via provider.post_analytics(), store, return
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

    // Check cache for this specific post
    let cached = crate::db::queries::get_single_cached_analytics(
        &state.db,
        auth.user_id,
        &integration.provider_identifier,
        &platform_post_id,
    )
    .await?;

    if let Some(entry) = cached {
        return Ok(Json(serde_json::json!({
            "data": entry.data,
            "cached": true,
            "cached_at": entry.cached_at.to_rfc3339(),
        })));
    }

    // Cache miss — live-fetch
    let provider = state
        .providers
        .get(&integration.provider_identifier)
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "Provider '{}' not found",
                integration.provider_identifier
            ))
        })?;

    // Decrypt token if encryption is enabled.
    let tok = crate::crypto::maybe_decrypt_token(&integration.access_token, state.token_key.as_ref());

    let analytics = provider
        .post_analytics(&tok, &platform_post_id)
        .await
        .map_err(AppError::from)?;

    // Store in cache (best-effort — cache miss is non-fatal)
    let data = serde_json::to_value(&analytics).unwrap_or(serde_json::Value::Null);
    if let Err(e) = crate::db::queries::upsert_analytics_cache(
        &state.db,
        auth.user_id,
        &integration.provider_identifier,
        Some(&platform_post_id),
        &data,
    )
    .await
    {
        tracing::warn!("Failed to upsert analytics cache: {e}");
    }

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

    let state_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT state::text, COUNT(*)::bigint FROM posts WHERE user_id = $1 AND created_at >= $2 GROUP BY state",
    )
    .bind(auth.user_id)
    .bind(cutoff)
    .fetch_all(&state.db)
    .await?;

    let provider_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT i.provider_identifier, COUNT(*)::bigint \
         FROM posts p JOIN integrations i ON p.integration_id = i.id \
         WHERE p.user_id = $1 AND p.created_at >= $2 \
         GROUP BY i.provider_identifier \
         ORDER BY COUNT(*)::bigint DESC",
    )
    .bind(auth.user_id)
    .bind(cutoff)
    .fetch_all(&state.db)
    .await?;

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
