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
use crate::services::posts::PostService;
use crate::api::tags::TagResponse;

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
    pub tag_ids: Option<Vec<Uuid>>,
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
    pub tags: Vec<TagResponse>,
}

#[derive(Debug, Serialize)]
pub struct PostDetailResponse {
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
    pub tags: Vec<TagResponse>,
}

#[derive(Debug, Deserialize)]
pub struct SetPostTagsRequest {
    pub tag_ids: Vec<Uuid>,
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

    // Enrich with integration names and tags
    let mut enriched = Vec::with_capacity(posts.len());
    for p in posts {
        let integration_name = if let Ok(Some(integ)) = queries::get_integration(&state.db, p.integration_id, auth.user_id).await {
            integ.provider_name
        } else {
            "Unknown".into()
        };
        let tags = queries::get_tags_for_post(&state.db, p.id, auth.user_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|row| TagResponse {
                id: row.id,
                name: row.name,
                color: row.color,
                created_at: row.created_at.to_rfc3339(),
                updated_at: row.updated_at.to_rfc3339(),
            })
            .collect();
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
            tags,
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

    // Insert post_tags if tag_ids provided
    if let Some(ref tag_ids) = body.tag_ids {
        if !tag_ids.is_empty() {
            queries::set_post_tags(&state.db, post.id, tag_ids).await?;
        }
    }

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
) -> Result<Json<PostDetailResponse>, crate::error::AppError> {
    let post = queries::get_post(&state.db, id, auth.user_id)
        .await?
        .ok_or_else(|| crate::error::AppError::NotFound("Post not found".into()))?;

    let integration_name = if let Ok(Some(integ)) = queries::get_integration(&state.db, post.integration_id, auth.user_id).await {
        integ.provider_name
    } else {
        "Unknown".into()
    };

    let tags = queries::get_tags_for_post(&state.db, post.id, auth.user_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|row| TagResponse {
            id: row.id,
            name: row.name,
            color: row.color,
            created_at: row.created_at.to_rfc3339(),
            updated_at: row.updated_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(PostDetailResponse {
        id: post.id,
        integration_id: post.integration_id,
        integration_name,
        state: post.state.to_string(),
        content: post.content,
        title: post.title,
        media: post.media,
        scheduled_at: post.scheduled_at.map(|d| d.to_rfc3339()),
        published_at: post.published_at.map(|d| d.to_rfc3339()),
        platform_post_url: post.platform_post_url,
        error_message: post.error_message,
        created_at: post.created_at.to_rfc3339(),
        tags,
    }))
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
    let deleted = PostService::delete(&state.db, &state.broadcast, auth.user_id, id)
        .await
        .map_err(crate::error::AppError::BadRequest)?;
    if !deleted {
        return Err(crate::error::AppError::NotFound("Post not found".into()));
    }
    Ok(Json(serde_json::json!({"deleted": true})))
}

/// PUT /api/posts/{id}/tags
pub async fn set_post_tags(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(body): Json<SetPostTagsRequest>,
) -> Result<Json<PostDetailResponse>, crate::error::AppError> {
    // Verify post exists and belongs to user
    let post = queries::get_post(&state.db, id, auth.user_id)
        .await?
        .ok_or_else(|| crate::error::AppError::NotFound("Post not found".into()))?;

    // Verify all tag_ids belong to the user
    for &tag_id in &body.tag_ids {
        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM tags WHERE id = $1 AND user_id = $2",
        )
        .bind(tag_id)
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;
        if exists == 0 {
            return Err(crate::error::AppError::BadRequest(
                format!("Tag {} not found or does not belong to user", tag_id),
            ));
        }
    }

    queries::set_post_tags(&state.db, id, &body.tag_ids).await?;

    // Return enriched post
    let integration_name = if let Ok(Some(integ)) = queries::get_integration(&state.db, post.integration_id, auth.user_id).await {
        integ.provider_name
    } else {
        "Unknown".into()
    };

    let tags = queries::get_tags_for_post(&state.db, post.id, auth.user_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|row| TagResponse {
            id: row.id,
            name: row.name,
            color: row.color,
            created_at: row.created_at.to_rfc3339(),
            updated_at: row.updated_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(PostDetailResponse {
        id: post.id,
        integration_id: post.integration_id,
        integration_name,
        state: post.state.to_string(),
        content: post.content,
        title: post.title,
        media: post.media,
        scheduled_at: post.scheduled_at.map(|d| d.to_rfc3339()),
        published_at: post.published_at.map(|d| d.to_rfc3339()),
        platform_post_url: post.platform_post_url,
        error_message: post.error_message,
        created_at: post.created_at.to_rfc3339(),
        tags,
    }))
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

/// POST /api/posts/{id}/publish — publish a post now or retry a failed post
#[derive(Debug, Serialize)]
pub struct PublishResponse {
    pub id: String,
    pub state: String,
    pub platform_post_url: String,
}

pub async fn publish_post(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<PublishResponse>, crate::error::AppError> {
    let platform_url = crate::services::posts::PostService::publish(
        &state.db,
        &state.providers,
        &state.broadcast,
        auth.user_id,
        id,
    )
    .await
    .map_err(crate::error::AppError::BadRequest)?;

    Ok(Json(PublishResponse {
        id: id.to_string(),
        state: "published".into(),
        platform_post_url: platform_url,
    }))
}
