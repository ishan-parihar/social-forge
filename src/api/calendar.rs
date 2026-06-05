// ─── Calendar API Routes ──────────────────────────────────────
// Date-range queries for the content calendar.
// Returns posts grouped by date for calendar grid rendering.

use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::api::tags::TagResponse;
use crate::auth::middleware::AuthenticatedUser;
use crate::db::queries;
use crate::error::AppError;
use super::AppState;

#[derive(Debug, Deserialize)]
pub struct CalendarQuery {
    pub start: String, // ISO8601 date or datetime
    pub end: String,
}

#[derive(Debug, Serialize)]
pub struct CalendarPost {
    pub id: Uuid,
    pub integration_id: Uuid,
    pub integration_name: String,
    pub state: String,
    pub content: String,
    pub title: Option<String>,
    pub media: serde_json::Value,
    pub scheduled_at: Option<String>,
    pub published_at: Option<String>,
    pub platform_post_url: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub tags: Vec<TagResponse>,
    pub repeat_interval_days: Option<i32>,
    pub repeat_end_date: Option<String>,
    pub group_id: Option<Uuid>,
    pub first_comment: Option<String>,
    pub sequence: i32,
    // Engagement metrics (optional — only populated when analytics_cache has data)
    pub likes: Option<i64>,
    pub comments: Option<i64>,
    pub shares: Option<i64>,
    pub impressions: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct CalendarDay {
    pub date: String,
    pub posts: Vec<CalendarPost>,
    pub post_count: usize,
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

    let posts = queries::get_calendar_posts_with_metrics(&state.db, auth.user_id, start, end).await?;

    let mut day_map: std::collections::BTreeMap<String, Vec<CalendarPost>> = std::collections::BTreeMap::new();
    for p in posts {
        // Use published_at for published posts, scheduled_at for queued/draft posts
        let date_key = if p.state == "published" && p.published_at.is_some() {
            p.published_at.map(|d| d.format("%Y-%m-%d").to_string())
        } else {
            p.scheduled_at.map(|d| d.format("%Y-%m-%d").to_string())
        }.unwrap_or_else(|| "unscheduled".into());

        let integration_name = p.integration_name.unwrap_or_else(|| "Unknown".into());

        let tags = queries::get_tags_for_post(&state.db, p.id, auth.user_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|row| TagResponse {
                id: row.id,
                name: row.name,
                color: row.color,
                created_at: row.created_at.to_rfc3339(),
                updated_at: row.updated_at.to_rfc3339(),
            })
            .collect();

        day_map.entry(date_key).or_default().push(CalendarPost {
            id: p.id,
            integration_id: p.integration_id,
            integration_name,
            state: p.state,
            content: p.content,
            title: p.title,
            media: p.media,
            scheduled_at: p.scheduled_at.map(|d| d.to_rfc3339()),
            published_at: p.published_at.map(|d| d.to_rfc3339()),
            platform_post_url: p.platform_post_url,
            error_message: p.error_message,
            created_at: p.created_at.to_rfc3339(),
            tags,
            repeat_interval_days: p.repeat_interval_days,
            repeat_end_date: p.repeat_end_date.map(|d| d.to_rfc3339()),
            group_id: p.group_id,
            first_comment: p.first_comment,
            sequence: p.sequence,
            likes: p.likes,
            comments: p.comments,
            shares: p.shares,
            impressions: p.impressions,
        });
    }

    let days: Vec<CalendarDay> = day_map
        .into_iter()
        .map(|(date, posts)| {
            let post_count = posts.len();
            CalendarDay { date, posts, post_count }
        })
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
