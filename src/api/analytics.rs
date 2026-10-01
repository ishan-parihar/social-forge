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
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(days as i64)).timestamp();

    let state_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT state, COUNT(*) FROM posts WHERE user_id = ? AND created_at >= ? GROUP BY state",
    )
    .bind(auth.user_id)
    .bind(cutoff)
    .fetch_all(&state.db)
    .await?;

    let provider_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT i.provider_identifier, COUNT(*) \
         FROM posts p JOIN integrations i ON p.integration_id = i.id \
         WHERE p.user_id = ? AND p.created_at >= ? \
         GROUP BY i.provider_identifier \
         ORDER BY COUNT(*) DESC",
    )
    .bind(auth.user_id)
    .bind(cutoff)
    .fetch_all(&state.db)
    .await?;

    let day_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT date(p.created_at, 'unixepoch'), COUNT(*) \
         FROM posts p WHERE p.user_id = ? AND p.created_at >= ? \
         GROUP BY date(p.created_at, 'unixepoch') \
         ORDER BY date(p.created_at, 'unixepoch') ASC",
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
    /// Optional per-day posting goal. Honored by the endpoints that measure
    /// against a goal (adherence, cadence); see [`resolve_goal_per_day`].
    /// Ignored by the endpoints that have no goal notion.
    pub goal_per_day: Option<f64>,
}

