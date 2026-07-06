// ─── Feed API Routes ──────────────────────────────────────────
// Scrollable feed of imported external posts (X, Reddit, Bluesky, etc.).
// Cursor-paginated by created_at DESC for infinite scroll.
// Engagement metrics from post_engagement table are LEFT JOINed.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::queries;
use crate::error::AppError;
use crate::feed;

use super::AppState;

#[derive(Debug, Deserialize)]
pub struct FeedQuery {
    /// ISO8601 cursor — returns posts older than this timestamp (exclusive)
    pub cursor: Option<String>,
    /// Optional provider filter
    pub provider: Option<String>,
    /// Optional author handle filter
    pub author_handle: Option<String>,
    /// Optional full-text search query (ILIKE on text/author_name/author_handle).
    /// When present, switches the underlying query from `list_all_external_posts`
    /// to `search_all_external_posts`. Empty string is treated as None.
    pub q: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    20
}

/// Engagement metrics included with each feed post.
/// Always present (even if all values are 0) — never None.
#[derive(Debug, Serialize)]
pub struct EngagementMetrics {
    pub likes: i32,
    pub comments: i32,
    pub shares: i32,
    pub views: i32,
    pub saves: i32,
    pub quotes: i32,
    pub reposts: i32,
    pub replies: i32,
    pub reactions: Option<serde_json::Value>,
    pub upvotes: i32,
    pub downvotes: i32,
    pub upvote_ratio: Option<f32>,
    pub awards: i32,
}

#[derive(Debug, Serialize)]
pub struct FeedPost {
    pub id: Uuid,
    pub provider: String,
    pub platform_post_id: String,
    pub text: String,
    pub author_name: Option<String>,
    pub author_handle: Option<String>,
    pub author_avatar: Option<String>,
    pub created_at: String,
    pub url: Option<String>,
    pub media: serde_json::Value,
    pub metadata: serde_json::Value,
    pub imported_at: String,
    /// Engagement metrics from post_engagement table
    pub engagement: Option<EngagementMetrics>,
}

#[derive(Debug, Serialize)]
pub struct FeedResponse {
    pub posts: Vec<FeedPost>,
    /// Cursor for the next page — the created_at of the last post in this page.
    /// null if there are no more posts.
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

/// GET /api/feed?cursor=...&provider=...&q=...&limit=20
pub async fn get(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<FeedQuery>,
) -> Result<Json<FeedResponse>, AppError> {
    let limit = query.limit.clamp(1, 100);

    // Parse cursor (ISO8601) if provided
    let cursor: Option<DateTime<Utc>> = query
        .cursor
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let provider = query.provider.as_deref();
    let author_handle = query.author_handle.as_deref();
    // Trim the search query; an empty/whitespace query means "no search".
    let q: Option<&str> = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    // If a search query is present, use the ILIKE search path; otherwise
    // use the regular list path. The search path ignores author_handle
    // (searching text/author_name/author_handle is more useful).
    let posts = if let Some(q) = q {
        queries::search_all_external_posts_with_engagement(
            &state.db, auth.user_id, q, provider, cursor, limit + 1,
        )
        .await?
    } else {
        queries::list_all_external_posts_with_engagement(
            &state.db, auth.user_id, provider, author_handle, cursor, limit + 1,
        )
        .await?
    };

    // Check if there are more posts beyond this page
    let has_more = posts.len() as i64 > limit;
    let posts: Vec<_> = posts.into_iter().take(limit as usize).collect();

    // Next cursor is the created_at of the last post in this page
    let next_cursor = posts.last().map(|p| p.created_at.to_rfc3339());

    let feed_posts: Vec<FeedPost> = posts
        .into_iter()
        .map(|p| {
            // Always construct EngagementMetrics — use 0 for None values
            let engagement = EngagementMetrics {
                likes: p.engagement_likes.unwrap_or(0),
                comments: p.engagement_comments.unwrap_or(0),
                shares: p.engagement_shares.unwrap_or(0),
                views: p.engagement_views.unwrap_or(0),
                saves: p.engagement_saves.unwrap_or(0),
                quotes: p.engagement_quotes.unwrap_or(0),
                reposts: p.engagement_reposts.unwrap_or(0),
                replies: p.engagement_replies.unwrap_or(0),
                reactions: p.engagement_reactions,
                upvotes: p.engagement_upvotes.unwrap_or(0),
                downvotes: p.engagement_downvotes.unwrap_or(0),
                upvote_ratio: p.engagement_upvote_ratio,
                awards: p.engagement_awards.unwrap_or(0),
            };

            FeedPost {
                id: p.id,
                provider: p.provider,
                platform_post_id: p.platform_post_id,
                text: p.text,
                author_name: p.author_name,
                author_handle: p.author_handle,
                author_avatar: p.author_avatar,
                created_at: p.created_at.to_rfc3339(),
                url: p.url,
                media: p.media,
                metadata: p.metadata,
                imported_at: p.imported_at.to_rfc3339(),
                engagement: Some(engagement),
            }
        })
        .collect();

    Ok(Json(FeedResponse {
        posts: feed_posts,
        next_cursor,
        has_more,
    }))
}

/// POST /api/feed/import — trigger an immediate import from all connected providers
pub async fn import(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let count = feed::refresh_user_posts(
        &state.db,
        auth.user_id,
        &state.providers,
        &state.broadcast,
        state.token_key,
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "imported": count,
        "status": "ok"
    })))
}

