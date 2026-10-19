// ─── MCP Campaign Tools ─────────────────────────────────────────
// Campaigns and sets call the REST handlers in `crate::api` (see rest_bridge).

use axum::extract::{Path, State};
use rmcp::{schemars::JsonSchema, Json};
use serde::{Deserialize, Serialize};

use crate::api::campaigns::{self, CreateCampaignRequest, UpdateCampaignRequest};
use crate::api::AppState;
use crate::auth::middleware::AuthenticatedUser;

use super::rest_bridge::{parse_id, rest_result};
use super::tools_posts::resolve_first_user;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CampaignListInput {}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CampaignCreateInput {
    /// Campaign name (required)
    pub name: String,
    pub description: Option<String>,
    /// Hex colour, defaults to indigo
    pub color: Option<String>,
    /// YYYY-MM-DD
    pub start_date: Option<String>,
    /// YYYY-MM-DD
    pub end_date: Option<String>,
    pub goal: Option<String>,
    /// active | paused | archived | completed
    pub status: Option<String>,
    /// posts | engagement | reach | followers | custom
    pub progress_metric: Option<String>,
    pub progress_target: Option<i32>,
    pub audience_persona: Option<serde_json::Value>,
    pub content_pillars: Option<serde_json::Value>,
    pub budget_cents: Option<i32>,
    pub kpi_targets: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CampaignUpdateInput {
    /// Campaign ID (UUID)
    pub campaign_id: String,
    pub name: Option<String>,
    /// Three-state: omitted = keep, `null` = clear, a value = replace.
    #[serde(default, deserialize_with = "double_option")]
    pub description: Option<Option<String>>,
    pub color: Option<String>,
    /// Three-state: omitted = keep, `null` = clear, a value = replace.
    #[serde(default, deserialize_with = "double_option")]
    pub start_date: Option<Option<String>>,
    /// Three-state: omitted = keep, `null` = clear, a value = replace.
    #[serde(default, deserialize_with = "double_option")]
    pub end_date: Option<Option<String>>,
    /// Three-state: omitted = keep, `null` = clear, a value = replace.
    #[serde(default, deserialize_with = "double_option")]
    pub goal: Option<Option<String>>,
    /// active | paused | archived | completed
    pub status: Option<String>,
    /// Three-state: omitted = keep, `null` = clear, a value = replace.
    #[serde(default, deserialize_with = "double_option")]
    pub progress_metric: Option<Option<String>>,
    /// Three-state: omitted = keep, `null` = clear, a value = replace.
    #[serde(default, deserialize_with = "double_option")]
    pub progress_target: Option<Option<i32>>,
    /// Three-state: omitted = keep, `null` = clear, a value = replace.
    #[serde(default, deserialize_with = "double_option")]
    pub audience_persona: Option<Option<serde_json::Value>>,
    /// Three-state: omitted = keep, `null` = clear, a value = replace.
    #[serde(default, deserialize_with = "double_option")]
    pub content_pillars: Option<Option<serde_json::Value>>,
    /// Three-state: omitted = keep, `null` = clear, a value = replace.
    #[serde(default, deserialize_with = "double_option")]
    pub budget_cents: Option<Option<i32>>,
    /// Three-state: omitted = keep, `null` = clear, a value = replace.
    #[serde(default, deserialize_with = "double_option")]
    pub kpi_targets: Option<Option<serde_json::Value>>,
}

/// Deserialize a field into `Option<Option<T>>` so "absent", "null" and
/// "a value" are three distinguishable states.
///
/// `Option<T>` collapses "absent" and "null" into the same `None`. The
/// MCP input used that shape and then mapped it with `.map(Some)`,
/// which turned "clear this field" into "leave this field alone" - so
/// `campaign_update {goal: null}` kept the goal and reported success.
fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    serde::Deserialize::deserialize(deserializer).map(Some)
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CampaignDeleteInput {
    /// Campaign ID (UUID). Soft-deletes and archives the campaign.
    pub campaign_id: String,
}

/// List campaigns, including archived ones.
pub async fn handle_campaign_list(
    state: &AppState,
    _input: &CampaignListInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_first_user(state).await?;
    rest_result(campaigns::list(State(state.clone()), AuthenticatedUser { user_id }).await)
}

/// Create a campaign.
pub async fn handle_campaign_create(
    state: &AppState,
    input: &CampaignCreateInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_first_user(state).await?;
    let body = CreateCampaignRequest {
        name: input.name.clone(),
        description: input.description.clone(),
        color: input.color.clone(),
        start_date: input.start_date.clone(),
        end_date: input.end_date.clone(),
        goal: input.goal.clone(),
        status: input.status.clone(),
        progress_metric: input.progress_metric.clone(),
        progress_target: input.progress_target,
        audience_persona: input.audience_persona.clone(),
        content_pillars: input.content_pillars.clone(),
        budget_cents: input.budget_cents,
        kpi_targets: input.kpi_targets.clone(),
    };
    rest_result(
        campaigns::create(
            State(state.clone()),
            AuthenticatedUser { user_id },
            axum::Json(body),
        )
        .await,
    )
}

/// Update a campaign. Omitted fields keep their current value.
pub async fn handle_campaign_update(
    state: &AppState,
    input: &CampaignUpdateInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_first_user(state).await?;
    let id = parse_id("campaign_id", &input.campaign_id)?;
    let body = UpdateCampaignRequest {
        name: input.name.clone(),
        color: input.color.clone(),
        status: input.status.clone(),
        // The clearable fields are three-state on the API (absent = keep,
        // null = clear, a value = replace) and three-state here too, so
        // they pass straight through. The MCP input used to be
        // two-state and mapped with `.map(Some)`, which turned "clear
        // this field" into "leave this field alone": `campaign_update
        // {goal: null}` kept the goal and reported success.
        description: input.description.clone(),
        start_date: input.start_date.clone(),
        end_date: input.end_date.clone(),
        goal: input.goal.clone(),
        progress_metric: input.progress_metric.clone(),
        progress_target: input.progress_target,
        audience_persona: input.audience_persona.clone(),
        content_pillars: input.content_pillars.clone(),
        budget_cents: input.budget_cents,
        kpi_targets: input.kpi_targets.clone(),
    };
    rest_result(
        campaigns::update(
            State(state.clone()),
            AuthenticatedUser { user_id },
            Path(id),
            axum::Json(body),
        )
        .await,
    )
}

/// Soft-delete a campaign.
pub async fn handle_campaign_delete(
    state: &AppState,
    input: &CampaignDeleteInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_first_user(state).await?;
    let id = parse_id("campaign_id", &input.campaign_id)?;
    rest_result(
        campaigns::delete(State(state.clone()), AuthenticatedUser { user_id }, Path(id)).await,
    )
}
