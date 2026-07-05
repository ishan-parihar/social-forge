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
    let posts = queries::list_all_external_posts(&state.db, auth.user_id, None, None, 50)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch posts: {e}")))?;

    let integrations = queries::list_integrations(&state.db, auth.user_id).await?;

    let mut comments: Vec<CommentItem> = Vec::new();

    for post in &posts {
        if let Some(ref p) = query.platform {
            if p != "all" && !post.provider.eq_ignore_ascii_case(p) {
                continue;
            }
        }

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

        let platform_post_id = &post.platform_post_id;

        if let Ok(provider_comments) = provider.get_post_comments(&access_token, platform_post_id).await {
            for c in provider_comments {
                let author = c.author_name.unwrap_or_else(|| "Unknown".into());
                let created_at = c.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
                comments.push(CommentItem {
                    id: c.id,
                    post_id: post.id.to_string(),
                    post_content: post.text.clone(),
                    platform: post.provider.clone(),
                    author,
                    content: c.text,
                    status: "new".into(),
                    created_at,
                });
            }
        }
    }

    if let Some(ref status) = query.status {
        if status != "all" {
            comments.retain(|c| c.status == *status);
        }
    }

    Ok(Json(serde_json::json!({ "comments": comments })))
}

pub async fn resolve(
    State(_state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(_comment_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn reply(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(comment_id): Path<String>,
    Json(body): Json<ReplyRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
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
