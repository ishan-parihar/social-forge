// ─── MCP RSS Tools ──────────────────────────────────────────────
// RSS autopost feeds. Calls the REST handlers (rest_bridge).

use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use rmcp::{schemars::JsonSchema, Json};
use serde::{Deserialize, Serialize};

use crate::api::rss::{self, CreateFeedRequest};
use crate::api::AppState;
use crate::auth::middleware::AuthenticatedUser;

use super::rest_bridge::{parse_id, rest_result};
use super::tools_posts::resolve_first_user;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RssFeedListInput {}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RssFeedCreateInput {
    /// Feed URL (RSS or Atom)
    pub feed_url: String,
    /// Channel (integration) ID that receives the posts (UUID)
    pub integration_id: String,
    pub title: Option<String>,
    /// Draft an AI summary for each item
    pub use_ai_summary: Option<bool>,
    /// Start polling immediately (defaults to true)
    pub enabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RssFeedIdInput {
    /// Feed ID (UUID)
    pub feed_id: String,
}

/// List RSS feeds.
pub async fn handle_rss_feed_list(
    state: &AppState,
    _input: &RssFeedListInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_first_user(state).await?;
    rest_result(rss::list_feeds(State(state.clone()), AuthenticatedUser { user_id }).await)
}

/// Add an RSS feed to a channel.
pub async fn handle_rss_feed_create(
    state: &AppState,
    input: &RssFeedCreateInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_first_user(state).await?;
    let body = CreateFeedRequest {
        feed_url: input.feed_url.clone(),
        integration_id: input.integration_id.clone(),
        title: input.title.clone(),
        use_ai_summary: input.use_ai_summary,
        enabled: input.enabled,
    };
    rest_result(
        rss::create_feed(
            State(state.clone()),
            AuthenticatedUser { user_id },
            axum::Json(body),
        )
        .await,
    )
}

/// Delete an RSS feed.
pub async fn handle_rss_feed_delete(
    state: &AppState,
    input: &RssFeedIdInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_first_user(state).await?;
    let id = parse_id("feed_id", &input.feed_id)?;
    rest_result(
        rss::delete_feed(
            State(state.clone()),
            AuthenticatedUser { user_id },
            Path(id),
        )
        .await,
    )
}

/// Pause or resume an RSS feed.
pub async fn handle_rss_feed_toggle(
    state: &AppState,
    input: &RssFeedIdInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_first_user(state).await?;
    let id = parse_id("feed_id", &input.feed_id)?;
    rest_result(
        rss::toggle_feed(
            State(state.clone()),
            AuthenticatedUser { user_id },
            Path(id),
        )
        .await,
    )
}

/// Poll a feed now for new items.
pub async fn handle_rss_feed_poll(
    state: &AppState,
    input: &RssFeedIdInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_first_user(state).await?;
    let id = parse_id("feed_id", &input.feed_id)?;
    rest_result(
        rss::poll_feed(
            State(state.clone()),
            AuthenticatedUser { user_id },
            Path(id),
        )
        .await,
    )
}

/// List the items a feed has produced, with their import state.
pub async fn handle_rss_feed_items(
    state: &AppState,
    input: &RssFeedIdInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_first_user(state).await?;
    let id = parse_id("feed_id", &input.feed_id)?;
    rest_result(
        rss::list_feed_items(
            State(state.clone()),
            AuthenticatedUser { user_id },
            Path(id),
            Query(HashMap::new()),
        )
        .await,
    )
}
