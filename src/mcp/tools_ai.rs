// ─── MCP AI Writing Tools ───────────────────────────────────────
// Thin wrappers over the REST AI handlers in `crate::api::ai` (rest_bridge).

use axum::extract::State;
use rmcp::{schemars::JsonSchema, Json};
use serde::{Deserialize, Serialize};

use crate::api::ai::{self, ChangeToneRequest, ContentRequest, GenerateBulkRequest, GeneratePostRequest};
use crate::api::AppState;
use crate::auth::middleware::AuthenticatedUser;

use super::rest_bridge::rest_result;
use super::tools_posts::resolve_first_user;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AiGeneratePostInput {
    /// What the post is about
    pub topic: String,
    /// Tone, for example "casual" or "professional"
    pub tone: String,
    /// Length, for example "short" or "long"
    pub length: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AiContentInput {
    /// Post text to work on
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AiChangeToneInput {
    pub content: String,
    pub tone: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AiGenerateBulkInput {
    pub topic: String,
    /// one_short | one_long | thread_short | thread_long
    pub format: String,
    /// personal | company
    pub tone: String,
    /// How many posts to generate (default 3, max 5)
    pub count: Option<u32>,
}

/// Draft a post from a topic, tone and length.
pub async fn handle_ai_generate_post(
    state: &AppState,
    input: &AiGeneratePostInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_first_user(state).await?;
    let body = GeneratePostRequest {
        topic: input.topic.clone(),
        tone: input.tone.clone(),
        length: input.length.clone(),
    };
    rest_result(
        ai::generate_post(State(state.clone()), AuthenticatedUser { user_id }, axum::Json(body))
            .await,
    )
}

/// Improve the wording of existing text.
pub async fn handle_ai_improve_writing(
    state: &AppState,
    input: &AiContentInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_first_user(state).await?;
    let body = ContentRequest { content: input.content.clone() };
    rest_result(
        ai::improve_writing(State(state.clone()), AuthenticatedUser { user_id }, axum::Json(body))
            .await,
    )
}

/// Suggest hashtags for text.
pub async fn handle_ai_suggest_hashtags(
    state: &AppState,
    input: &AiContentInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_first_user(state).await?;
    let body = ContentRequest { content: input.content.clone() };
    rest_result(
        ai::suggest_hashtags(State(state.clone()), AuthenticatedUser { user_id }, axum::Json(body))
            .await,
    )
}

/// Rewrite text in a different tone.
pub async fn handle_ai_change_tone(
    state: &AppState,
    input: &AiChangeToneInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_first_user(state).await?;
    let body = ChangeToneRequest {
        content: input.content.clone(),
        tone: input.tone.clone(),
    };
    rest_result(
        ai::change_tone(State(state.clone()), AuthenticatedUser { user_id }, axum::Json(body))
            .await,
    )
}

/// Summarise text.
pub async fn handle_ai_summarize(
    state: &AppState,
    input: &AiContentInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_first_user(state).await?;
    let body = ContentRequest { content: input.content.clone() };
    rest_result(
        ai::summarize(State(state.clone()), AuthenticatedUser { user_id }, axum::Json(body)).await,
    )
}

/// Generate several posts on one topic in one call.
pub async fn handle_ai_generate_bulk(
    state: &AppState,
    input: &AiGenerateBulkInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_first_user(state).await?;
    let body = GenerateBulkRequest {
        topic: input.topic.clone(),
        format: input.format.clone(),
        tone: input.tone.clone(),
        count: input.count.unwrap_or(3),
    };
    rest_result(
        ai::generate_bulk(State(state.clone()), AuthenticatedUser { user_id }, axum::Json(body))
            .await,
    )
}