// ── Feed Accounts Endpoint ──────────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct FeedAccount {
    pub provider: String,
    pub author_name: Option<String>,
    pub author_handle: Option<String>,
    pub author_avatar: Option<String>,
}

/// GET /api/feed/accounts — list unique author handles per provider for filtering
pub async fn accounts(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<FeedAccount>>, AppError> {
    // Return one row per unique (provider, author) combination.
    // Combines data from external_posts and integrations tables so that
    // connected accounts show up even before posts are imported.
    let rows: Vec<FeedAccount> = sqlx::query_as::<_, FeedAccount>(
        "SELECT DISTINCT provider, author_name, author_handle, author_avatar
         FROM (
           SELECT provider, author_name, author_handle, author_avatar
           FROM external_posts
           WHERE user_id = $1
           UNION ALL
           SELECT provider_identifier AS provider,
             profile_name AS author_name,
             NULL::text AS author_handle,
             profile_picture AS author_avatar
           FROM integrations
           WHERE user_id = $1 AND disabled = false
         ) AS combined
         ORDER BY provider, author_handle, author_name",
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}

/// GET /api/feed/analytics — overall engagement summary across all imported posts
#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub provider: Option<String>,
    /// Phase 2: filter by date range (days back from now). If None, returns lifetime totals.
    pub days: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsResponse {
    pub total_likes: i64,
    pub total_comments: i64,
    pub total_shares: i64,
    pub total_views: i64,
    pub total_reposts: i64,
    pub total_replies: i64,
    pub total_upvotes: i64,
    pub total_awards: i64,
    pub posts_with_engagement: i64,
}

pub async fn analytics(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<AnalyticsResponse>, AppError> {
    // Phase 2: compute the cutoff date if days is provided.
    let cutoff: Option<chrono::DateTime<chrono::Utc>> = query.days.map(|d| {
        chrono::Utc::now() - chrono::Duration::days(d as i64)
    });
    let summary = queries::get_engagement_summary(&state.db, auth.user_id, query.provider.as_deref(), cutoff).await?;
    Ok(Json(AnalyticsResponse {
        total_likes: summary.total_likes.unwrap_or(0),
        total_comments: summary.total_comments.unwrap_or(0),
        total_shares: summary.total_shares.unwrap_or(0),
        total_views: summary.total_views.unwrap_or(0),
        total_reposts: summary.total_reposts.unwrap_or(0),
        total_replies: summary.total_replies.unwrap_or(0),
        total_upvotes: summary.total_upvotes.unwrap_or(0),
        total_awards: summary.total_awards.unwrap_or(0),
        posts_with_engagement: summary.posts_with_engagement.unwrap_or(0),
    }))
}

/// GET /api/feed/{post_id}/comments — fetch comments for a feed post from the provider
pub async fn get_comments(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(post_id): Path<Uuid>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    // Look up the external post to get provider + platform_post_id
    let post = queries::get_external_post_by_id(&state.db, auth.user_id, post_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".into()))?;

    // Find the provider in the registry
    let provider = state.providers.get(&post.provider)
        .ok_or_else(|| AppError::BadRequest(format!("Unknown provider: {}", post.provider)))?;

    // Get the integration for this provider to get the access token
    // ExternalPost doesn't store integration_id, so find by provider name
    let integrations = queries::list_integrations(&state.db, auth.user_id).await?;
    let integration = integrations
        .into_iter()
        .find(|i| i.provider_identifier == post.provider)
        .ok_or_else(|| AppError::NotFound("Integration not found for provider".into()))?;

    let access_token = crate::crypto::maybe_decrypt_token(&integration.access_token, state.token_key.as_ref());

    // Fetch comments from the provider
    let comments = provider
        .get_post_comments(&access_token, &post.platform_post_id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch comments: {e}")))?;

    // Serialize comments as JSON values
    let json_comments: Vec<serde_json::Value> = comments
        .into_iter()
        .map(|c| serde_json::to_value(c).unwrap_or_default())
        .collect();

    Ok(Json(json_comments))
}

/// DELETE /api/feed/{post_id} — soft-hide an imported feed post.
/// Phase 3: changed from hard DELETE to UPDATE hidden_at = NOW() so the
/// hide persists across refresh cycles (re-import won't clear it).
pub async fn delete_post(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    auth: AuthenticatedUser,
) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("UPDATE external_posts SET hidden_at = NOW() WHERE id = $1 AND user_id = $2")
        .bind(post_id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to hide feed post: {e}")))?;

    Ok(Json(serde_json::json!({ "hidden": true })))
}

/// POST /api/feed/{post_id}/save — bookmark a feed post for later reference.
pub async fn save_post(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    auth: AuthenticatedUser,
) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("UPDATE external_posts SET saved_at = NOW() WHERE id = $1 AND user_id = $2")
        .bind(post_id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to save feed post: {e}")))?;

    Ok(Json(serde_json::json!({ "saved": true })))
}

/// DELETE /api/feed/{post_id}/save — remove bookmark from a feed post.
pub async fn unsave_post(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    auth: AuthenticatedUser,
) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("UPDATE external_posts SET saved_at = NULL WHERE id = $1 AND user_id = $2")
        .bind(post_id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to unsave feed post: {e}")))?;

    Ok(Json(serde_json::json!({ "saved": false })))
}
