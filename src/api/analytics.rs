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

// ── v23: New dashboard analytics endpoints ────────────────────
//
// These power the upgraded dashboard widgets. Each returns richer data
// than the v22 summary endpoint — per-day sparklines, deltas vs the
// previous period, and adherence/cadence metrics.

#[derive(Debug, Deserialize)]
pub struct AnalyticsDaysQuery {
    pub days: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct EngagementResponse {
    pub total_likes: i64,
    pub total_comments: i64,
    pub total_shares: i64,
    pub total_impressions: i64,
    // Deltas vs the previous period of the same length.
    pub likes_delta: i64,
    pub comments_delta: i64,
    pub shares_delta: i64,
    pub impressions_delta: i64,
    // Per-day breakdown for sparklines.
    pub by_day: Vec<DayEngagement>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DayEngagement {
    pub date: String,
    pub likes: i64,
    pub comments: i64,
    pub shares: i64,
    pub impressions: i64,
}

/// GET /api/analytics/engagement?days=7
///
/// Returns total engagement (likes/comments/shares/impressions) for the
/// last N days, with deltas vs the previous N days, and a per-day
/// breakdown for sparklines. Reads from post_engagement joined to posts.
pub async fn get_engagement(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<AnalyticsDaysQuery>,
) -> Result<Json<EngagementResponse>, AppError> {
    let days = query.days.unwrap_or(7).max(1) as i64;
    let now = chrono::Utc::now();
    let cutoff = now - chrono::Duration::days(days);
    let prev_cutoff = cutoff - chrono::Duration::days(days);

    // Current period totals.
    let current: DayEngagement = sqlx::query_as(
        r#"SELECT
            COALESCE(SUM(pe.likes), 0)::bigint as likes,
            COALESCE(SUM(pe.comments), 0)::bigint as comments,
            COALESCE(SUM(pe.shares), 0)::bigint as shares,
            COALESCE(SUM(pe.impressions), 0)::bigint as impressions,
            '' as date
           FROM post_engagement pe
           JOIN posts p ON pe.post_id = p.id
           WHERE p.user_id = $1 AND p.deleted_at IS NULL
             AND pe.created_at >= $2"#,
    )
    .bind(auth.user_id)
    .bind(cutoff)
    .fetch_one(&state.db)
    .await?;

    // Previous period totals (for delta calculation).
    let prev: DayEngagement = sqlx::query_as(
        r#"SELECT
            COALESCE(SUM(pe.likes), 0)::bigint as likes,
            COALESCE(SUM(pe.comments), 0)::bigint as comments,
            COALESCE(SUM(pe.shares), 0)::bigint as shares,
            COALESCE(SUM(pe.impressions), 0)::bigint as impressions,
            '' as date
           FROM post_engagement pe
           JOIN posts p ON pe.post_id = p.id
           WHERE p.user_id = $1 AND p.deleted_at IS NULL
             AND pe.created_at >= $2 AND pe.created_at < $3"#,
    )
    .bind(auth.user_id)
    .bind(prev_cutoff)
    .bind(cutoff)
    .fetch_one(&state.db)
    .await?;

    // Per-day breakdown for sparkline.
    let by_day: Vec<DayEngagement> = sqlx::query_as(
        r#"SELECT
            DATE(pe.created_at)::text as date,
            COALESCE(SUM(pe.likes), 0)::bigint as likes,
            COALESCE(SUM(pe.comments), 0)::bigint as comments,
            COALESCE(SUM(pe.shares), 0)::bigint as shares,
            COALESCE(SUM(pe.impressions), 0)::bigint as impressions
           FROM post_engagement pe
           JOIN posts p ON pe.post_id = p.id
           WHERE p.user_id = $1 AND p.deleted_at IS NULL
             AND pe.created_at >= $2
           GROUP BY DATE(pe.created_at)
           ORDER BY DATE(pe.created_at) ASC"#,
    )
    .bind(auth.user_id)
    .bind(cutoff)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(EngagementResponse {
        total_likes: current.likes,
        total_comments: current.comments,
        total_shares: current.shares,
        total_impressions: current.impressions,
        likes_delta: current.likes - prev.likes,
        comments_delta: current.comments - prev.comments,
        shares_delta: current.shares - prev.shares,
        impressions_delta: current.impressions - prev.impressions,
        by_day,
    }))
}

#[derive(Debug, Serialize)]
pub struct AdherenceResponse {
    pub scheduled: i64,
    pub published: i64,
    pub failed: i64,
    pub adherence_rate: f64, // published / scheduled * 100
}

/// GET /api/analytics/adherence?days=7
///
/// Returns scheduled-vs-actual adherence: how many posts were scheduled,
/// how many actually published, how many failed, and the adherence rate.
pub async fn get_adherence(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<AnalyticsDaysQuery>,
) -> Result<Json<AdherenceResponse>, AppError> {
    let days = query.days.unwrap_or(7).max(1) as i64;
    let cutoff = chrono::Utc::now() - chrono::Duration::days(days);

    let row: (i64, i64, i64) = sqlx::query_as(
        r#"SELECT
            COUNT(*) FILTER (WHERE state IN ('published', 'error', 'publishing'))::bigint as scheduled,
            COUNT(*) FILTER (WHERE state = 'published')::bigint as published,
            COUNT(*) FILTER (WHERE state = 'error')::bigint as failed
           FROM posts
           WHERE user_id = $1 AND deleted_at IS NULL
             AND scheduled_at >= $2"#,
    )
    .bind(auth.user_id)
    .bind(cutoff)
    .fetch_one(&state.db)
    .await?;

    let (scheduled, published, failed) = row;
    let adherence_rate = if scheduled > 0 {
        (published as f64 / scheduled as f64) * 100.0
    } else {
        100.0
    };

    Ok(Json(AdherenceResponse {
        scheduled,
        published,
        failed,
        adherence_rate,
    }))
}

#[derive(Debug, Serialize)]
pub struct CadenceResponse {
    pub goal_per_day: Option<f64>,
    pub actual_per_day: f64,
    pub streak_days: i64,
    pub total_posts: i64,
    pub by_day: Vec<DayCount>,
}

/// GET /api/analytics/cadence?days=30
///
/// Returns posting cadence: posts per day (actual vs goal if set in
/// brand profile), streak, and per-day breakdown. The "goal" comes
/// from the brand profile's posting_frequency field (stored in
/// localStorage on the frontend — TODO: sync to backend in v24).
pub async fn get_cadence(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<AnalyticsDaysQuery>,
) -> Result<Json<CadenceResponse>, AppError> {
    let days = query.days.unwrap_or(30).max(1) as i64;
    let cutoff = chrono::Utc::now() - chrono::Duration::days(days);

    let day_rows: Vec<(String, i64)> = sqlx::query_as(
        r#"SELECT DATE(p.published_at)::text, COUNT(*)::bigint
           FROM posts p
           WHERE p.user_id = $1 AND p.deleted_at IS NULL
             AND p.state = 'published' AND p.published_at >= $2
           GROUP BY DATE(p.published_at)
           ORDER BY DATE(p.published_at) ASC"#,
    )
    .bind(auth.user_id)
    .bind(cutoff)
    .fetch_all(&state.db)
    .await?;

    let total_posts: i64 = day_rows.iter().map(|(_, c)| c).sum();
    let actual_per_day = if days > 0 {
        total_posts as f64 / days as f64
    } else {
        0.0
    };

    // Streak: count consecutive days (ending today or yesterday) with
    // at least one published post.
    let streak_days = calculate_streak(&state.db, auth.user_id).await;

    let by_day: Vec<DayCount> = day_rows
        .into_iter()
        .map(|(date, count)| DayCount { date, count })
        .collect();

    Ok(Json(CadenceResponse {
        goal_per_day: None, // TODO: read from brand profile when backend-synced
        actual_per_day,
        streak_days,
        total_posts,
        by_day,
    }))
}

/// Calculate the current posting streak: consecutive days (ending today
/// or yesterday) with at least one published post.
async fn calculate_streak(db: &crate::db::PgPool, user_id: Uuid) -> i64 {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"SELECT DISTINCT DATE(p.published_at)::text
           FROM posts p
           WHERE p.user_id = $1 AND p.deleted_at IS NULL
             AND p.state = 'published' AND p.published_at IS NOT NULL
           ORDER BY DATE(p.published_at) DESC
           LIMIT 400"#, // cap at ~13 months
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    if rows.is_empty() {
        return 0;
    }

    let parse_date = |s: &str| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok();
    let today = chrono::Utc::now().date_naive();
    let yesterday = today - chrono::Duration::days(1);

    // Start from today or yesterday (so a streak isn't broken if the
    // user hasn't posted yet today).
    let first_date = parse_date(&rows[0].0);
    let streak_start = match first_date {
        Some(d) if d == today || d == yesterday => d,
        _ => return 0,
    };

    let mut streak = 1i64;
    let mut expected = streak_start - chrono::Duration::days(1);
    for (date_str,) in rows.iter().skip(1) {
        if let Some(d) = parse_date(date_str) {
            if d == expected {
                streak += 1;
                expected = expected - chrono::Duration::days(1);
            } else if d < expected {
                // Gap found — streak over.
                break;
            }
            // If d > expected (duplicate day, shouldn't happen with DISTINCT), skip.
        }
    }
    streak
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct EventLogEntry {
    pub id: Uuid,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct RecentEventsQuery {
    pub limit: Option<i32>,
}

/// GET /api/events/recent?limit=10
///
/// Returns the last N events from the events_log table. Powers the
/// dashboard's "Recent Activity" widget. The Broadcaster only fires
/// events when a subscriber is connected, so this endpoint is needed
/// to show activity that happened while no SSE client was connected.
pub async fn get_recent_events(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<RecentEventsQuery>,
) -> Result<Json<Vec<EventLogEntry>>, AppError> {
    let limit = query.limit.unwrap_or(10).clamp(1, 100) as i64;
    let entries: Vec<EventLogEntry> = sqlx::query_as(
        r#"SELECT id, event_type, payload, created_at
           FROM events_log
           WHERE user_id = $1
           ORDER BY created_at DESC
           LIMIT $2"#,
    )
    .bind(auth.user_id)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(entries))
}
