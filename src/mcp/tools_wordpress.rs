// ─── MCP WordPress Tools ─────────────────────────────────────────
// WordPress REST API tools via the WordPressProvider (Application Password).

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::api::AppState;
use crate::social::wordpress::WordPressProvider;
use crate::social::{PostContent, SocialProvider};

// ── Input Types ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WpCreatePostInput {
    /// Post title
    pub title: String,
    /// Post body content (HTML or blocks)
    pub content: String,
    /// Publish status: "draft", "publish", "pending" (default: "draft")
    pub status: Option<String>,
    /// Category IDs to assign
    pub categories: Option<Vec<i32>>,
    /// Tag IDs to assign
    pub tags: Option<Vec<i32>>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WpListPostsInput {
    /// Filter by status: "publish", "draft", "pending", etc.
    pub status: Option<String>,
    /// Number of posts per page (default: 10)
    pub per_page: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WpGetPostInput {
    /// WordPress post ID
    pub post_id: i32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WpListCategoriesInput {
}

// ── Helpers ──────────────────────────────────────────────────

async fn find_wordpress_token(state: &AppState, user_id: Uuid) -> Result<String, String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    integrations
        .into_iter()
        .find(|i| i.provider_identifier == "wordpress")
        .map(|i| i.access_token)
        .ok_or_else(|| {
            "No WordPress integration found. Connect WordPress first via integrations_connect."
                .to_string()
        })
}

fn create_wordpress_provider(state: &AppState) -> WordPressProvider {
    WordPressProvider::new(&state.config)
}

// ── Tool Implementations ─────────────────────────────────────

pub async fn handle_wp_create_post(
    state: &AppState,
    input: &WpCreatePostInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_wordpress_token(state, user_id).await?;
    let provider = create_wordpress_provider(state);

    let mut settings = json!({
        "title": input.title,
    });

    settings["status"] = json!(input.status.as_deref().unwrap_or("draft"));

    if let Some(categories) = &input.categories {
        settings["categories"] = json!(categories);
    }
    if let Some(tags) = &input.tags {
        settings["tags"] = json!(tags);
    }

    let post = PostContent {
        content: input.content.clone(),
        media: vec![],
        settings,
    in_reply_to: None,
    idempotency_key: None,
            delay_minutes: None
    };

    let result = provider
        .publish(&token, &post)
        .await
        .map_err(|e| format!("WordPress publish failed: {e}"))?;

    Ok(Json(json!(result)))
}

pub async fn handle_wp_list_posts(
    state: &AppState,
    input: &WpListPostsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_wordpress_token(state, user_id).await?;
    let provider = create_wordpress_provider(state);

    let result = provider
        .list_posts(&token, input.status.as_deref(), input.per_page)
        .await
        .map_err(|e| format!("WordPress list posts failed: {e}"))?;

    Ok(Json(json!({ "posts": result })))
}

pub async fn handle_wp_get_post(
    state: &AppState,
    input: &WpGetPostInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_wordpress_token(state, user_id).await?;
    let provider = create_wordpress_provider(state);

    let result = provider
        .get_post(&token, input.post_id)
        .await
        .map_err(|e| format!("WordPress get post failed: {e}"))?;

    Ok(Json(json!({ "post": result })))
}

pub async fn handle_wp_list_categories(
    state: &AppState,
    _input: &WpListCategoriesInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_wordpress_token(state, user_id).await?;
    let provider = create_wordpress_provider(state);

    let result = provider
        .list_categories(&token)
        .await
        .map_err(|e| format!("WordPress list categories failed: {e}"))?;

    Ok(Json(json!({ "categories": result })))
}
