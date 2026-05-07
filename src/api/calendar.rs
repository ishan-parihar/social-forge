// ─── Calendar API Routes ──────────────────────────────────────
// Date-range queries for the content calendar.
// Returns posts grouped by date for calendar grid rendering.

use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use crate::auth::middleware::AuthenticatedUser;
use crate::db::models::PostPublic;
use crate::db::queries;
use crate::error::AppError;
use super::AppState;

#[derive(Debug, Deserialize)]
pub struct CalendarQuery {
    pub start: String, // ISO8601 date or datetime
    pub end: String,
}

#[derive(Debug, Serialize)]
pub struct CalendarDay {
    pub date: String,
    pub posts: Vec<PostPublic>,
}

#[derive(Debug, Serialize)]
pub struct CalendarResponse {
    pub days: Vec<CalendarDay>,
    pub total: usize,
}

/// GET /api/calendar?start=2026-05-01&end=2026-05-31
pub async fn get(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<CalendarQuery>,
) -> Result<Json<CalendarResponse>, AppError> {
    // Parse date range
    let start = parse_date_or_datetime(&query.start)
        .ok_or_else(|| AppError::BadRequest("Invalid start date. Use ISO8601 (YYYY-MM-DD or full datetime)".into()))?;
    let end = parse_date_or_datetime(&query.end)
        .ok_or_else(|| AppError::BadRequest("Invalid end date".into()))?;

    let posts = queries::get_posts_by_date_range(&state.db, auth.user_id, start, end).await?;

    // Group posts by date
    let mut day_map: std::collections::BTreeMap<String, Vec<PostPublic>> = std::collections::BTreeMap::new();
    for p in posts {
        let date_key = p
            .scheduled_at
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "unscheduled".into());
        day_map.entry(date_key).or_default().push(PostPublic::from(p));
    }

    let days: Vec<CalendarDay> = day_map
        .into_iter()
        .map(|(date, posts)| CalendarDay { date, posts })
        .collect();

    let total = days.iter().map(|d| d.posts.len()).sum();
    Ok(Json(CalendarResponse { days, total }))
}

fn parse_date_or_datetime(s: &str) -> Option<DateTime<Utc>> {
    // Try full ISO8601 first
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    // Try date-only
    if let Ok(naive) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let naive_dt = naive
            .and_hms_opt(0, 0, 0)?
            .and_local_timezone(Utc)
            .single()?;
        return Some(naive_dt);
    }
    None
}
