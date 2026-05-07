// ─── Posts API Routes ─────────────────────────────────────────
// CRUD for social media posts with scheduling support.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::models::PostPublic;
use crate::db::queries;


use super::AppState;

// ── Request Types ───────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreatePostRequest {
    pub integration_id: Uuid,
    pub content: String,
    pub title: Option<String>,
    pub media: Option<serde_json::Value>,
    pub settings: Option<serde_json::Value>,
    pub scheduled_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePostRequest {
    pub content: Option<String>,
    pub title: Option<String>,
    pub media: Option<serde_json::Value>,
    pub settings: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct SchedulePostRequest {
    pub scheduled_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ListPostsQuery {
    pub state: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PostsListResponse {
    pub posts: Vec<PostWithIntegrationName>,
    pub total: i64,
}

#[derive(Debug, Serialize)]
pub struct PostWithIntegrationName {
    pub id: Uuid,
    pub integration_id: Uuid,
    pub integration_name: String,
    pub state: String,
    pub content: String,
    pub title: Option<String>,
    pub media: serde_json::Value,
    pub scheduled_at: Option<String>,
    pub published_at: Option<String>,
    pub platform_post_url: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct FindSlotResponse {
    pub date: String,
}

// ── Handlers ─────────────────────────────────────────────────

/// GET /api/posts
pub async fn list(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<ListPostsQuery>,
) -> Result<Json<PostsListResponse>, crate::error::AppError> {
    let limit = query.limit.unwrap_or(50).min(200);
    let offset = query.offset.unwrap_or(0);
    
    // Get total count for pagination
    let total = queries::count_posts_by_user(
        &state.db,
        auth.user_id,
        query.state.as_deref(),
    )
    .await?;

    let posts = queries::list_posts(
        &state.db,
        auth.user_id,
        query.state.as_deref(),
        limit,
        offset,
    )
    .await?;

    // Enrich with integration names
    let mut enriched = Vec::with_capacity(posts.len());
    for p in posts {
        let integration_name = if let Ok(Some(integ)) = queries::get_integration(&state.db, p.integration_id, auth.user_id).await {
            integ.provider_name
        } else {
            "Unknown".into()
        };
        enriched.push(PostWithIntegrationName {
            id: p.id,
            integration_id: p.integration_id,
            integration_name,
            state: p.state.to_string(),
            content: p.content,
            title: p.title,
            media: p.media,
            scheduled_at: p.scheduled_at.map(|d| d.to_rfc3339()),
            published_at: p.published_at.map(|d| d.to_rfc3339()),
            platform_post_url: p.platform_post_url,
            error_message: p.error_message,
            created_at: p.created_at.to_rfc3339(),
        });
    }

    Ok(Json(PostsListResponse {
        total,
        posts: enriched,
    }))
}

/// POST /api/posts
pub async fn create(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<CreatePostRequest>,
) -> Result<Json<PostPublic>, crate::error::AppError> {
    // Validate integration belongs to user
    let integ = queries::get_integration(&state.db, body.integration_id, auth.user_id)
        .await?
        .ok_or_else(|| crate::error::AppError::NotFound("Integration not found".into()))?;

    if integ.disabled {
        return Err(crate::error::AppError::BadRequest("Integration is disabled".into()));
    }

    let scheduled_at: Option<DateTime<Utc>> = match body.scheduled_at {
        Some(ref s) => {
            let dt = DateTime::parse_from_rfc3339(s)
                .map_err(|_| crate::error::AppError::BadRequest("Invalid date format, use ISO8601".into()))?;
            Some(dt.with_timezone(&Utc))
        }
        None => None,
    };

    let media = body.media.unwrap_or(serde_json::json!([]));
    let settings = body.settings.unwrap_or(serde_json::json!({}));
    let state_enum = if scheduled_at.is_some() {
        Some(crate::db::models::PostState::Queued)
    } else {
        Some(crate::db::models::PostState::Draft)
    };

    let post = queries::create_post(
        &state.db,
        auth.user_id,
        body.integration_id,
        &body.content,
        body.title.as_deref(),
        &media,
        &settings,
        scheduled_at,
        state_enum,
    )
    .await?;

    // Broadcast event
    let public = PostPublic::from(post.clone());
    state.broadcast.send("post_created", &public);

    Ok(Json(public))
}

/// GET /api/posts/:id
pub async fn get(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<PostPublic>, crate::error::AppError> {
    let post = queries::get_post(&state.db, id, auth.user_id)
        .await?
        .ok_or_else(|| crate::error::AppError::NotFound("Post not found".into()))?;
    Ok(Json(PostPublic::from(post)))
}

/// PUT /api/posts/:id
pub async fn update(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdatePostRequest>,
) -> Result<Json<PostPublic>, crate::error::AppError> {
    let content = body.content.unwrap_or_default();
    let media = body.media.unwrap_or(serde_json::json!([]));
    let settings = body.settings.unwrap_or(serde_json::json!({}));

    let post = queries::update_post_content(
        &state.db,
        id,
        auth.user_id,
        &content,
        body.title.as_deref(),
        &media,
        &settings,
    )
    .await?
    .ok_or_else(|| crate::error::AppError::NotFound("Post not found".into()))?;

    Ok(Json(PostPublic::from(post)))
}

/// POST /api/posts/:id/schedule
pub async fn schedule(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(body): Json<SchedulePostRequest>,
) -> Result<Json<PostPublic>, crate::error::AppError> {
    let scheduled_at = DateTime::parse_from_rfc3339(&body.scheduled_at)
        .map_err(|_| crate::error::AppError::BadRequest("Invalid date format, use ISO8601".into()))?
        .with_timezone(&Utc);

    let post = queries::schedule_post(&state.db, id, auth.user_id, scheduled_at)
        .await?
        .ok_or_else(|| crate::error::AppError::NotFound("Post not found".into()))?;

    let public = PostPublic::from(post.clone());
    state.broadcast.send("post_scheduled", &public);

    Ok(Json(public))
}

/// DELETE /api/posts/:id
pub async fn delete(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let deleted = queries::delete_post(&state.db, id, auth.user_id).await?;
    if !deleted {
        return Err(crate::error::AppError::NotFound("Post not found".into()));
    }
    state.broadcast.send("post_deleted", &serde_json::json!({"id": id}));
    Ok(Json(serde_json::json!({"deleted": true})))
}

/// GET /api/posts/find-slot
#[derive(Debug, Deserialize)]
pub struct FindSlotQuery {
    pub integration_id: Option<Uuid>,
}

pub async fn find_slot(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<FindSlotQuery>,
) -> Result<Json<FindSlotResponse>, crate::error::AppError> {
    let slot = queries::find_next_free_slot(&state.db, auth.user_id, query.integration_id)
        .await?
        .unwrap_or_else(Utc::now);

    Ok(Json(FindSlotResponse {
        date: slot.to_rfc3339(),
    }))
}
