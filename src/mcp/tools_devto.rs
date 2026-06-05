// ─── MCP Dev.to Tools ────────────────────────────────────────────
// Dev.to API v0 tools via the DevtoProvider (API key-based).

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::api::AppState;
use crate::social::devto::DevtoProvider;
use crate::social::{PostContent, SocialProvider};

// ── Input Types ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DvCreatePostInput {
    /// JWT auth token from login
    pub token: String,
    /// Article title (optional; derived from content first line if absent)
    pub title: Option<String>,
    /// Article body (markdown)
    pub content: String,
    /// Tags for the article (max 4)
    pub tags: Option<Vec<String>>,
    /// Publish immediately (default: false, saves as draft)
    pub published: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DvListPostsInput {
    /// JWT auth token from login
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DvGetPostInput {
    /// JWT auth token from login
    pub token: String,
    /// Dev.to article ID
    pub article_id: String,
}

// ── Helpers ──────────────────────────────────────────────────

async fn find_devto_token(state: &AppState, user_id: Uuid) -> Result<String, String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    integrations
        .into_iter()
        .find(|i| i.provider_identifier == "devto")
        .map(|i| i.access_token)
        .ok_or_else(|| {
            "No Dev.to integration found. Connect Dev.to first via integrations_connect."
                .to_string()
        })
}

fn create_devto_provider(state: &AppState) -> DevtoProvider {
    DevtoProvider::new(&state.config)
}

// ── Tool Implementations ─────────────────────────────────────

pub async fn handle_dv_create_post(
    state: &AppState,
    input: &DvCreatePostInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_devto_token(state, user_id).await?;
    let provider = create_devto_provider(state);

    let mut settings = json!({});
    if let Some(title) = &input.title {
        settings["title"] = json!(title);
    }
    if let Some(tags) = &input.tags {
        settings["tags"] = json!(tags);
    }
    if let Some(published) = input.published {
        settings["published"] = json!(published);
    }

    let post = PostContent {
        content: input.content.clone(),
        media: vec![],
        settings,
    };

    let result = provider
        .publish(&token, &post)
        .await
        .map_err(|e| format!("Dev.to publish failed: {e}"))?;

    Ok(Json(json!(result)))
}

pub async fn handle_dv_list_posts(
    state: &AppState,
    _input: &DvListPostsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_devto_token(state, user_id).await?;
    let provider = create_devto_provider(state);

    let pages = provider
        .pages(&token)
        .await
        .map_err(|e| format!("Dev.to list posts failed: {e}"))?;

    Ok(Json(json!({ "articles": pages })))
}

pub async fn handle_dv_get_post(
    state: &AppState,
    input: &DvGetPostInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_devto_token(state, user_id).await?;
    let provider = create_devto_provider(state);

    // Use fetch_page_info which tries the article endpoint first
    let page = provider
        .fetch_page_info(&token, &input.article_id)
        .await
        .map_err(|e| format!("Dev.to get post failed: {e}"))?;

    Ok(Json(json!({ "article": page })))
}
