// ─── MCP Medium Tools ────────────────────────────────────────────
// Medium API v1 tools via the MediumProvider (API key-based).

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::api::AppState;
use crate::social::medium::MediumProvider;
use crate::social::{PostContent, SocialProvider};

// ── Input Types ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MdCreatePostInput {
    /// Post title (optional; derived from content first line if absent)
    pub title: Option<String>,
    /// Post body content (markdown)
    pub content: String,
    /// Tags for the post
    pub tags: Option<Vec<String>>,
    /// Publish status: "draft", "public", or "unlisted" (default: "draft")
    pub publish_status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MdListPostsInput {
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MdGetPostInput {
    /// Medium post ID
    pub post_id: String,
}

// ── Helpers ──────────────────────────────────────────────────

async fn find_medium_token(state: &AppState, user_id: Uuid) -> Result<String, String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    integrations
        .into_iter()
        .find(|i| i.provider_identifier == "medium")
        .map(|i| i.access_token)
        .ok_or_else(|| {
            "No Medium integration found. Connect Medium first via integrations_connect."
                .to_string()
        })
}

fn create_medium_provider(state: &AppState) -> MediumProvider {
    MediumProvider::new(&state.config)
}

// ── Tool Implementations ─────────────────────────────────────

pub async fn handle_md_create_post(
    state: &AppState,
    input: &MdCreatePostInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_medium_token(state, user_id).await?;
    let provider = create_medium_provider(state);

    let mut settings = json!({});
    if let Some(title) = &input.title {
        settings["title"] = json!(title);
    }
    if let Some(tags) = &input.tags {
        settings["tags"] = json!(tags);
    }
    if let Some(publish_status) = &input.publish_status {
        settings["publish_status"] = json!(publish_status);
    }

    let post = PostContent {
        content: input.content.clone(),
        media: vec![],
        settings,
    in_reply_to: None,
    };

    let result = provider
        .publish(&token, &post)
        .await
        .map_err(|e| format!("Medium publish failed: {e}"))?;

    Ok(Json(json!(result)))
}

pub async fn handle_md_list_posts(
    state: &AppState,
    _input: &MdListPostsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_medium_token(state, user_id).await?;
    let provider = create_medium_provider(state);

    let pages = provider
        .pages(&token)
        .await
        .map_err(|e| format!("Medium list posts failed: {e}"))?;

    Ok(Json(json!({ "pages": pages })))
}

pub async fn handle_md_get_post(
    state: &AppState,
    input: &MdGetPostInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_medium_token(state, user_id).await?;
    let provider = create_medium_provider(state);

    let pages = provider
        .pages(&token)
        .await
        .map_err(|e| format!("Medium get post failed: {e}"))?;

    let page = pages.into_iter().next();

    Ok(Json(json!({
        "user": page,
        "requested_post_id": input.post_id,
        "note": "Medium API v1 does not provide a get-post endpoint. Use post_id for reference only."
    })))
}
