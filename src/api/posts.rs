// ─── Posts API Routes ─────────────────────────────────────────
// CRUD for social media posts with scheduling support.

use std::collections::HashMap;

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
pub struct PostOverride {
    pub content: Option<String>,
    pub settings: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePostRequest {
    pub integration_ids: Vec<Uuid>,
    pub content: String,
    pub title: Option<String>,
    pub media: Option<serde_json::Value>,
    pub settings: Option<serde_json::Value>,
    pub scheduled_at: Option<String>,
    pub tag_ids: Option<Vec<Uuid>>,
    pub first_comment: Option<String>,
    pub sequence: Option<i32>,
    pub overrides: Option<HashMap<String, PostOverride>>,
}

#[derive(Debug, Serialize)]
pub struct CreatePostsResponse {
    pub posts: Vec<PostPublic>,
    pub group_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CreateThreadRequest {
    pub content_parts: Vec<String>,
    pub integration_ids: Vec<Uuid>,
    pub scheduled_at: Option<String>,
    /// Delay (in minutes) between each thread part. If set, each part's
    /// scheduled_at is offset by (sequence - 1) * delay_minutes.
    /// e.g. delay_minutes=5 with 3 parts scheduled at 09:00 →
    /// part 1 at 09:00, part 2 at 09:05, part 3 at 09:10.
    /// If omitted, all parts share the same scheduled_at.
    pub delay_minutes: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct CreateThreadResponse {
    pub posts: Vec<PostPublic>,
    pub group_id: Uuid,
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
    pub repeat_interval_days: Option<i32>,
    pub repeat_end_date: Option<String>,
    pub group_id: Option<Uuid>,
    pub first_comment: Option<String>,
    pub sequence: i32,
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
    pub repeat_interval_days: Option<i32>,
    pub repeat_end_date: Option<String>,
    pub group_id: Option<Uuid>,
    pub first_comment: Option<String>,
    pub sequence: i32,
}

#[derive(Debug, Deserialize)]
pub struct SetPostTagsRequest {
    pub tag_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct FindSlotResponse {
    pub date: String,
}

// ── Helpers ──────────────────────────────────────────────────

/// Fetch and convert tags for a post. Logs DB errors instead of silently swallowing them.
async fn enrich_post_tags(db: &crate::db::PgPool, post_id: Uuid, user_id: Uuid) -> Vec<TagResponse> {
    match queries::get_tags_for_post(db, post_id, user_id).await {
        Ok(rows) => rows
            .into_iter()
            .map(|row| TagResponse {
                id: row.id,
                name: row.name,
                color: row.color,
                created_at: row.created_at.to_rfc3339(),
                updated_at: row.updated_at.to_rfc3339(),
            })
            .collect(),
        Err(e) => {
            tracing::warn!("Failed to fetch tags for post {post_id}: {e}");
            Vec::new()
        }
    }
}

/// Verify that all tag_ids belong to the given user.
async fn verify_tag_ownership(
    db: &crate::db::PgPool,
    tag_ids: &[Uuid],
    user_id: Uuid,
) -> Result<(), crate::error::AppError> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM tags WHERE id = ANY($1) AND user_id = $2",
    )
    .bind(tag_ids)
    .bind(user_id)
    .fetch_one(db)
    .await?;
    if (count as usize) != tag_ids.len() {
        return Err(crate::error::AppError::BadRequest(
            "One or more tags not found or do not belong to the user".into(),
        ));
    }
    Ok(())
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

    let mut enriched = Vec::with_capacity(posts.len());
    for p in posts {
        let integration_name = if let Ok(Some(integ)) = queries::get_integration(&state.db, p.integration_id, auth.user_id).await {
            integ.provider_name
        } else {
            "Unknown".into()
        };
        let tags = enrich_post_tags(&state.db, p.id, auth.user_id).await;
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
             repeat_interval_days: p.repeat_interval_days,
             repeat_end_date: p.repeat_end_date.map(|d| d.to_rfc3339()),
             group_id: p.group_id,
             first_comment: p.first_comment.clone(),
             sequence: p.sequence,
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
) -> Result<Json<CreatePostsResponse>, crate::error::AppError> {
    // Validate ALL integration_ids belong to user and collect for enrichment
    let mut validated_integrations = Vec::with_capacity(body.integration_ids.len());
    for &id in &body.integration_ids {
        let integ = queries::get_integration(&state.db, id, auth.user_id)
            .await?
            .ok_or_else(|| crate::error::AppError::NotFound("Integration not found".into()))?;

        if integ.disabled {
            return Err(crate::error::AppError::BadRequest("Integration is disabled".into()));
        }
        validated_integrations.push(integ);
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

    // Verify tag ownership before creating the post (fail fast)
    if let Some(ref tag_ids) = body.tag_ids {
        if !tag_ids.is_empty() {
            verify_tag_ownership(&state.db, tag_ids, auth.user_id).await?;
        }
    }

    let posts = if body.overrides.is_some() {
        let overrides = body.overrides.as_ref().unwrap();
        let mut posts = Vec::with_capacity(body.integration_ids.len());
        for &id in &body.integration_ids {
            let post_content = overrides
                .get(&id.to_string())
                .and_then(|o| o.content.as_deref())
                .unwrap_or(&body.content);
            let post_settings = overrides
                .get(&id.to_string())
                .and_then(|o| o.settings.clone())
                .unwrap_or_else(|| settings.clone());
            let post = queries::create_post(
                &state.db,
                auth.user_id,
                id,
                post_content,
                body.title.as_deref(),
                &media,
                &post_settings,
                scheduled_at,
                state_enum.clone(),
                body.first_comment.as_deref(),
                body.sequence.unwrap_or(0),
            )
            .await?;
            posts.push(post);
        }
        posts
    } else {
        queries::create_posts_for_integrations(
            &state.db,
            auth.user_id,
            &body.integration_ids,
            &body.content,
            body.title.as_deref(),
            &media,
            &settings,
            scheduled_at,
            state_enum,
            body.first_comment.as_deref(),
            body.sequence.unwrap_or(0),
        )
        .await?
    };

    // Insert post_tags for each post if tag_ids provided
    if let Some(ref tag_ids) = body.tag_ids {
        if !tag_ids.is_empty() {
            for post in &posts {
                queries::set_post_tags(&state.db, post.id, tag_ids).await?;
            }
        }
    }

    // Enrich with integration names, broadcast, and return
    let publics: Vec<PostPublic> = posts
        .into_iter()
        .map(|p| {
            let integration_name = validated_integrations
                .iter()
                .find(|i| i.id == p.integration_id)
                .map(|i| i.provider_name.clone())
                .unwrap_or_default();
            let mut public = PostPublic::from(p);
            public.integration_name = integration_name;
            state.broadcast.send("post_created", &public);
            public
        })
        .collect();

    Ok(Json(CreatePostsResponse { posts: publics, group_id: None }))
}

/// POST /api/posts/thread — create a thread of posts (X/Twitter threads)
pub async fn create_thread(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<CreateThreadRequest>,
) -> Result<Json<CreateThreadResponse>, crate::error::AppError> {
    if body.content_parts.is_empty() {
        return Err(crate::error::AppError::BadRequest("content_parts must not be empty".into()));
    }
    if body.content_parts.len() > 25 {
        return Err(crate::error::AppError::BadRequest("Maximum 25 tweets per thread".into()));
    }
    if body.integration_ids.is_empty() {
        return Err(crate::error::AppError::BadRequest("At least one integration is required".into()));
    }

    // Validate ALL integration_ids belong to user
    let mut validated_integrations = Vec::with_capacity(body.integration_ids.len());
    for &id in &body.integration_ids {
        let integ = queries::get_integration(&state.db, id, auth.user_id)
            .await?
            .ok_or_else(|| crate::error::AppError::NotFound("Integration not found".into()))?;

        if integ.disabled {
            return Err(crate::error::AppError::BadRequest("Integration is disabled".into()));
        }
        validated_integrations.push(integ);
    }

    let scheduled_at: Option<DateTime<Utc>> = match body.scheduled_at {
        Some(ref s) => {
            let dt = DateTime::parse_from_rfc3339(s)
                .map_err(|_| crate::error::AppError::BadRequest("Invalid date format, use ISO8601".into()))?;
            Some(dt.with_timezone(&Utc))
        }
        None => None,
    };

    let state_enum = if scheduled_at.is_some() {
        Some(crate::db::models::PostState::Queued)
    } else {
        Some(crate::db::models::PostState::Draft)
    };

    let group_id = Uuid::new_v4();

    let posts = queries::create_thread_posts(
        &state.db,
        auth.user_id,
        &body.integration_ids,
        &body.content_parts,
        scheduled_at,
        state_enum,
        group_id,
        body.delay_minutes,
    )
    .await?;

    // Enrich with integration names and broadcast
    let publics: Vec<PostPublic> = posts
        .into_iter()
        .map(|p| {
            let integration_name = validated_integrations
                .iter()
                .find(|i| i.id == p.integration_id)
                .map(|i| i.provider_name.clone())
                .unwrap_or_default();
            let mut public = PostPublic::from(p);
            public.integration_name = integration_name;
            state.broadcast.send("post_created", &public);
            public
        })
        .collect();

    Ok(Json(CreateThreadResponse { posts: publics, group_id }))
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

    let tags = enrich_post_tags(&state.db, post.id, auth.user_id).await;

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
         repeat_interval_days: post.repeat_interval_days,
         repeat_end_date: post.repeat_end_date.map(|d| d.to_rfc3339()),
         group_id: post.group_id,
         first_comment: post.first_comment.clone(),
         sequence: post.sequence,
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

/// PUT /api/posts/:id/date — reschedule a post by dragging it on the calendar.
///
/// Accepts `{ "scheduled_at": "<RFC3339>" }` and updates the post's
/// `scheduled_at` field. Only works for posts in `queued` or `draft`
/// state — published posts can't be rescheduled (they're already live).
///
/// If the post is part of a thread (has `group_id`), the caller can
/// pass `move_group: true` to reschedule all posts in the same group
/// by the same delta. This is useful for dragging a thread to a new
/// time slot.
#[derive(Debug, Deserialize)]
pub struct RescheduleRequest {
    pub scheduled_at: String,
    /// If true and the post is part of a thread, reschedule all posts
    /// in the same group by the same time delta. Default: false.
    pub move_group: Option<bool>,
}

pub async fn reschedule(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(body): Json<RescheduleRequest>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let new_scheduled_at = DateTime::parse_from_rfc3339(&body.scheduled_at)
        .map_err(|_| crate::error::AppError::BadRequest("Invalid date format, use RFC3339/ISO8601".into()))?
        .with_timezone(&Utc);

    // Fetch the post to verify ownership and get current scheduled_at + group_id
    let post = queries::get_post(&state.db, id, auth.user_id)
        .await
        .map_err(|e| crate::error::AppError::Internal(format!("DB error: {e}")))?
        .ok_or_else(|| crate::error::AppError::NotFound("Post not found".into()))?;

    // Only allow rescheduling queued or draft posts
    if post.state == crate::db::models::PostState::Published {
        return Err(crate::error::AppError::BadRequest(
            "Cannot reschedule a published post. It's already live.".into(),
        ));
    }

    // If move_group is true and post has a group_id, reschedule all posts in the group
    let move_group = body.move_group.unwrap_or(false);
    if move_group && post.group_id.is_some() {
        let group_id = post.group_id.as_ref().unwrap();
        let group_posts = queries::get_posts_by_group_id(&state.db, auth.user_id, *group_id)
            .await
            .map_err(|e| crate::error::AppError::Internal(format!("DB error: {e}")))?;

        // Calculate the delta from the dragged post's current scheduled_at
        let old_scheduled_at = post.scheduled_at.unwrap_or_else(Utc::now);
        let delta = new_scheduled_at - old_scheduled_at;

        // Apply the same delta to all posts in the group
        for group_post in &group_posts {
            if let Some(ref old_at) = group_post.scheduled_at {
                let new_at = *old_at + delta;
                let _ = queries::schedule_post(
                    &state.db,
                    group_post.id,
                    auth.user_id,
                    new_at,
                )
                .await;
            }
        }

        return Ok(Json(serde_json::json!({
            "rescheduled": true,
            "group_id": group_id,
            "count": group_posts.len(),
            "new_scheduled_at": new_scheduled_at.to_rfc3339(),
        })));
    }

    // Single post reschedule
    let updated = queries::schedule_post(&state.db, id, auth.user_id, new_scheduled_at)
        .await
        .map_err(|e| crate::error::AppError::Internal(format!("DB error: {e}")))?
        .ok_or_else(|| crate::error::AppError::NotFound("Post not found after reschedule".into()))?;

    let public = PostPublic::from(updated);
    state.broadcast.send("post_scheduled", &public);

    Ok(Json(serde_json::json!({
        "rescheduled": true,
        "post": public,
    })))
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
    let post = queries::get_post(&state.db, id, auth.user_id)
        .await?
        .ok_or_else(|| crate::error::AppError::NotFound("Post not found".into()))?;

    if !body.tag_ids.is_empty() {
        verify_tag_ownership(&state.db, &body.tag_ids, auth.user_id).await?;
    }

    queries::set_post_tags(&state.db, id, &body.tag_ids).await?;

    let integration_name = if let Ok(Some(integ)) = queries::get_integration(&state.db, post.integration_id, auth.user_id).await {
        integ.provider_name
    } else {
        "Unknown".into()
    };

    let tags = enrich_post_tags(&state.db, post.id, auth.user_id).await;

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
         repeat_interval_days: post.repeat_interval_days,
         repeat_end_date: post.repeat_end_date.map(|d| d.to_rfc3339()),
         group_id: post.group_id,
         first_comment: post.first_comment.clone(),
         sequence: post.sequence,
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

/// POST /api/posts/{id}/repeat — set up recurring posts
#[derive(Debug, Deserialize)]
pub struct RepeatPostRequest {
    pub interval_days: i32,
    pub end_date: String,
}

#[derive(Debug, Serialize)]
pub struct RepeatPostResponse {
    pub group_id: Uuid,
    pub count: usize,
    pub post_ids: Vec<Uuid>,
    pub scheduled_dates: Vec<String>,
}

pub async fn repeat_post(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(body): Json<RepeatPostRequest>,
) -> Result<Json<RepeatPostResponse>, crate::error::AppError> {
    // 1. Validate the post exists and belongs to user
    let original = queries::get_post(&state.db, id, auth.user_id)
        .await?
        .ok_or_else(|| crate::error::AppError::NotFound("Post not found".into()))?;

    // 1a. Guard against double-recurring
    if original.repeat_interval_days.is_some() {
        return Err(crate::error::AppError::BadRequest(
            "Post is already part of a recurring series".into(),
        ));
    }

    // 1b. State guard — only draft or queued posts can be repeated
    match original.state {
        crate::db::models::PostState::Draft | crate::db::models::PostState::Queued => {}
        _ => {
            return Err(crate::error::AppError::BadRequest(
                "Can only set recurring for draft or queued posts".into(),
            ));
        }
    }

    let original_scheduled = original.scheduled_at
        .ok_or_else(|| crate::error::AppError::BadRequest("Post must have a scheduled_at time".into()))?;

    // 2. Validate parameters
    if body.interval_days <= 0 {
        return Err(crate::error::AppError::BadRequest("interval_days must be positive".into()));
    }

    let end_date = DateTime::parse_from_rfc3339(&body.end_date)
        .map_err(|_| crate::error::AppError::BadRequest("Invalid end_date format, use ISO8601".into()))?
        .with_timezone(&Utc);

    if end_date <= original_scheduled {
        return Err(crate::error::AppError::BadRequest("end_date must be after the original scheduled_at".into()));
    }

    // 3. Upper bound check — guard against runaway copy loops
    const MAX_COPIES: i32 = 100;
    let max_possible = ((end_date - original_scheduled).num_days() / body.interval_days as i64) + 1;
    if max_possible > MAX_COPIES as i64 {
        return Err(crate::error::AppError::BadRequest("Too many copies".into()));
    }

    // 4. Generate a group_id for this recurring series
    let group_id = Uuid::new_v4();

    // 5. Update original + create copies in a single transaction
    let (post_ids, scheduled_dates) = queries::set_post_recurring_with_copies(
        &state.db,
        id,
        auth.user_id,
        body.interval_days,
        &end_date,
        group_id,
        &original_scheduled,
    )
    .await?;

    Ok(Json(RepeatPostResponse {
        group_id,
        count: post_ids.len(),
        post_ids,
        scheduled_dates,
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
        state.token_key,
    )
    .await
    .map_err(crate::error::AppError::BadRequest)?;

    Ok(Json(PublishResponse {
        id: id.to_string(),
        state: "published".into(),
        platform_post_url: platform_url,
    }))
}
