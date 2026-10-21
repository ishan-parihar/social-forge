// ─── MCP Calendar Tools ───────────────────────────────────────
// Date-range queries for the content calendar.

use chrono::{DateTime, NaiveDate, Utc};
use rmcp::{schemars::JsonSchema, Json};
use serde::{Deserialize, Serialize};

use crate::api::AppState;
use crate::db::queries;

// ── Input/Output Types ──────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CalendarInput {
    pub start: String,
    pub end: String,
    /// The founder's zone as an IANA NAME ("America/New_York").
    /// Preferred over `tz_offset`: it resolves per-instant, so a range
    /// spanning a DST transition is bucketed correctly - matching
    /// GET /api/calendar, which the web client now calls with the
    /// NAME too. An unknown name is an error, not a silent UTC
    /// fallback.
    #[serde(default)]
    pub tz: Option<String>,
    /// The founder's UTC offset, in SECONDS. Legacy form: correct
    /// within a window with no DST transition, wrong across one.
    #[serde(default)]
    pub tz_offset: Option<i64>,
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
    let end = parse_date_or_datetime(&input.end).ok_or_else(|| "Invalid end date".to_string())?;

    // Zone FIRST, before any query: a bad zone must be loud no matter
    // what else the request contains, and the parse happens ONCE, not
    // per post. Same contract as GET /api/calendar (AGENTS.md 9.21: the
    // MCP surface is a CLIENT of the REST surface, so the two must
    // refuse the same malformed input the same way).
    let zone = match input.tz.as_deref() {
        Some(name) => Some(crate::api::calendar::parse_zone(name)?),
        None => None,
    };
    // The offset bound is UNCONDITIONAL: a zone name must not let a
    // garbage `tz_offset` through. The first version gated it under
    // `if zone.is_none()`, so a VALID name beside an absurd offset
    // skipped the check - the REST endpoint 400s the same pair, and
    // the two surfaces must not disagree (AGENTS.md 9.21).
    if let Some(secs) = input.tz_offset {
        if secs.unsigned_abs() > 86_400 {
            return Err(
                "tz_offset must be within plus or minus one day (in seconds)".to_string(),
            );
        }
    }

    let posts = queries::get_posts_by_date_range(&state.db, user_id, start, end)
        .await
        .map_err(|e| e.to_string())?;

    // Group by date
    let mut day_map: std::collections::BTreeMap<String, Vec<CalendarPost>> =
        std::collections::BTreeMap::new();
    for p in posts {
        // THE SHARED DAY KEY (AGENTS.md 9.21). This used to be a local
        // `d.format("%Y-%m-%d")` - always UTC - so the tool disagreed
        // with GET /api/calendar for the same operation.
        //
        // A NAME resolves per-instant (DST-correct); the constant is
        // the legacy form. Same precedence as the REST handler.
        let date_key = p
            .scheduled_at
            .map(|d| {
                let secs = zone
                    .map(|z| crate::api::calendar::zone_offset_seconds(z, d))
                    .or(input.tz_offset)
                    .unwrap_or(0);
                crate::api::calendar::local_day_key(d, Some(secs))
            })
            .unwrap_or_else(|| "unscheduled".into());

        let preview = p.content.chars().take(100).collect::<String>();
        day_map.entry(date_key).or_default().push(CalendarPost {
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

    Ok(Json(CalendarOutput {
        days,
        total_posts: total,
    }))
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
