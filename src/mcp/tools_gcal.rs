// ─── MCP Google Calendar Tools ────────────────────────────────
// Google Calendar API v3 tools via CalendarProvider.
// Uses Google OAuth (reusing YOUTUBE_CLIENT_ID credentials).

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::crypto;
use crate::social::calendar::CalendarProvider;

// ── Input Types ───────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GcalListCalendarsInput {
    /// Max results (default 50)
    pub max_results: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GcalListEventsInput {
    /// Calendar ID (default: 'primary')
    pub calendar_id: Option<String>,
    /// Max results (default 20)
    pub max_results: Option<u32>,
    /// Start of time range in ISO 8601 (default: now)
    pub time_min: Option<String>,
    /// End of time range in ISO 8601 (default: +1 week)
    pub time_max: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GcalCreateEventInput {
    /// Calendar ID (default: 'primary')
    pub calendar_id: Option<String>,
    pub summary: String,
    pub description: Option<String>,
    /// Start time in ISO 8601 (e.g. "2026-05-15T10:00:00Z")
    pub start_time: String,
    /// End time in ISO 8601
    pub end_time: String,
    /// Timezone (default: "UTC")
    pub timezone: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GcalGetEventInput {
    /// Calendar ID (default: 'primary')
    pub calendar_id: Option<String>,
    pub event_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GcalUpdateEventInput {
    /// Calendar ID (default: 'primary')
    pub calendar_id: Option<String>,
    pub event_id: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GcalDeleteEventInput {
    /// Calendar ID (default: 'primary')
    pub calendar_id: Option<String>,
    pub event_id: String,
}

// ── Helpers ───────────────────────────────────────────────────

async fn find_gcal_token(state: &AppState, user_id: Uuid) -> Result<String, String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let cal = integrations
        .iter()
        .find(|i| i.provider_identifier == "calendar")
        .ok_or_else(|| "Google Calendar not connected. Connect it via the onboarding page first.".to_string())?;

    let tok = cal.access_token.clone();
    let tok = state.token_key.as_ref()
        .and_then(|k| crypto::decrypt_string(&tok, k).ok())
        .unwrap_or(tok);
    Ok(tok)
}

fn create_gcal_provider(state: &AppState) -> CalendarProvider {
    CalendarProvider::new(&state.config)
}

// ── Handlers ──────────────────────────────────────────────────

pub async fn handle_gcal_list_calendars(
    state: &AppState,
    _input: &GcalListCalendarsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gcal_token(state, user_id).await?;
    let provider = create_gcal_provider(state);
    let result = provider
        .list_calendars(&token)
        .await
        .map_err(|e| format!("Calendar list failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gcal_list_events(
    state: &AppState,
    input: &GcalListEventsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gcal_token(state, user_id).await?;
    let provider = create_gcal_provider(state);
    let cal_id = input.calendar_id.as_deref().unwrap_or("primary");
    let max_results = input.max_results.unwrap_or(20);
    let result = provider
        .list_events(&token, cal_id, max_results, input.time_min.as_deref(), input.time_max.as_deref())
        .await
        .map_err(|e| format!("Calendar list events failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gcal_get_event(
    state: &AppState,
    input: &GcalGetEventInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gcal_token(state, user_id).await?;
    let provider = create_gcal_provider(state);
    let cal_id = input.calendar_id.as_deref().unwrap_or("primary");
    let result = provider
        .get_event(&token, cal_id, &input.event_id)
        .await
        .map_err(|e| format!("Calendar get event failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gcal_create_event(
    state: &AppState,
    input: &GcalCreateEventInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gcal_token(state, user_id).await?;
    let provider = create_gcal_provider(state);
    let cal_id = input.calendar_id.as_deref().unwrap_or("primary");
    let result = provider
        .create_event(&token, cal_id, &input.summary, &input.start_time, &input.end_time, input.description.as_deref())
        .await
        .map_err(|e| format!("Calendar create event failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gcal_update_event(
    state: &AppState,
    input: &GcalUpdateEventInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gcal_token(state, user_id).await?;
    let provider = create_gcal_provider(state);
    let cal_id = input.calendar_id.as_deref().unwrap_or("primary");
    let result = provider
        .update_event(&token, cal_id, &input.event_id, input.summary.as_deref(), input.description.as_deref())
        .await
        .map_err(|e| format!("Calendar update event failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gcal_delete_event(
    state: &AppState,
    input: &GcalDeleteEventInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gcal_token(state, user_id).await?;
    let provider = create_gcal_provider(state);
    let cal_id = input.calendar_id.as_deref().unwrap_or("primary");
    let result = provider
        .delete_event(&token, cal_id, &input.event_id)
        .await
        .map_err(|e| format!("Calendar delete event failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}
