// ─── MCP Calendar Tools ───────────────────────────────────────
// Date-range queries for the content calendar.

use chrono::{DateTime, NaiveDate, Utc};
use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};

use crate::api::AppState;
use crate::db::queries;

// ── Input/Output Types ──────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CalendarInput {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CalendarPost {
    pub id: String,
    pub state: String,
    pub content_preview: String,
    pub scheduled_at: Option<String>,
    pub platform_post_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CalendarDay {
    pub date: String,
    pub posts: Vec<CalendarPost>,
    pub post_count: i32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CalendarOutput {
    pub days: Vec<CalendarDay>,
    pub total_posts: i32,
}

// ── Tool Implementation ────────────────────────────────────

pub async fn get_calendar(
    state: &AppState,
    input: &CalendarInput,
) -> Result<Json<CalendarOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;

    let start = parse_date_or_datetime(&input.start)
        .ok_or_else(|| "Invalid start date. Use YYYY-MM-DD or ISO8601".to_string())?;
    let end = parse_date_or_datetime(&input.end)
        .ok_or_else(|| "Invalid end date".to_string())?;

    let posts = queries::get_posts_by_date_range(&state.db, user_id, start, end)
        .await
        .map_err(|e| e.to_string())?;

    // Group by date
    let mut day_map: std::collections::BTreeMap<String, Vec<CalendarPost>> =
        std::collections::BTreeMap::new();
    for p in posts {
        let date_key = p
            .scheduled_at
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "unscheduled".into());

        let preview = p.content.chars().take(100).collect::<String>();
        day_map
            .entry(date_key)
            .or_default()
            .push(CalendarPost {
                id: p.id.to_string(),
                state: p.state.to_string(),
                content_preview: if p.content.len() > 100 {
                    format!("{preview}...")
                } else {
                    preview
                },
                scheduled_at: p.scheduled_at.map(|d| d.to_rfc3339()),
                platform_post_url: p.platform_post_url,
            });
    }

    let days: Vec<CalendarDay> = day_map
        .into_iter()
        .map(|(date, posts)| {
            let count = posts.len() as i32;
            CalendarDay {
                date,
                posts,
                post_count: count,
            }
        })
        .collect();

    let total: i32 = days.iter().map(|d| d.post_count).sum();

    Ok(Json(CalendarOutput { days, total_posts: total }))
}

fn parse_date_or_datetime(s: &str) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(naive) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let naive_dt = naive.and_hms_opt(0, 0, 0)?;
        return Some(naive_dt.and_utc());
    }
    None
}
