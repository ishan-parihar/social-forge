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
    let days = query.days.unwrap_or(7).clamp(1, 3650) as u32;

    // Check cache first
    let now = chrono::Utc::now();
    let cached =
        crate::db::queries::get_cached_analytics(&state.db, auth.user_id, &query.provider, now)
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
        .ok_or_else(|| {
            AppError::NotFound(format!("Provider '{}' not connected", query.provider))
        })?;

    let provider = state
        .providers
        .get(&query.provider)
        .ok_or_else(|| AppError::NotFound(format!("Provider '{}' not found", query.provider)))?;

    // Decrypt token if encryption is enabled.
    let tok =
        crate::crypto::maybe_decrypt_token(&integration.access_token, state.token_key.as_ref());

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
    Query(query): Query<PostAnalyticsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let post = crate::db::queries::get_post(&state.db, id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".into()))?;

    let platform_post_id = post
        .platform_post_id
        .ok_or_else(|| AppError::BadRequest("Post has not been published yet".into()))?;

    let integration =
        crate::db::queries::get_integration(&state.db, post.integration_id, auth.user_id)
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
            "data": filter_series_to_window(entry.data.clone(), query.days),
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
    let tok =
        crate::crypto::maybe_decrypt_token(&integration.access_token, state.token_key.as_ref());

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

/// The `days` window for `GET /api/analytics/post/{id}`.
#[derive(Debug, Deserialize)]
pub struct PostAnalyticsQuery {
    /// How many days of history to return. `None` means everything the
    /// provider gave us.
    pub days: Option<i64>,
}

/// Trim a provider-shaped series list to the last `days` days.
///
/// The provider's `post_analytics()` takes no window argument - it
/// returns whatever window the provider chooses. The cached payload
/// DOES carry a date per point, so the window can be applied to it here.
/// The client sent `?days=` all along and the handler had no query
/// extractor at all, so the range buttons in the Post Stats modal were
/// inert: clicking 90d showed exactly what 7d showed.
fn filter_series_to_window(data: serde_json::Value, days: Option<i64>) -> serde_json::Value {
    let Some(days) = days.filter(|d| *d > 0) else {
        return data;
    };
    let Some(series) = data.as_array() else {
        return data;
    };
    let cutoff = chrono::Utc::now() - chrono::Duration::days(days);

    let out: Vec<serde_json::Value> = series
        .iter()
        .map(|s| {
            let Some(points) = s.get("data").and_then(|d| d.as_array()) else {
                return s.clone();
            };
            let kept: Vec<serde_json::Value> = points
                .iter()
                .filter(|p| {
                    let Some(date) = p.get("date").and_then(|d| d.as_str()) else {
                        // A point with no date cannot be placed in the
                        // window. Keep it: dropping data is worse than
                        // showing a point that might be older.
                        return true;
                    };
                    match chrono::DateTime::parse_from_rfc3339(date) {
                        Ok(dt) => dt.with_timezone(&chrono::Utc) >= cutoff,
                        // An unparseable date is kept for the same reason.
                        Err(_) => true,
                    }
                })
                .cloned()
                .collect();
            let mut s = s.clone();
            if let Some(obj) = s.as_object_mut() {
                obj.insert("data".to_string(), serde_json::Value::Array(kept));
            }
            s
        })
        .collect();
    serde_json::Value::Array(out)
}

// ── Aggregated Summary ────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AnalyticsSummaryQuery {
    pub days: Option<i32>,
    /// The founder's zone as an IANA NAME ("America/New_York"). The
    /// day series buckets into the zone's days, so the chart's day
    /// boundaries match the calendar's. Absent: UTC (legacy).
    pub tz: Option<String>,
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

/// A DENSE day series: every day in the window, zeros included.
///
/// The query it replaces grouped by day, so it returned only the days
/// that HAD posts. The chart then drew one bar per day-with-a-post -
/// a month with 11 posts on 11 different days drew 11 full-height
/// bars, and the chart said "you posted every day at full volume"
/// while the cadence card beside it said "0.4 posts per day against a
/// goal of 1.5".
///
/// The chart shows a WINDOW, so the series has to cover the window.
///
/// The day is the LOCAL day in the founder's zone when `tz` is given
/// (an IANA name, resolved per-instant - DST-correct), and a UTC day
/// otherwise. Before the zone arrived the chart bucketed in UTC while
/// the calendar bucketed locally, so the same publish showed up on
/// two different days on the two pages of the same app.
fn dense_day_series(
    cutoff_epoch: i64,
    epochs: &[i64],
    tz: Option<chrono_tz::Tz>,
) -> Vec<DayCount> {
    use std::collections::HashMap;

    let mut counts: HashMap<chrono::NaiveDate, i64> = HashMap::new();
    for ts in epochs {
        let Some(dt) = chrono::DateTime::from_timestamp(*ts, 0) else {
            continue;
        };
        let day = match tz {
            Some(z) => (dt + chrono::Duration::seconds(
                crate::api::calendar::zone_offset_seconds(z, dt),
            )).date_naive(),
            None => dt.date_naive(),
        };
        *counts.entry(day).or_insert(0) += 1;
    }

    let Some(cutoff_dt) = chrono::DateTime::from_timestamp(cutoff_epoch, 0) else {
        return Vec::new();
    };
    // ALL THREE edges are the zone's days, or the window is incoherent:
    // buckets are zone days, `today` is the zone's date of now, and
    // `start` must be the zone's date of the cutoff. With a UTC start,
    // a post published at 01:00 UTC on the cutoff day in New York
    // (still 21:00 the PREVIOUS local day) passed the SQL filter but
    // bucketed to a day BEFORE the loop's first day - it existed in
    // the SQL counts and was MISSING from the chart.
    let zone_date = |dt: chrono::DateTime<chrono::Utc>| {
        match tz {
            Some(z) => (dt
                + chrono::Duration::seconds(
                    crate::api::calendar::zone_offset_seconds(z, dt),
                ))
            .date_naive(),
            None => dt.date_naive(),
        }
    };
    let start = zone_date(cutoff_dt);
    // "Today" is the zone's today, not UTC's: in Auckland the UTC
    // day is YESTERDAY'S, and the series would end one day early.
    let today = zone_date(chrono::Utc::now());

    let mut out = Vec::new();
    let mut day = start;
    while day <= today {
        let count = counts.get(&day).copied().unwrap_or(0);
        out.push(DayCount {
            date: day.format("%Y-%m-%d").to_string(),
            count,
        });
        day += chrono::Duration::days(1);
    }
    out
}

#[derive(Debug, Serialize)]
pub struct AnalyticsSummaryResponse {
    pub total_posts: i64,
    pub published: i64,
    /// The posts PUBLISHED inside the window, counted by their publish
    /// time.
    ///
    /// `published` is counted by `created_at`, so a post drafted 40 days
    /// ago and published today is in `published` but not in the window.
    /// It is the right DENOMINATOR for "interactions per post published
    /// in the last N days", because the numerator (the engagement
    /// total) is windowed by publish time too.
    ///
    /// The frontend used to compute the denominator from the newest 100
    /// posts it happened to have loaded, so with 150 published in the
    /// window the rate was divided by 100 - and both numbers looked
    /// plausible on their own.
    pub published_in_window: i64,
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
    let days = query.days.unwrap_or(30).clamp(1, 3650);
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(days as i64)).timestamp();

    let state_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT state, COUNT(*) FROM posts WHERE user_id = ? AND created_at >= ? GROUP BY state",
    )
    .bind(auth.user_id)
    .bind(cutoff)
    .fetch_all(&state.db)
    .await?;

    // "Posts by channel" counts PUBLISHED posts, by the day they were
    // published. It used to count every post by its CREATION date, so a
    // user who drafted 10 posts a month ago and published 2 this week
    // saw a bar of 10 for last month and none for this week - while the
    // rest of the page (published, cadence, engagement) counts by
    // publish time. The two headline charts of the page described
    // different events.
    let provider_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT i.provider_identifier, COUNT(*) \
         FROM posts p JOIN integrations i ON p.integration_id = i.id \
         WHERE p.user_id = ? AND p.state = 'published' \
           AND p.published_at >= ? AND p.published_at IS NOT NULL \
         GROUP BY i.provider_identifier \
         ORDER BY COUNT(*) DESC",
    )
    .bind(auth.user_id)
    .bind(cutoff)
    .fetch_all(&state.db)
    .await?;

    // "Posting volume" counts PUBLISHED posts, by the day they went
    // out - for the same reason as the channel mix above.
    //
    // THE SERIES IS DENSE.
    //
    // `GROUP BY day` returns only the days that HAVE posts. The chart
    // then drew one bar per day-with-a-post, so a month with 11 posts
    // on 11 different days drew 11 full-height bars - the chart said
    // "you posted every day at full volume" while the card beside it
    // said "0.4 posts per day against a goal of 1.5". Two numbers, one
    // page, and they contradicted each other.
    //
    // The chart shows a WINDOW, so the series has to cover the window:
    // every day in it, zeros included.
    //
    // The epochs are selected and bucketed in RUST rather than grouped
    // in SQL, so the day key and the gap-filling live in ONE place.
    //
    // The day is a UTC day, the convention the rest of this endpoint
    // uses. The founder's own zone is an IANA name in a FRONTEND
    // localStorage store, which the server never sees - so the day
    // bucket cannot follow it here. See the worklog: the two sources
    // of the zone are named there as an open item.
    let published_epochs: Vec<i64> = sqlx::query_scalar(
        "SELECT p.published_at FROM posts p \
         WHERE p.user_id = ? AND p.state = 'published' \
           AND p.published_at >= ? AND p.published_at IS NOT NULL",
    )
    .bind(auth.user_id)
    .bind(cutoff)
    .fetch_all(&state.db)
    .await?;
    let tz = match query.tz.as_deref() {
        Some(name) => Some(crate::api::calendar::parse_zone(name).map_err(AppError::BadRequest)?),
        None => None,
    };
    let posts_by_day = dense_day_series(cutoff, &published_epochs, tz);

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

    // The posts published INSIDE the window, by publish time. The
    // denominator for "interactions per post published in the last N
    // days", and the one the frontend could not compute for itself -
    // it only ever held the newest 100 posts.
    let published_in_window: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM posts
          WHERE user_id = ? AND state = 'published'
            AND published_at IS NOT NULL AND published_at >= ?",
    )
    .bind(auth.user_id)
    .bind(cutoff)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    Ok(Json(AnalyticsSummaryResponse {
        total_posts,
        published,
        published_in_window,
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
    // Percentage change vs the previous period of the same length.
    //
    // These were an ABSOLUTE difference (`current - previous`) while the
    // UI rendered them with a `%` sign: a card reading 100 likes with a
    // delta of 20 showed "Likes +20%", which reads as twenty percent
    // and is actually twenty likes. The type is a float now and the
    // value is the percentage the sign promises.
    pub likes_delta: f64,
    pub comments_delta: f64,
    pub shares_delta: f64,
    pub impressions_delta: f64,
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

/// Fill the gaps in a per-day engagement series.
///
/// The source query groups by publish date, so a day with nothing
/// published has no row. A sparkline drawn from that series connects
/// non-adjacent calendar dates as if they were evenly spaced, and a day
/// with zero engagement is not plotted as zero - so the shape of the
/// trend is wrong whenever publishing is not daily.
///
/// This inserts a zero row for every missing day between the window's
/// start and its end, so the series is one point per calendar day.
fn fill_missing_days(rows: Vec<DayEngagement>, cutoff_epoch: i64, days: i64) -> Vec<DayEngagement> {
    // The window's last day is today (in UTC, which is what the day
    // buckets use).
    let today = chrono::Utc::now().date_naive();
    let first = chrono::DateTime::from_timestamp(cutoff_epoch, 0)
        .map(|dt| dt.date_naive())
        .unwrap_or(today);

    let mut by_date: std::collections::HashMap<chrono::NaiveDate, DayEngagement> =
        std::collections::HashMap::new();
    for r in rows {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(&r.date, "%Y-%m-%d") {
            by_date.insert(d, r);
        }
    }

    let mut out = Vec::with_capacity(days as usize + 1);
    let mut d = first;
    // Guard against a pathological `days`: the query clamps it to 3650.
    let limit = first + chrono::Duration::days(days.min(3650));
    while d <= limit && d <= today {
        out.push(by_date.remove(&d).unwrap_or(DayEngagement {
            date: d.format("%Y-%m-%d").to_string(),
            likes: 0,
            comments: 0,
            shares: 0,
            impressions: 0,
        }));
        d += chrono::Duration::days(1);
    }
    out
}

/// The percentage change from `previous` to `current`.
///
/// Returns 0.0 when there is no previous period to compare against:
/// "0% vs nothing" is the honest reading, where dividing by zero would
/// produce infinity or NaN.
fn percent_change(current: i64, previous: i64) -> f64 {
    if previous == 0 {
        return 0.0;
    }
    let delta = current - previous;
    (delta as f64 / previous as f64) * 100.0
}

/// GET /api/analytics/engagement?days=7
///
/// Returns total engagement (likes/comments/shares/impressions) for the
/// last N days, with deltas vs the previous N days, and a per-day
/// breakdown for sparklines.
///
/// The source is `analytics_cache`, NOT `post_engagement`. Own-post
/// engagement is written to `analytics_cache` by
/// `/api/analytics/post/{id}` after a live provider fetch, and
/// `post_engagement.post_id` is a foreign key to `external_posts` - so
/// joining it to `posts` (as this handler used to) could never match a
/// row, and the endpoint answered 0 for every metric no matter how much
/// engagement the user's published posts had.
pub async fn get_engagement(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<AnalyticsDaysQuery>,
) -> Result<Json<EngagementResponse>, AppError> {
    let days = query.days.unwrap_or(7).clamp(1, 3650) as i64;
    let now = chrono::Utc::now();
    let window = chrono::Duration::days(days);
    let cutoff = now - window;
    let prev_cutoff = cutoff - window;

    // Current period totals.
    let current =
        crate::db::queries::own_post_engagement_totals(&state.db, auth.user_id, cutoff).await?;

    // Previous period totals (for delta calculation).
    let prev = crate::db::queries::own_post_engagement_totals(&state.db, auth.user_id, prev_cutoff)
        .await?;

    // Per-day breakdown for the sparkline, keyed by the day each post
    // was published.
    //
    // The label set comes from `queries::metric_label_matches` - the one
    // place the provider vocabulary is mapped. Matching only
    // 'likes'/'comments'/'shares' meant an X user's sparkline showed
    // likes but no comments and no shares, forever.
    let by_day_sql = format!(
        r#"SELECT
            date(p.published_at, 'unixepoch') as date,
            COALESCE(SUM((SELECT json_extract(point.value, '$.total')
                            FROM analytics_cache ac, json_each(ac.data) AS series,
                                 json_each(json_extract(series.value, '$.data')) AS point
                           WHERE ac.user_id = p.user_id
                             AND ac.platform_post_id = p.platform_post_id
                             AND {likes}
                           ORDER BY point.key DESC LIMIT 1)), 0) as likes,
            COALESCE(SUM((SELECT json_extract(point.value, '$.total')
                            FROM analytics_cache ac, json_each(ac.data) AS series,
                                 json_each(json_extract(series.value, '$.data')) AS point
                           WHERE ac.user_id = p.user_id
                             AND ac.platform_post_id = p.platform_post_id
                             AND {comments}
                           ORDER BY point.key DESC LIMIT 1)), 0) as comments,
            COALESCE(SUM((SELECT json_extract(point.value, '$.total')
                            FROM analytics_cache ac, json_each(ac.data) AS series,
                                 json_each(json_extract(series.value, '$.data')) AS point
                           WHERE ac.user_id = p.user_id
                             AND ac.platform_post_id = p.platform_post_id
                             AND {shares}
                           ORDER BY point.key DESC LIMIT 1)), 0) as shares,
            COALESCE(SUM((SELECT json_extract(point.value, '$.total')
                            FROM analytics_cache ac, json_each(ac.data) AS series,
                                 json_each(json_extract(series.value, '$.data')) AS point
                           WHERE ac.user_id = p.user_id
                             AND ac.platform_post_id = p.platform_post_id
                             AND {impressions}
                           ORDER BY point.key DESC LIMIT 1)), 0) as impressions
           FROM posts p
           WHERE p.user_id = ? AND p.deleted_at IS NULL
             AND p.state = 'published'
             AND p.published_at >= ?
             GROUP BY date(p.published_at, 'unixepoch')
            ORDER BY date(p.published_at, 'unixepoch') ASC"#,
        likes = crate::db::queries::metric_label_matches(crate::db::queries::METRIC_LIKES),
        comments = crate::db::queries::metric_label_matches(crate::db::queries::METRIC_COMMENTS),
        shares = crate::db::queries::metric_label_matches(crate::db::queries::METRIC_SHARES),
        impressions =
            crate::db::queries::metric_label_matches(crate::db::queries::METRIC_IMPRESSIONS),
    );
    let by_day_rows: Vec<DayEngagement> = sqlx::query_as(&by_day_sql)
        .bind(auth.user_id)
        .bind(crate::db::types::EpochUtc::from(cutoff))
        .fetch_all(&state.db)
        .await?;

    // Fill in the days with no published post.
    //
    // The query groups by publish date, so a day with nothing published
    // is ABSENT from the series. The frontend maps the series to a
    // positional array and draws a sparkline, so the missing days were
    // silently dropped: the chart connected non-adjacent calendar dates
    // as if they were evenly spaced, and a day with zero engagement was
    // not plotted as zero. The shape of the trend was wrong whenever
    // publishing was not daily.
    //
    // Filling the gaps makes the x-axis "days in the window" - which is
    // what the chart's label says it is.
    let by_day = fill_missing_days(by_day_rows, cutoff.timestamp(), days);

    Ok(Json(EngagementResponse {
        total_likes: current.likes,
        total_comments: current.comments,
        total_shares: current.shares,
        total_impressions: current.impressions,
        likes_delta: percent_change(current.likes, prev.likes),
        comments_delta: percent_change(current.comments, prev.comments),
        shares_delta: percent_change(current.shares, prev.shares),
        impressions_delta: percent_change(current.impressions, prev.impressions),
        by_day,
    }))
}

#[derive(Debug, Serialize)]
pub struct AdherenceResponse {
    pub scheduled: i64,
    pub published: i64,
    pub failed: i64,
    pub adherence_rate: f64, // published / max(scheduled, goal * days) * 100
    /// published / scheduled * 100.
    ///
    /// The SUCCESS RATE, and it is a different number from the
    /// adherence rate.
    ///
    /// The two were conflated on two surfaces, and the founder could
    /// not reconcile them: a card read "Publish success rate 24%" and
    /// directly under it "11 of 13 scheduled posts went out" - which
    /// is 85%. The 24% came from the goal-inflated denominator
    /// (`max(scheduled, goal x days)`), so with a 1.5/day goal over 30
    /// days the denominator was 45 and the "success rate" was 11/45.
    ///
    /// A success rate has to be a success rate. This is the plain
    /// fraction of the scheduled posts that actually went out, and the
    /// goal-based adherence rate is reported beside it under its own
    /// name.
    pub success_rate: f64,
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
    let days = query.days.unwrap_or(7).clamp(1, 3650) as i64;
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(days)).timestamp();

    // The window is BOUNDED on both ends. It used to be only a floor
    // (`scheduled_at >= cutoff`), so a post scheduled for NEXT MONTH
    // counted towards the denominator of a 7-day window: the UI read
    // "3 of 5 scheduled posts went out" for a window that had not
    // happened yet, with a misleadingly low success rate.
    let now = chrono::Utc::now().timestamp();
    let row: (i64, i64, i64) = sqlx::query_as(
        r#"SELECT
            COUNT(*) FILTER (WHERE state IN ('published', 'error', 'publishing')) as scheduled,
            COUNT(*) FILTER (WHERE state = 'published') as published,
            COUNT(*) FILTER (WHERE state = 'error') as failed
           FROM posts
           WHERE user_id = ? AND deleted_at IS NULL
             AND scheduled_at >= ? AND scheduled_at <= ?"#,
    )
    .bind(auth.user_id)
    .bind(cutoff)
    .bind(now)
    .fetch_one(&state.db)
    .await?;

    let goal = resolve_goal_per_day(&state.db, auth.user_id, query.goal_per_day).await;

    let (scheduled, published, failed) = row;
    let adherence_rate = adherence_percent(scheduled, published, goal, days);
    let success_rate = if scheduled > 0 {
        ((published as f64 / scheduled as f64) * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };

    Ok(Json(AdherenceResponse {
        scheduled,
        published,
        failed,
        adherence_rate,
        success_rate,
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
    let days = query.days.unwrap_or(30).clamp(1, 3650) as i64;
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
                expected -= chrono::Duration::days(1);
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

/// A row of `events_log` plus the sentence the UI shows for it.
///
/// The sentence is computed HERE, by the same function the read-only
/// activity log uses (`crate::api::activity::event_summary`). The analytics
/// page used to carry its own copy of that map, and the two drifted: the
/// same `first_comment_failed` row read "First comment failed" on the
/// dashboard and "First comment could not be posted" on the activity page.
/// One owner, one sentence.
#[derive(Debug, Serialize)]
pub struct EventLogEntryWithSummary {
    #[serde(flatten)]
    pub entry: EventLogEntry,
    /// Human-readable one-liner. The frontend renders this verbatim.
    pub summary: String,
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
) -> Result<Json<Vec<EventLogEntryWithSummary>>, AppError> {
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
    let entries = entries
        .into_iter()
        .map(|entry| {
            let summary = crate::api::activity::event_summary(&entry.event_type, &entry.payload);
            EventLogEntryWithSummary { entry, summary }
        })
        .collect();
    Ok(Json(entries))
}

#[cfg(test)]
mod tests {
    use super::adherence_percent;
    use super::dense_day_series;
    use chrono::{Duration, Utc};

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

    /// The day series covers EVERY day in the window, zeros included.
    ///
    /// The query it replaces grouped by day, so it returned only the
    /// days that had posts - and the chart drew one full-height bar per
    /// such day. A month with 11 posts on 11 different days read as
    /// "you posted every day at full volume" next to a card that said
    /// "0.4 posts per day".
    #[test]
    fn the_day_series_covers_every_day_in_the_window() {
        let today = Utc::now().date_naive();
        let start = today - Duration::days(6);
        let midnight = |d: chrono::NaiveDate| d.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
        let cutoff = midnight(start);

        // Two posts, on the FIRST and the LAST day of the window.
        let first = midnight(start);
        let last = midnight(today);
        let series = dense_day_series(cutoff, &[first, last, last], None);

        // Seven days, not two.
        assert_eq!(series.len(), 7, "the series must cover every day");

        // Contiguous and ascending.
        for pair in series.windows(2) {
            let a = chrono::NaiveDate::parse_from_str(&pair[0].date, "%Y-%m-%d").unwrap();
            let b = chrono::NaiveDate::parse_from_str(&pair[1].date, "%Y-%m-%d").unwrap();
            assert_eq!(
                (b - a).num_days(),
                1,
                "the days must be contiguous: {pair:?}"
            );
        }

        // The counts: one on the first day, two on the last, ZERO in
        // between.
        assert_eq!(series[0].count, 1);
        assert_eq!(series[6].count, 2);
        assert!(
            series[1..6].iter().all(|d| d.count == 0),
            "the days with no posts must be zero, not missing: {series:?}"
        );
    }

    /// The date format is the one the frontend slices.
    ///
    /// The chart does `day.date.slice(5)` to show "MM-DD", so the
    /// format has to stay "YYYY-MM-DD".
    #[test]
    fn the_day_series_dates_are_iso_days() {
        let cutoff = Utc::now().timestamp();
        let series = dense_day_series(cutoff, &[], None);
        assert!(!series.is_empty());
        assert!(
            chrono::NaiveDate::parse_from_str(&series[0].date, "%Y-%m-%d").is_ok(),
            "the date must be YYYY-MM-DD: {}",
            series[0].date
        );
    }

    /// A windowed post must land INSIDE the series. The bug: buckets
    /// and `today` were zone days but `start` was the UTC date of the
    /// cutoff, so a post at 01:00 UTC on the cutoff day in New York
    /// (21:00 the PREVIOUS local day) passed the SQL filter, bucketed
    /// to a day BEFORE the loop's first day, and vanished from the
    /// chart while the SQL counts still included it.
    #[test]
    fn a_zone_series_covers_a_post_published_at_the_windows_zone_edge() {
        let ny = crate::api::calendar::parse_zone("America/New_York").unwrap();
        // 2026-10-21 01:00Z: local New York 2026-10-20 21:00 (EDT).
        let edge_post = chrono::DateTime::parse_from_rfc3339("2026-10-21T01:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
            .timestamp();
        // Cutoff exactly at the window start: 2026-10-21 00:00Z.
        let cutoff = chrono::DateTime::parse_from_rfc3339("2026-10-21T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
            .timestamp();
        let series = dense_day_series(cutoff, &[edge_post], Some(ny));
        // The series starts at the ZONE date of the cutoff (Oct 20 in
        // New York), so the edge post's zone day is the FIRST day and
        // the count is visible - not dropped off the front.
        assert_eq!(
            series[0].date, "2026-10-20",
            "the first day is the zone date of the cutoff: {}",
            series[0].date
        );
        assert_eq!(
            series[0].count, 1,
            "the edge post bucketed to the first day, so it is IN the chart"
        );
        let total: i64 = series.iter().map(|d| d.count).sum();
        assert_eq!(total, 1, "no post may vanish from the window");
    }
}
