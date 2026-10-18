// ─── MCP Set Tools ──────────────────────────────────────────────
// Reusable post templates bound to channels. Calls the REST handlers (rest_bridge).

use axum::extract::{Path, State};
use rmcp::{schemars::JsonSchema, Json};
use serde::{Deserialize, Serialize};

use crate::api::sets::{self, CreateSetRequest};
use crate::api::AppState;
use crate::auth::middleware::AuthenticatedUser;

use super::rest_bridge::{parse_id, rest_result};
use super::tools_posts::resolve_first_user;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SetListInput {}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SetCreateInput {
    /// Set name (required)
    pub name: String,
    pub description: Option<String>,
    /// Template content, as stored by the composer
    pub content: serde_json::Value,
    /// Channel (integration) IDs the set publishes to (UUIDs)
    pub channel_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SetDeleteInput {
    /// Set ID (UUID)
    pub set_id: String,
}

/// List post sets.
pub async fn handle_set_list(
    state: &AppState,
    _input: &SetListInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_first_user(state).await?;
    rest_result(sets::list_sets(State(state.clone()), AuthenticatedUser { user_id }).await)
}

/// Create a post set bound to channels.
pub async fn handle_set_create(
    state: &AppState,
    input: &SetCreateInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_first_user(state).await?;
    let channel_ids = input
        .channel_ids
        .iter()
        .map(|c| parse_id("channel_ids", c))
        .collect::<Result<Vec<_>, _>>()?;
    let body = CreateSetRequest {
        name: input.name.clone(),
        description: input.description.clone(),
        content: input.content.clone(),
        channel_ids,
    };
    rest_result(
        sets::create_set(
            State(state.clone()),
            AuthenticatedUser { user_id },
            axum::Json(body),
        )
        .await,
    )
}

/// Delete a post set.
pub async fn handle_set_delete(
    state: &AppState,
    input: &SetDeleteInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_first_user(state).await?;
    let id = parse_id("set_id", &input.set_id)?;
    rest_result(sets::delete_set(State(state.clone()), AuthenticatedUser { user_id }, Path(id)).await)
}
