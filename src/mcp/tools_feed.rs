// ─── MCP Feed Tools ──────────────────────────────────────────
// Tools for listing imported external posts and triggering imports.
// Mirrors the REST API /api/feed endpoints for AI agent access.

use chrono::{DateTime, Utc};
use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};

use crate::api::AppState;
use crate::feed;


// ── Input/Output Types ──────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FeedListInput {
    /// ISO8601 cursor — returns posts older than this timestamp (exclusive).
    /// Omit for the first page.
    pub cursor: Option<String>,
    /// Optional provider filter (e.g. "x", "reddit", "bluesky")
    pub provider: Option<String>,
    /// Number of posts to return (max 100)
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    20
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FeedListOutput {
    pub posts: Vec<FeedPostItem>,
    /// Cursor for the next page — the created_at of the last post in this page.
    /// null if there are no more posts.
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]

pub struct FeedPostItem {
    pub id: String,
    pub provider: String,
    pub platform_post_id: String,
    pub text: String,
    pub author_name: Option<String>,
    pub author_handle: Option<String>,
    pub author_avatar: Option<String>,
    pub created_at: String,
    pub url: Option<String>,
    pub media: serde_json::Value,
    pub imported_at: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FeedImportOutput {
    pub imported: u32,
    pub status: String,
}

/// Format an ExternalPost into a FeedPostItem for MCP output
fn format_feed_post(post: &crate::db::models::ExternalPost) -> FeedPostItem {
    FeedPostItem {
        id: post.id.to_string(),
        provider: post.provider.clone(),
        platform_post_id: post.platform_post_id.clone(),
        text: post.text.clone(),
        author_name: post.author_name.clone(),
        author_handle: post.author_handle.clone(),
        author_avatar: post.author_avatar.clone(),
        created_at: post.created_at.to_rfc3339(),
        url: post.url.clone(),
        media: post.media.clone(),
        imported_at: post.imported_at.to_rfc3339(),
    }
}

// ── Tool Handlers ───────────────────────────────────────────

/// MCP tool: list imported external posts (feed), cursor-paginated.
/// Works like the REST /api/feed endpoint.
pub async fn handle_feed_list(
    state: &AppState,
    input: &FeedListInput,
) -> Result<Json<FeedListOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let limit = input.limit.clamp(1, 100);

    let cursor: Option<DateTime<Utc>> = input
        .cursor
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let provider = input.provider.as_deref();

    let posts = crate::db::queries::list_all_external_posts(
        &state.db,
        user_id,
        provider,
        cursor,
        limit + 1,
    )
    .await
    .map_err(|e| format!("DB error: {e}"))?;

    let has_more = posts.len() as i64 > limit;
    let posts: Vec<_> = posts.into_iter().take(limit as usize).collect();

    let next_cursor = posts.last().map(|p| p.created_at.to_rfc3339());

    let feed_posts: Vec<FeedPostItem> = posts.iter().map(format_feed_post).collect();

    Ok(Json(FeedListOutput {
        posts: feed_posts,
        next_cursor,
        has_more,
    }))
}

/// MCP tool: trigger an immediate import of recent posts from all connected providers.
/// Works like POST /api/feed/import.
pub async fn handle_feed_import(
    state: &AppState,
) -> Result<Json<FeedImportOutput>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;

    let count = feed::refresh_user_posts(
        &state.db,
        user_id,
        &state.providers,
        &state.broadcast,
        state.token_key,
    )
    .await
    .map_err(|e| format!("Import error: {e}"))?;

    Ok(Json(FeedImportOutput {
        imported: count,
        status: "ok".into(),
    }))
}
