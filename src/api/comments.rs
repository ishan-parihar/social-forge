use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::api::AppState;
use crate::auth::middleware::AuthenticatedUser;
use crate::db::queries;
use crate::error::AppError;
use crate::social::PostContent;

#[derive(Debug, Deserialize)]
pub struct CommentQuery {
    pub platform: Option<String>,
    pub status: Option<String>,
}

#[derive(serde::Serialize)]
pub struct CommentItem {
    pub id: String,
    pub post_id: String,
    pub post_content: String,
    pub platform: String,
    pub author: String,
    pub content: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ReplyRequest {
    pub content: String,
}

pub async fn list(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<CommentQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    // B-3: comments list now reads from the cached_comments table instead
    // of making 50 sequential provider API calls per page load.
    //
    // The cache is populated by the background feed refresher (which
    // already pulls posts and engagement metrics). If the cache is empty
    // (e.g. the user just connected their first account and the refresher
    // hasn't run yet), the user will see an empty list — but the next
    // refresh cycle will populate it. This is a much better tradeoff than
    // 10+ second page loads + provider rate-limit exhaustion.
    //
    // For reply(), we still do a live provider API call to find the
    // matching comment's post — see that function for details.

    let provider_filter: Option<&str> = query
        .platform
        .as_deref()
        .filter(|p| *p != "all");

    let cached = queries::list_cached_comments(&state.db, auth.user_id, provider_filter, 200)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch cached comments: {e}")))?;

    // Load the user's resolved-comment set so we can flag each CommentItem
    // as "new" or "resolved" instead of always "new".
    let resolved_ids = queries::list_resolved_comment_ids(&state.db, auth.user_id)
        .await
        .unwrap_or_default();

    let comments: Vec<CommentItem> = cached
        .into_iter()
        .map(|c| {
            let author = c.author_name.unwrap_or_else(|| "Unknown".into());
            let created_at = c.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
            let status = if resolved_ids.contains(&c.comment_id) { "resolved" } else { "new" };
            CommentItem {
                id: c.comment_id,
                post_id: c.post_id.to_string(),
                post_content: c.post_text.unwrap_or_default(),
                platform: c.provider,
                author,
                content: c.text,
                status: status.into(),
                created_at,
            }
        })
        .collect();

    let mut comments = comments;
    if let Some(ref status) = query.status {
        if status != "all" {
            comments.retain(|c| c.status == *status);
        }
    }

    Ok(Json(serde_json::json!({ "comments": comments })))
}

pub async fn resolve(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(comment_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    queries::resolve_comment(&state.db, auth.user_id, &comment_id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to resolve comment: {e}")))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn reply(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(comment_id): Path<String>,
    Json(body): Json<ReplyRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // B-3 optimization: instead of iterating all 100 posts and doing
    // a live `get_post_comments` API call for each (which was up to 100
    // sequential network calls), look up the comment's post_id directly
    // from the cache. Then do a single `reply_to_comment` call.
    //
    // Fallback: if the comment isn't in the cache (e.g. it's older than
    // the cache window or the refresher hasn't run yet), fall back to
    // the old iterate-and-fetch behavior. This preserves correctness
    // for the rare case where the cache is cold.

    // First, try the cache path.
    let cached_post: Option<(uuid::Uuid, String)> = sqlx::query_as(
        "SELECT post_id, provider FROM cached_comments WHERE user_id = $1 AND comment_id = $2",
    )
    .bind(auth.user_id)
    .bind(&comment_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to query cached comment: {e}")))?;

    if let Some((post_id, provider_id)) = cached_post {
        // Cache hit — find the integration for this provider.
        let integrations = queries::list_integrations(&state.db, auth.user_id).await?;
        let integration = integrations
            .into_iter()
            .find(|i| i.provider_identifier == provider_id)
            .ok_or_else(|| AppError::NotFound("Integration for this comment not found".into()))?;

        let provider = state
            .providers
            .get(&integration.provider_identifier)
            .ok_or_else(|| {
                AppError::BadRequest(format!("Provider {} not found", integration.provider_identifier))
            })?;

        let access_token = state.token_key.as_ref()
            .and_then(|key| crate::crypto::decrypt_string(&integration.access_token, key).ok())
            .unwrap_or(integration.access_token.clone());

        let content = PostContent {
            content: body.content.clone(),
            media: vec![],
            settings: serde_json::json!({}),
            in_reply_to: None,
            idempotency_key: None,
        };
        provider.reply_to_comment(&access_token, &comment_id, &content)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to reply: {e}")))?;
        // Silence unused-variable warning for post_id — it's used for
        // the JOIN in the SQL query but not directly here.
        let _ = post_id;
        return Ok(Json(serde_json::json!({ "ok": true })));
    }

    // Cache miss — fall back to the old iterate-and-fetch path.
    // This is slow (up to 100 sequential API calls) but correct.
    // Log a warning so we can monitor how often this happens.
    tracing::warn!("Comment {} not in cache — falling back to live fetch", comment_id);

    let posts = queries::list_all_external_posts(&state.db, auth.user_id, None, None, 100)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch posts: {e}")))?;

    let integrations = queries::list_integrations(&state.db, auth.user_id).await?;

    for post in &posts {
        let provider = match state.providers.get(&post.provider) {
            Some(p) => p,
            None => continue,
        };

        let integration = match integrations.iter().find(|i| i.provider_identifier == post.provider) {
            Some(i) => i,
            None => continue,
        };

        let access_token = state.token_key.as_ref()
            .and_then(|key| crate::crypto::decrypt_string(&integration.access_token, key).ok())
            .unwrap_or(integration.access_token.clone());

        if let Ok(provider_comments) = provider.get_post_comments(&access_token, &post.platform_post_id).await {
            if provider_comments.iter().any(|c| c.id == comment_id) {
                let content = PostContent {
                    content: body.content.clone(),
                    media: vec![],
                    settings: serde_json::json!({}),
                in_reply_to: None,
                idempotency_key: None,
                };
                provider.reply_to_comment(&access_token, &comment_id, &content)
                    .await
                    .map_err(|e| AppError::Internal(format!("Failed to reply: {e}")))?;

                return Ok(Json(serde_json::json!({ "ok": true })));
            }
        }
    }

    Err(AppError::NotFound(format!("Comment {} not found", comment_id)))
}