/// Resolve the per-day posting goal for one request.
///
/// Precedence: an explicit `?goal_per_day=N` wins for the current request and
/// is persisted to `brand_profiles.posts_per_day_goal` (the column exists for
/// exactly this widget — migration 036) so the goal sticks across requests and
/// devices instead of living in frontend localStorage. With no param, the
/// stored value is returned. `None` when neither is set, which preserves the
/// pre-goal behavior of every caller.
async fn resolve_goal_per_day(
    db: &sqlx::SqlitePool,
    user_id: Uuid,
    requested: Option<f64>,
) -> Option<f64> {
    // A goal of 0 or negative is meaningless as an adherence basis, so it is
    // ignored rather than persisted.
    if let Some(goal) = requested.filter(|g| g.is_finite() && *g > 0.0) {
        if let Err(e) = sqlx::query(
            r#"INSERT INTO brand_profiles (user_id, posts_per_day_goal)
               VALUES (?, ?)
               ON CONFLICT (user_id) DO UPDATE
                   SET posts_per_day_goal = EXCLUDED.posts_per_day_goal,
                       updated_at = unixepoch()"#,
        )
        .bind(user_id)
        .bind(goal)
        .execute(db)
        .await
        {
            // Never fail the read because the goal could not be saved — the
            // requested goal still applies to this response.
            tracing::warn!("could not persist goal_per_day: {e}");
        }
        return Some(goal);
    }

    sqlx::query_scalar::<_, Option<f64>>(
        "SELECT posts_per_day_goal FROM brand_profiles WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .flatten()
}

/// Adherence percentage for a window.
///
/// The expectation is the larger of what was actually planned (`scheduled`) and
/// what the goal implies over the window, so a plan that sits below the goal
/// still surfaces the shortfall rather than reporting 100%. With no goal the
/// expectation is the plan alone — the original rate.
///
/// Clamped to 0..=100 because the dashboard renders this value as a bar width.
fn adherence_percent(scheduled: i64, published: i64, goal: Option<f64>, days: i64) -> f64 {
    let expected = match goal {
        Some(g) => (scheduled as f64).max(g * days as f64),
        None => scheduled as f64,
    };
    if expected <= 0.0 {
        return 100.0;
    }
    ((published as f64 / expected) * 100.0).clamp(0.0, 100.0)
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
    let cutoff = (now - chrono::Duration::days(days)).timestamp();
    let prev_cutoff = cutoff - days * 86400;

    // Current period totals.
    let current: DayEngagement = sqlx::query_as(
        r#"SELECT
            COALESCE(SUM(pe.likes), 0) as likes,
            COALESCE(SUM(pe.comments), 0) as comments,
            COALESCE(SUM(pe.shares), 0) as shares,
            COALESCE(SUM(pe.impressions), 0) as impressions,
            '' as date
           FROM post_engagement pe
           JOIN posts p ON pe.post_id = p.id
           WHERE p.user_id = ? AND p.deleted_at IS NULL
             AND pe.created_at >= ?"#,
    )
    .bind(auth.user_id)
    .bind(cutoff)
    .fetch_one(&state.db)
    .await?;

    // Previous period totals (for delta calculation).
    let prev: DayEngagement = sqlx::query_as(
        r#"SELECT
            COALESCE(SUM(pe.likes), 0) as likes,
            COALESCE(SUM(pe.comments), 0) as comments,
            COALESCE(SUM(pe.shares), 0) as shares,
            COALESCE(SUM(pe.impressions), 0) as impressions,
            '' as date
           FROM post_engagement pe
           JOIN posts p ON pe.post_id = p.id
           WHERE p.user_id = ? AND p.deleted_at IS NULL
             AND pe.created_at >= ? AND pe.created_at < ?"#,
    )
    .bind(auth.user_id)
    .bind(prev_cutoff)
    .bind(cutoff)
    .fetch_one(&state.db)
    .await?;

    // Per-day breakdown for sparkline.
    let by_day: Vec<DayEngagement> = sqlx::query_as(
        r#"SELECT
            date(pe.created_at, 'unixepoch') as date,
            COALESCE(SUM(pe.likes), 0) as likes,
            COALESCE(SUM(pe.comments), 0) as comments,
            COALESCE(SUM(pe.shares), 0) as shares,
            COALESCE(SUM(pe.impressions), 0) as impressions
           FROM post_engagement pe
           JOIN posts p ON pe.post_id = p.id
           WHERE p.user_id = ? AND p.deleted_at IS NULL
             AND pe.created_at >= ?
           GROUP BY date(pe.created_at, 'unixepoch')
           ORDER BY date(pe.created_at, 'unixepoch') ASC"#,
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
    pub adherence_rate: f64, // published / max(scheduled, goal * days) * 100
}

/// GET /api/analytics/adherence?days=7[&goal_per_day=N]
///
/// Returns scheduled-vs-actual adherence: how many posts were scheduled,
/// how many actually published, how many failed, and the adherence rate.
///
/// `goal_per_day` sets (and persists) the per-day posting goal; the rate is
/// then measured against the larger of the plan and the goal for the window.
/// Omitting it uses the stored goal, and with no goal stored the rate is
/// exactly what it was before goals existed.
pub async fn get_adherence(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<AnalyticsDaysQuery>,
) -> Result<Json<AdherenceResponse>, AppError> {
    let days = query.days.unwrap_or(7).max(1) as i64;
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(days)).timestamp();

    let row: (i64, i64, i64) = sqlx::query_as(
        r#"SELECT
            COUNT(*) FILTER (WHERE state IN ('published', 'error', 'publishing')) as scheduled,
            COUNT(*) FILTER (WHERE state = 'published') as published,
            COUNT(*) FILTER (WHERE state = 'error') as failed
           FROM posts
           WHERE user_id = ? AND deleted_at IS NULL
             AND scheduled_at >= ?"#,
    )
    .bind(auth.user_id)
    .bind(cutoff)
    .fetch_one(&state.db)
    .await?;

    let goal = resolve_goal_per_day(&state.db, auth.user_id, query.goal_per_day).await;

    let (scheduled, published, failed) = row;
    let adherence_rate = adherence_percent(scheduled, published, goal, days);

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

/// GET /api/analytics/cadence?days=30[&goal_per_day=N]
///
/// Returns posting cadence: posts per day (actual vs goal), streak, and
/// per-day breakdown. The goal comes from the brand profile
/// (`brand_profiles.posts_per_day_goal`, migration 036); `?goal_per_day=N`
/// sets and persists it. `None` when no goal was ever set.
pub async fn get_cadence(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<AnalyticsDaysQuery>,
) -> Result<Json<CadenceResponse>, AppError> {
    let days = query.days.unwrap_or(30).max(1) as i64;
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(days)).timestamp();

    let day_rows: Vec<(String, i64)> = sqlx::query_as(
        r#"SELECT date(p.published_at, 'unixepoch'), COUNT(*)
           FROM posts p
           WHERE p.user_id = ? AND p.deleted_at IS NULL
             AND p.state = 'published' AND p.published_at >= ?
           GROUP BY date(p.published_at, 'unixepoch')
           ORDER BY date(p.published_at, 'unixepoch') ASC"#,
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

    let goal = resolve_goal_per_day(&state.db, auth.user_id, query.goal_per_day).await;

    let by_day: Vec<DayCount> = day_rows
        .into_iter()
        .map(|(date, count)| DayCount { date, count })
        .collect();

    Ok(Json(CadenceResponse {
        goal_per_day: goal,
        actual_per_day,
        streak_days,
        total_posts,
        by_day,
    }))
}

/// Calculate the current posting streak: consecutive days (ending today
/// or yesterday) with at least one published post.
async fn calculate_streak(db: &sqlx::SqlitePool, user_id: Uuid) -> i64 {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"SELECT DISTINCT date(p.published_at, 'unixepoch')
           FROM posts p
           WHERE p.user_id = ? AND p.deleted_at IS NULL
             AND p.state = 'published' AND p.published_at IS NOT NULL
           ORDER BY date(p.published_at, 'unixepoch') DESC
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
           WHERE user_id = ?
           ORDER BY created_at DESC
           LIMIT ?"#,
    )
    .bind(auth.user_id)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(entries))
}

#[cfg(test)]
mod tests {
    use super::adherence_percent;

    #[test]
    fn no_goal_keeps_plan_only_rate() {
        // 7 of 10 shipped, nothing scheduled against a goal → 70%.
        assert_eq!(adherence_percent(10, 7, None, 7), 70.0);
    }

    #[test]
    fn no_goal_and_nothing_planned_is_vacuously_full() {
        assert_eq!(adherence_percent(0, 0, None, 7), 100.0);
    }

    #[test]
    fn goal_above_plan_surfaces_the_shortfall() {
        // Plan of 5 over 7 days, goal of 3/day (21 expected), 7 published.
        assert!((adherence_percent(5, 7, Some(3.0), 7) - 33.333).abs() < 0.01);
    }

    #[test]
    fn plan_above_goal_keeps_plan_as_expectation() {
        // 12 planned, goal only implies 7 → the plan still wins, 100%.
        assert_eq!(adherence_percent(12, 12, Some(1.0), 7), 100.0);
    }

    #[test]
    fn rate_is_clamped_to_a_renderable_width() {
        // Goal 1/day over 1 day but 3 published: >100% must not reach the bar.
        assert_eq!(adherence_percent(3, 3, Some(1.0), 1), 100.0);
    }
}
