// ─── MCP Skool Tool ─────────────────────────────────────────────
// Skool publish wrapper (Cookie auth, no read methods).
// Uses Chrome extension to capture the skool.com session auth_token.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::api::AppState;
use crate::social::skool::SkoolProvider;
use crate::social::{PostContent, SocialProvider};

// ── Input Types ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SkPublishInput {
    pub group_id: String,
    pub title: String,
    pub content: String,
    pub label: Option<String>,
}

// ── Helpers ──────────────────────────────────────────────────

/// Find the Skool integration and return its access token (session cookie).
async fn find_sk_token(state: &AppState, user_id: Uuid) -> Result<String, String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    integrations
        .into_iter()
        .find(|i| i.provider_identifier == "skool")
        .map(|i| i.access_token)
        .ok_or_else(|| {
            "No Skool integration found. Connect Skool first via integrations_connect."
                .to_string()
        })
}

/// Create a SkoolProvider instance (no config needed).
fn create_sk_provider() -> SkoolProvider {
    SkoolProvider::new()
}

// ── Tool Implementations ─────────────────────────────────────

/// Publish a post to a Skool group.
pub async fn handle_sk_publish(
    state: &AppState,
    input: &SkPublishInput,
) -> Result<Json<Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_sk_token(state, user_id).await?;
    let provider = create_sk_provider();

    let post = PostContent {
        content: input.content.clone(),
        media: vec![],
        settings: json!({
            "groupId": input.group_id,
            "title": input.title,
            "label": input.label.clone().unwrap_or_default(),
        }),
    in_reply_to: None,
    idempotency_key: None,
    delay_minutes: None,
    };

    let result = provider
        .publish(&token, &post)
        .await
        .map_err(|e| format!("Skool publish failed: {e}"))?;

    Ok(Json(json!(result)))
}

// ── New Tool: Get Community Info ──────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SkGetInfoInput {
    pub community_slug: String,
}

/// Get community information (name, description, member count, etc.).
pub async fn handle_sk_get_info(
    state: &AppState,
    input: &SkGetInfoInput,
) -> Result<Json<Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_sk_token(state, user_id).await?;
    let provider = create_sk_provider();

    let result = provider
        .get_community_info(&input.community_slug, &token)
        .await
        .map_err(|e| format!("Skool get_community_info failed: {e}"))?;

    Ok(Json(result))
}

// ── New Tool: List Posts ─────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SkListPostsInput {
    pub community_slug: String,
    pub page: Option<u32>,
    pub sort: Option<String>,
    pub category: Option<String>,
}

/// List posts in a community with optional pagination/sort/category filters.
pub async fn handle_sk_list_posts(
    state: &AppState,
    input: &SkListPostsInput,
) -> Result<Json<Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_sk_token(state, user_id).await?;
    let provider = create_sk_provider();

    let result = provider
        .list_posts(
            &input.community_slug,
            &token,
            input.page,
            input.sort.as_deref(),
            input.category.as_deref(),
        )
        .await
        .map_err(|e| format!("Skool list_posts failed: {e}"))?;

    Ok(Json(result))
}

// ── New Tool: Get Post ───────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SkGetPostInput {
    pub community_slug: String,
    pub post_slug: String,
}

/// Get a single post by community slug and post slug.
pub async fn handle_sk_get_post(
    state: &AppState,
    input: &SkGetPostInput,
) -> Result<Json<Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_sk_token(state, user_id).await?;
    let provider = create_sk_provider();

    let result = provider
        .get_post(&input.community_slug, &input.post_slug, &token)
        .await
        .map_err(|e| format!("Skool get_post failed: {e}"))?;

    Ok(Json(result))
}

// ── New Tool: Create Comment ──────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SkCreateCommentInput {
    pub post_id: String,
    pub group_id: String,
    pub content: String,
}

/// Create a comment on a Skool post.
pub async fn handle_sk_create_comment(
    state: &AppState,
    input: &SkCreateCommentInput,
) -> Result<Json<Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_sk_token(state, user_id).await?;
    let provider = create_sk_provider();

    let result = provider
        .create_comment(&input.post_id, &input.group_id, &input.content, &token)
        .await
        .map_err(|e| format!("Skool create_comment failed: {e}"))?;

    Ok(Json(result))
}
