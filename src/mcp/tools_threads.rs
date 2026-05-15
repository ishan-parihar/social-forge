// ─── MCP Threads Tools ─────────────────────────────────────────
// Meta Threads API v1.0 via graph.threads.net.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::crypto;
use crate::social::threads::ThreadsProvider;
use crate::social::SocialProvider;

// ── Input/Output Types ──────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ThreadsGetProfileInput {
    pub threads_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ThreadsGetThreadsInput {
    pub threads_id: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ThreadsGetThreadDetailInput {
    pub threads_id: String,
    pub media_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ThreadsGetRepliesInput {
    pub threads_id: String,
    pub media_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ThreadsReplyToThreadInput {
    pub threads_id: String,
    pub media_id: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ThreadsCreateThreadInput {
    pub threads_id: String,
    pub text: String,
    pub media_url: Option<String>,
    pub media_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ThreadsDeleteThreadInput {
    pub threads_id: String,
    pub media_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ThreadsGetInsightsInput {
    pub threads_id: String,
    pub metric: String,
    pub period: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ThreadsPollStatusInput {
    pub threads_id: String,
    pub creation_id: String,
}

// ── Helpers ──────────────────────────────────────────────────

async fn find_threads_token(
    state: &AppState,
    user_id: Uuid,
    threads_id: &str,
) -> Result<String, String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let threads = integrations
        .iter()
        .find(|i| i.provider_identifier == "threads" && i.internal_id == threads_id)
        .ok_or_else(|| {
            format!(
                "Threads account '{threads_id}' not connected. Connect it via the onboarding page first."
            )
        })?;

    let __tok = threads.access_token.clone();
    let __tok = state.token_key.as_ref()
        .and_then(|k| crate::crypto::decrypt_string(&__tok, k).ok())
        .unwrap_or(__tok);
    Ok(__tok)
}

fn create_provider(state: &AppState) -> ThreadsProvider {
    ThreadsProvider::new(&state.config)
}

// ── Tool Implementations ─────────────────────────────────────

pub async fn handle_th_get_profile(
    state: &AppState,
    input: &ThreadsGetProfileInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_threads_token(state, user_id, &input.threads_id).await?;
    let provider = create_provider(state);
    let result = provider
        .get_profile(&token)
        .await
        .map_err(|e| format!("Threads get profile failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_th_get_threads(
    state: &AppState,
    input: &ThreadsGetThreadsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_threads_token(state, user_id, &input.threads_id).await?;
    let provider = create_provider(state);
    let limit = input.limit.unwrap_or(20);
    let result = provider
        .get_threads(&token, &input.threads_id, limit)
        .await
        .map_err(|e| format!("Threads get threads failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_th_get_thread_detail(
    state: &AppState,
    input: &ThreadsGetThreadDetailInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_threads_token(state, user_id, &input.threads_id).await?;
    let provider = create_provider(state);
    let result = provider
        .get_thread_detail(&token, &input.media_id)
        .await
        .map_err(|e| format!("Threads get thread detail failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_th_get_replies(
    state: &AppState,
    input: &ThreadsGetRepliesInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_threads_token(state, user_id, &input.threads_id).await?;
    let provider = create_provider(state);
    let result = provider
        .get_thread_replies(&token, &input.media_id)
        .await
        .map_err(|e| format!("Threads get replies failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_th_reply_to_thread(
    state: &AppState,
    input: &ThreadsReplyToThreadInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_threads_token(state, user_id, &input.threads_id).await?;
    let provider = create_provider(state);
    let result = provider
        .reply_to_thread(&token, &input.media_id, &input.message)
        .await
        .map_err(|e| format!("Threads reply failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_th_create_thread(
    state: &AppState,
    input: &ThreadsCreateThreadInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_threads_token(state, user_id, &input.threads_id).await?;
    let provider = create_provider(state);

    let post = crate::social::PostContent {
        content: input.text.clone(),
        media: if let Some(url) = &input.media_url {
            vec![crate::social::MediaAttachment {
                url: url.clone(),
                mime_type: input
                    .media_type
                    .as_ref()
                    .map(|t| match t.to_lowercase().as_str() {
                        "video" | "mp4" => "video/mp4".to_string(),
                        "image" | "jpg" | "jpeg" | "png" | "gif" | "webp" => {
                            "image/jpeg".to_string()
                        }
                        _ => "application/octet-stream".to_string(),
                    })
                    .unwrap_or_else(|| "application/octet-stream".to_string()),
                alt: None,
            }]
        } else {
            vec![]
        },
        settings: serde_json::Value::Object(serde_json::Map::new()),
    };

    let result = provider
        .publish(&token, &post)
        .await
        .map_err(|e| format!("Threads create thread failed: {e}"))?;

    Ok(Json(serde_json::json!({
        "data": {
            "id": result.platform_post_id,
            "url": result.platform_post_url,
            "status": result.status,
        }
    })))
}

pub async fn handle_th_delete_thread(
    state: &AppState,
    input: &ThreadsDeleteThreadInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_threads_token(state, user_id, &input.threads_id).await?;
    let provider = create_provider(state);
    let result = provider
        .delete_thread(&token, &input.media_id)
        .await
        .map_err(|e| format!("Threads delete thread failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_th_get_insights(
    state: &AppState,
    input: &ThreadsGetInsightsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_threads_token(state, user_id, &input.threads_id).await?;
    let provider = create_provider(state);
    let period = input.period.as_deref().unwrap_or("day");
    let result = provider
        .get_insights(&token, &input.threads_id, &input.metric, period)
        .await
        .map_err(|e| format!("Threads get insights failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_th_poll_publish_status(
    state: &AppState,
    input: &ThreadsPollStatusInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_threads_token(state, user_id, &input.threads_id).await?;
    let provider = create_provider(state);

    let result = provider
        .get_thread_detail(&token, &input.creation_id)
        .await
        .map_err(|e| format!("Threads poll status failed: {e}"))?;

    let status = if result.get("id").is_some() {
        "published"
    } else {
        "pending"
    };

    Ok(Json(serde_json::json!({
        "data": {
            "creation_id": input.creation_id,
            "status": status,
            "thread": result,
        }
    })))
}