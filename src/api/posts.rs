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
    // v23-5: tag_ids + first_comment now persistable on update.
    // Previously the composer's edit-mode silently dropped these fields.
    pub tag_ids: Option<Vec<Uuid>>,
    pub first_comment: Option<String>,
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
    /// Phase 5: search query (ILIKE on content + title).
    pub q: Option<String>,
    /// Phase 5: filter by integration IDs (comma-separated).
    pub integration_ids: Option<String>,
    /// Phase 5: filter by tag IDs (comma-separated).
    pub tag_ids: Option<String>,
    /// Phase 5: sort order. One of: scheduled_date (default), created_date,
    /// engagement. Prefix with '-' for descending (e.g. '-scheduled_date').
    #[serde(default = "default_sort")]
    pub sort: String,
}

fn default_sort() -> String {
    "scheduled_date".into()
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
    // v25-3: kanban fields surfaced to the frontend so the kanban card UI
    // can render priority / due date / substate / sort order without a
    // second fetch. Backend writes them via update_stage (see campaigns.rs).
    pub kanban_sort_order: i32,
    pub kanban_substate: Option<String>,
    pub due_date: Option<String>,
    pub priority: String,
    // v25-3: campaign_id is already on Post (column since v22 Phase 6) but
    // wasn't being surfaced in the API response. The kanban filter relies
    // on it (v22 Phase 6 fix in the frontend), so we add it here for
    // completeness + to avoid a future round-trip.
    pub campaign_id: Option<Uuid>,
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

    // Phase 5: parse the new search/filter/sort params.
    let q = query.q.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let integration_ids: Option<Vec<Uuid>> = query.integration_ids.as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| s.split(',')
             .filter_map(|id| Uuid::parse_str(id.trim()).ok())
             .collect::<Vec<_>>())
        .filter(|v| !v.is_empty());
    let tag_ids: Option<Vec<Uuid>> = query.tag_ids.as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| s.split(',')
             .filter_map(|id| Uuid::parse_str(id.trim()).ok())
             .collect::<Vec<_>>())
        .filter(|v| !v.is_empty());

    // Use the new search query if any of the new params are present;
    // otherwise fall back to the original list_posts for backward compat.
    let use_search = q.is_some() || integration_ids.is_some() || tag_ids.is_some() || query.sort != "scheduled_date";

    let (posts, total) = if use_search {
        let posts = queries::list_posts_search(
            &state.db,
            auth.user_id,
            query.state.as_deref(),
            q,
            integration_ids.as_deref(),
            tag_ids.as_deref(),
            &query.sort,
            limit,
            offset,
        ).await?;
        let total = queries::count_posts_search(
            &state.db,
            auth.user_id,
            query.state.as_deref(),
            q,
            integration_ids.as_deref(),
            tag_ids.as_deref(),
        ).await?;
        (posts, total)
    } else {
        let total = queries::count_posts_by_user(
            &state.db,
            auth.user_id,
            query.state.as_deref(),
        ).await?;
        let posts = queries::list_posts(
            &state.db,
            auth.user_id,
            query.state.as_deref(),
            limit,
            offset,
        ).await?;
        (posts, total)
    };

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
             kanban_sort_order: p.kanban_sort_order,
             kanban_substate: p.kanban_substate.clone(),
             due_date: p.due_date.map(|d| d.to_rfc3339()),
             priority: p.priority.clone(),
             campaign_id: p.campaign_id,
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

    // ── Per-provider content validation at create-time ────────
    // Checks each integration's provider-specific limits (char count,
    // media count, media type) BEFORE creating posts. Returns structured
    // errors so the frontend can highlight the failing integration.
    let media_val: Vec<crate::social::MediaAttachment> =
        serde_json::from_value(body.media.clone().unwrap_or(serde_json::json!([])))
            .unwrap_or_default();
    let validation_errors = validate_posts_for_integrations(
        &state,
        &body,
        &validated_integrations,
        &media_val,
    );
    if !validation_errors.is_empty() {
        return Err(crate::error::AppError::BadRequest(
            serde_json::to_string(&serde_json::json!({
                "validation_errors": validation_errors,
            }))
            .unwrap_or_else(|_| "Content validation failed".into()),
        ));
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

/// GET /api/posts/group/:group_id — fetch all posts sharing a group_id.
///
/// Used by the composer's edit-mode to load all sibling posts in a thread
/// (main post + first-comment + thread parts). Returns an empty array if
/// the group_id has no posts (or if the caller doesn't own them).
///
/// Excludes soft-deleted posts.
pub async fn get_group(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(group_id): Path<Uuid>,
) -> Result<Json<Vec<PostPublic>>, crate::error::AppError> {
    let posts = queries::list_posts_by_group(&state.db, auth.user_id, group_id).await?;
    let publics: Vec<PostPublic> = posts.into_iter().map(PostPublic::from).collect();
    Ok(Json(publics))
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
    let first_comment = body.first_comment.as_deref();

    // v23-5: use update_post_full so first_comment is persisted (was
    // silently dropped by update_post_content).
    let post = queries::update_post_full(
        &state.db,
        id,
        auth.user_id,
        &content,
        body.title.as_deref(),
        &media,
        &settings,
        first_comment,
    )
    .await?
    .ok_or_else(|| crate::error::AppError::NotFound("Post not found".into()))?;

    // v23-5: update tags if tag_ids was provided in the request.
    if let Some(tag_ids) = body.tag_ids.as_ref() {
        queries::set_post_tags(&state.db, id, tag_ids).await?;
    }

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

/// POST /api/posts/:id/unschedule — v24-1
///
/// Transitions a post back to draft state by clearing scheduled_at and
/// setting state = 'draft'. Only works on posts in 'queued', 'draft',
/// or 'error' state (not 'published' or 'publishing').
///
/// This closes the ComposerModal TODO at line 551 which used the
/// "100 years in the future" hack to effectively unschedule. Now the
/// composer's saveAsDraft edit-mode flow calls this endpoint to
/// properly transition the post back to draft.
pub async fn unschedule(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<PostPublic>, crate::error::AppError> {
    let post = queries::unschedule_post(&state.db, id, auth.user_id)
        .await?
        .ok_or_else(|| crate::error::AppError::NotFound("Post not found or cannot be unscheduled (only queued/draft/error posts can be unscheduled)".into()))?;

    // Broadcast the state change so the calendar/kanban update in real-time.
    state.broadcast.send(
        "post_scheduled",
        &serde_json::json!({"id": id.to_string(), "unscheduled": true}),
    );

    Ok(Json(PostPublic::from(post)))
}

/// PUT /api/posts/:id/date — reschedule a post by dragging it on the calendar.
///
/// Accepts `{ "scheduled_at": "<RFC3339>", "move_group": bool, "action": "schedule" | "update" }`.
///
/// ## The `action` parameter (Phase v21 — postiz-inspired)
///
/// For posts in `queued` or `draft` state, the `action` parameter is
/// ignored — the post is simply rescheduled (state stays `queued`/`draft`).
///
/// For posts in `published` state, `action` determines what happens:
///   - `action: "schedule"` → reset state to `queued`, clear
///     `platform_post_id` / `platform_post_url` / `published_at`,
///     set `scheduled_at` to the new time. The scheduler will re-publish
///     the post at the new time (creating a NEW post on the platform).
///     This is the "republish" flow.
///   - `action: "update"` → just change `scheduled_at` but leave the
///     state, `platform_post_id`, `platform_post_url`, and `published_at`
///     alone. Useful for archival re-dating without triggering a re-publish.
///
/// If `action` is omitted (None) and the post is published, the request
/// is rejected with HTTP 400 (preserving the pre-v21 behavior so existing
/// callers that don't send `action` don't accidentally re-publish posts).
///
/// If the post is part of a thread (has `group_id`), the caller can pass
/// `move_group: true` to reschedule all posts in the same group by the
/// same delta. The `action` parameter applies to every post in the group.
#[derive(Debug, Deserialize)]
pub struct RescheduleRequest {
    pub scheduled_at: String,
    /// If true and the post is part of a thread, reschedule all posts
    /// in the same group by the same time delta. Default: false.
    pub move_group: Option<bool>,
    /// Phase v21: disambiguate published-post reschedules.
    /// - `schedule` → reset state to queued + clear release fields (re-publish).
    /// - `update`   → change scheduled_at only (archive re-date).
    /// Required for published posts; ignored for queued/draft posts.
    pub action: Option<String>,
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

    // Phase v21: published-post handling. The `action` param disambiguates
    // between "re-publish at new time" (schedule) and "just re-date for
    // archival" (update). Without `action`, we preserve the pre-v21
    // behavior of rejecting published-post reschedules.
    let action = body.action.as_deref().unwrap_or("");
    if post.state == crate::db::models::PostState::Published {
        if action != "schedule" && action != "update" {
            return Err(crate::error::AppError::BadRequest(
                "Cannot reschedule a published post without specifying action. \
                 Use action: 'schedule' to re-publish, or action: 'update' to \
                 just re-date for archival.".into(),
            ));
        }
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

        // Apply the same delta to all posts in the group.
        // For published posts in the group, honor the same `action`.
        let mut updated_count = 0usize;
        for group_post in &group_posts {
            if let Some(ref old_at) = group_post.scheduled_at {
                let new_at = *old_at + delta;
                if group_post.state == crate::db::models::PostState::Published
                    && action == "schedule"
                {
                    // Re-publish: reset state + clear release fields.
                    let _ = queries::reset_post_for_republish(
                        &state.db, group_post.id, auth.user_id, new_at,
                    ).await;
                } else if group_post.state == crate::db::models::PostState::Published
                    && action == "update"
                {
                    // Archive re-date: change scheduled_at only.
                    let _ = queries::update_post_date_only(
                        &state.db, group_post.id, auth.user_id, new_at,
                    ).await;
                } else {
                    // Normal reschedule for queued/draft posts.
                    let _ = queries::schedule_post(
                        &state.db, group_post.id, auth.user_id, new_at,
                    ).await;
                }
                updated_count += 1;
            }
        }

        return Ok(Json(serde_json::json!({
            "rescheduled": true,
            "group_id": group_id,
            "count": updated_count,
            "action": action,
            "new_scheduled_at": new_scheduled_at.to_rfc3339(),
        })));
    }

    // Single post reschedule
    if post.state == crate::db::models::PostState::Published && action == "schedule" {
        // Re-publish: reset state to queued, clear release fields.
        let updated = queries::reset_post_for_republish(
            &state.db, id, auth.user_id, new_scheduled_at,
        )
        .await
        .map_err(|e| crate::error::AppError::Internal(format!("DB error: {e}")))?
        .ok_or_else(|| crate::error::AppError::NotFound("Post not found after reschedule".into()))?;

        let public = PostPublic::from(updated);
        state.broadcast.send("post_scheduled", &public);

        return Ok(Json(serde_json::json!({
            "rescheduled": true,
            "action": "schedule",
            "post": public,
        })));
    } else if post.state == crate::db::models::PostState::Published && action == "update" {
        // Archive re-date: change scheduled_at only, leave state + release fields.
        let updated = queries::update_post_date_only(
            &state.db, id, auth.user_id, new_scheduled_at,
        )
        .await
        .map_err(|e| crate::error::AppError::Internal(format!("DB error: {e}")))?
        .ok_or_else(|| crate::error::AppError::NotFound("Post not found after reschedule".into()))?;

        let public = PostPublic::from(updated);
        state.broadcast.send("post_scheduled", &public);

        return Ok(Json(serde_json::json!({
            "rescheduled": true,
            "action": "update",
            "post": public,
        })));
    }

    // Normal reschedule (queued or draft post)
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

// ── Per-provider validation ──────────────────────────────────

/// A single validation error for a specific integration.
#[derive(Debug, serde::Serialize)]
pub struct ValidationError {
    pub integration_id: String,
    pub provider: String,
    pub provider_name: String,
    pub kind: String, // "too_long", "empty", "media_count", "media_type"
    pub message: String,
    pub max_length: Option<usize>,
    pub actual_length: Option<usize>,
}

/// Validate post content against each selected integration's provider limits.
/// Returns a list of validation errors (empty if all valid).
fn validate_posts_for_integrations(
    state: &AppState,
    body: &CreatePostRequest,
    integrations: &[crate::db::models::Integration],
    media: &[crate::social::MediaAttachment],
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let settings = body.settings.clone().unwrap_or(serde_json::json!({}));

    for integ in integrations {
        let provider = match state.providers.get(&integ.provider_identifier) {
            Some(p) => p,
            None => continue, // Skip unregistered providers
        };

        // Get per-integration content (from overrides if present, else global)
        let content = body
            .overrides
            .as_ref()
            .and_then(|o| o.get(&integ.id.to_string()))
            .and_then(|o| o.content.as_deref())
            .unwrap_or(&body.content);

        // Strip HTML for length check (same as PostService::sanitize_content)
        let clean: String = content
            .chars()
            .fold((false, String::new()), |(in_tag, mut acc), ch| {
                match ch {
                    '<' => (true, acc),
                    '>' => (false, acc),
                    _ if !in_tag => {
                        acc.push(ch);
                        (false, acc)
                    }
                    _ => (in_tag, acc),
                }
            })
            .1;
        let clean = clean.trim();
        let max_len = provider.max_content_length();

        // Check empty content
        if clean.is_empty() {
            errors.push(ValidationError {
                integration_id: integ.id.to_string(),
                provider: integ.provider_identifier.clone(),
                provider_name: integ.provider_name.clone(),
                kind: "empty".into(),
                message: format!("Content is empty for {}", integ.provider_name),
                max_length: None,
                actual_length: None,
            });
            continue; // No point checking other constraints if empty
        }

        // Check content too long
        if clean.len() > max_len {
            errors.push(ValidationError {
                integration_id: integ.id.to_string(),
                provider: integ.provider_identifier.clone(),
                provider_name: integ.provider_name.clone(),
                kind: "too_long".into(),
                message: format!(
                    "Content is {} chars, max {} for {}",
                    clean.len(),
                    max_len,
                    integ.provider_name
                ),
                max_length: Some(max_len),
                actual_length: Some(clean.len()),
            });
        }

        // Check media limits
        let post_content = crate::social::PostContent {
            content: clean.to_string(),
            media: media.to_vec(),
            settings: settings.clone(),
            in_reply_to: None,
            idempotency_key: None,
            delay_minutes: None
        };

        if let Err(e) = crate::social::validate_media_limits(&integ.provider_identifier, &post_content) {
            errors.push(ValidationError {
                integration_id: integ.id.to_string(),
                provider: integ.provider_identifier.clone(),
                provider_name: integ.provider_name.clone(),
                kind: "media_count".into(),
                message: e,
                max_length: None,
                actual_length: None,
            });
        }

        // Check provider-specific media validation
        if let Err(e) = provider.validate_media(&post_content) {
            errors.push(ValidationError {
                integration_id: integ.id.to_string(),
                provider: integ.provider_identifier.clone(),
                provider_name: integ.provider_name.clone(),
                kind: "media_type".into(),
                message: e,
                max_length: None,
                actual_length: None,
            });
        }
    }

    errors
}

/// POST /api/posts/validate — validate post content against provider limits
/// without creating the post. Used by the composer for live validation.
pub async fn validate(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<CreatePostRequest>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    // Validate integration ownership
    let mut integrations = Vec::with_capacity(body.integration_ids.len());
    for &id in &body.integration_ids {
        let integ = queries::get_integration(&state.db, id, auth.user_id)
            .await?
            .ok_or_else(|| crate::error::AppError::NotFound("Integration not found".into()))?;
        integrations.push(integ);
    }

    let media: Vec<crate::social::MediaAttachment> =
        serde_json::from_value(body.media.clone().unwrap_or(serde_json::json!([])))
            .unwrap_or_default();

    let errors = validate_posts_for_integrations(&state, &body, &integrations, &media);

    Ok(Json(serde_json::json!({
        "valid": errors.is_empty(),
        "errors": errors,
    })))
}
