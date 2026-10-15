// ─── MCP Post Tools ───────────────────────────────────────────
// Tool handlers for post CRUD and scheduling.
// Thin wrappers over the shared PostService.

use chrono::{DateTime, Utc};
use rmcp::{schemars::JsonSchema, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::services::posts::{PostService, UpdatePostInput as SvcUpdatePostInput};

// ── Shared publish-now pipeline (audit §4.2) ─────────────────

/// Best-effort MIME type from a media URL's extension. The provider
/// re-detects from bytes on download; this only feeds validation and
/// DB row shaping. Replaces per-tool hard-codes like `"image/jpeg"`
/// that silently broke video posts.
pub(crate) fn mime_from_url(url: &str) -> String {
    let path = url.split('?').next().unwrap_or(url);
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "mp4" | "m4v" => "video/mp4",
        "mov" => "video/quicktime",
        "webm" => "video/webm",
        _ => "image/jpeg",
    }
    .to_string()
}

/// Resolve ONE integration row (audit §5.4). `selector` accepts:
/// - `None` → the first connected integration for the provider (the
///   historical single-account behavior);
/// - a UUID → that exact integration id;
/// - `"provider:Name"` → the provider's integration whose
///   `internal_id` or name matches `Name` (e.g. "linkedin-page:acme",
///   "x:alt-account").
pub(crate) async fn select_integration(
    state: &AppState,
    user_id: Uuid,
    provider: &str,
    selector: Option<&str>,
) -> Result<crate::db::models::Integration, String> {
    let integrations: Vec<crate::db::models::Integration> =
        crate::db::queries::list_integrations(&state.db, user_id)
            .await
            .map_err(|e| format!("DB error: {e}"))?
            .into_iter()
            .filter(|i| i.provider_identifier == provider)
            .collect();

    let found = match selector.map(str::trim).filter(|s| !s.is_empty()) {
        None => integrations.into_iter().next(),
        Some(sel) => {
            let by_uuid = Uuid::parse_str(sel)
                .ok()
                .and_then(|id| integrations.iter().find(|i| i.id == id).cloned());
            by_uuid.or_else(|| {
                let name = sel.strip_prefix(&format!("{provider}:")).unwrap_or(sel);
                integrations.into_iter().find(|i| {
                    i.internal_id.eq_ignore_ascii_case(name)
                        || i.provider_name.eq_ignore_ascii_case(name)
                })
            })
        }
    };

    found.ok_or_else(|| match selector {
        Some(sel) => format!(
            "No {provider} integration matches '{sel}'. Connected: none — connect it first."
        ),
        None => {
            format!("{provider} account not connected. Connect it via the onboarding page first.")
        }
    })
}

/// §5.2 lifecycle parity — REST routes that had no MCP tool.

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ThreadInput {
    /// Thread parts, in order (max 25).
    pub content_parts: Vec<String>,
    /// Integration IDs to thread on (each gets the full thread).
    pub integration_ids: Vec<String>,
    /// RFC3339. Omitted → drafts; provided → queued + scheduled.
    pub scheduled_at: Option<String>,
    /// Delay in minutes between thread parts (part N due at base + (N-1)*delay).
    pub delay_minutes: Option<i64>,
}

/// Create a thread: ordered parts sharing one group_id, linked at publish
/// time by the scheduler's predecessor gate. Mirrors POST /api/posts/thread.
pub async fn create_thread(
    state: &AppState,
    input: &ThreadInput,
) -> Result<Json<crate::mcp::McpJsonValue>, String> {
    let user_id = resolve_first_user(state).await?;
    if input.content_parts.is_empty() {
        return Err("content_parts must not be empty".into());
    }
    if input.content_parts.len() > 25 {
        return Err("Maximum 25 parts per thread".into());
    }
    let integration_ids: Vec<Uuid> = input
        .integration_ids
        .iter()
        .map(|s| Uuid::parse_str(s).map_err(|_| format!("Invalid integration_id: {s}")))
        .collect::<Result<_, _>>()?;
    if integration_ids.is_empty() {
        return Err("At least one integration_id is required".into());
    }
    for id in &integration_ids {
        let integ = crate::db::queries::get_integration(&state.db, *id, user_id)
            .await
            .map_err(|e| format!("DB error: {e}"))?
            .ok_or_else(|| format!("Integration {id} not found"))?;
        if integ.disabled {
            return Err(format!("Integration {id} is disabled"));
        }
    }

    let scheduled_at = parse_rfc3339_opt(&input.scheduled_at)?;
    let state_enum = if scheduled_at.is_some() {
        crate::db::models::PostState::Queued
    } else {
        crate::db::models::PostState::Draft
    };
    let state_label = state_enum.to_string();
    let group_id = Uuid::new_v4();
    let posts = crate::db::queries::create_thread_posts(
        &state.db,
        user_id,
        &integration_ids,
        &input.content_parts,
        scheduled_at,
        Some(state_enum),
        group_id,
        input.delay_minutes,
    )
    .await
    .map_err(|e| format!("Failed to create thread: {e}"))?;

    for p in &posts {
        state
            .broadcast
            .send("post_created", &serde_json::json!({"id": p.id.to_string()}));
    }

    Ok(Json(crate::mcp::McpJsonValue(serde_json::json!({
        "group_id": group_id.to_string(),
        "post_ids": posts.iter().map(|p| p.id.to_string()).collect::<Vec<_>>(),
        "state": state_label,
    }))))
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct UnscheduleInput {
    pub post_id: String,
}

/// Move a queued/draft/error post back to draft (unschedule). Mirrors POST
/// /api/posts/{id}/unschedule.
pub async fn unschedule(
    state: &AppState,
    input: &UnscheduleInput,
) -> Result<Json<crate::mcp::McpJsonValue>, String> {
    let user_id = resolve_first_user(state).await?;
    let post_id =
        Uuid::parse_str(&input.post_id).map_err(|_| "Invalid post_id UUID".to_string())?;
    let post = crate::db::queries::unschedule_post(&state.db, post_id, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?
        .ok_or_else(|| {
            "Post not found or cannot be unscheduled (only queued/draft/error posts can)"
                .to_string()
        })?;
    state.broadcast.send(
        "post_scheduled",
        &serde_json::json!({"id": post_id.to_string(), "unscheduled": true}),
    );
    Ok(Json(crate::mcp::McpJsonValue(serde_json::to_value(&post).unwrap_or_default())))
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RescheduleInput {
    pub post_id: String,
    /// New time (RFC3339).
    pub scheduled_at: String,
    /// Apply the same time delta to every post in the thread/campaign group.
    #[serde(default)]
    pub move_group: Option<bool>,
    /// For PUBLISHED posts only: "schedule" (re-publish) or "update"
    /// (archive re-date). Required when the post is published.
    #[serde(default)]
    pub action: Option<String>,
}

/// Reschedule a post (or its whole group with move_group). Mirrors PUT
/// /api/posts/{id}/date via the shared PostService::reschedule core.
pub async fn reschedule(
    state: &AppState,
    input: &RescheduleInput,
) -> Result<Json<crate::mcp::McpJsonValue>, String> {
    let user_id = resolve_first_user(state).await?;
    let post_id =
        Uuid::parse_str(&input.post_id).map_err(|_| "Invalid post_id UUID".to_string())?;
    let new_at = chrono::DateTime::parse_from_rfc3339(&input.scheduled_at)
        .map_err(|e| format!("Invalid scheduled_at (use RFC3339): {e}"))?
        .with_timezone(&chrono::Utc);
    let value = crate::services::posts::PostService::reschedule(
        &state.db,
        &state.broadcast,
        user_id,
        post_id,
        new_at,
        input.move_group.unwrap_or(false),
        input.action.as_deref(),
    )
    .await?;
    Ok(Json(crate::mcp::McpJsonValue(value)))
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GroupInput {
    /// Campaign/thread group_id (from posts_stage or posts_thread).
    pub group_id: String,
}

/// Fetch every post in a campaign group. Mirrors GET /api/posts/group/{id}.
pub async fn get_group(
    state: &AppState,
    input: &GroupInput,
) -> Result<Json<crate::mcp::McpJsonValue>, String> {
    let user_id = resolve_first_user(state).await?;
    let group_id =
        Uuid::parse_str(&input.group_id).map_err(|_| "Invalid group_id UUID".to_string())?;
    let posts = crate::db::queries::list_posts_by_group(&state.db, user_id, group_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;
    Ok(Json(crate::mcp::McpJsonValue(serde_json::json!({
        "group_id": group_id.to_string(),
        "posts": posts,
    }))))
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ValidateInput {
    pub integration_ids: Vec<String>,
    pub content: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub media: Option<serde_json::Value>,
    #[serde(default)]
    pub settings: Option<serde_json::Value>,
}

/// Validate a post against every integration WITHOUT creating it — the
/// same rules the scheduler enforces at publish time. Mirrors POST
/// /api/posts/validate.
pub async fn validate(
    state: &AppState,
    input: &ValidateInput,
) -> Result<Json<crate::mcp::McpJsonValue>, String> {
    let user_id = resolve_first_user(state).await?;
    let integration_ids: Vec<Uuid> = input
        .integration_ids
        .iter()
        .map(|s| Uuid::parse_str(s).map_err(|_| format!("Invalid integration_id: {s}")))
        .collect::<Result<_, _>>()?;

    let mut integrations = Vec::with_capacity(integration_ids.len());
    for id in &integration_ids {
        let integ = crate::db::queries::get_integration(&state.db, *id, user_id)
            .await
            .map_err(|e| format!("DB error: {e}"))?
            .ok_or_else(|| format!("Integration {id} not found"))?;
        integrations.push(integ);
    }

    let media: Vec<crate::social::MediaAttachment> =
        serde_json::from_value(input.media.clone().unwrap_or(serde_json::json!([])))
            .unwrap_or_default();

    let body = crate::api::CreatePostRequest {
        integration_ids,
        content: input.content.clone(),
        title: input.title.clone(),
        media: input.media.clone(),
        settings: input.settings.clone(),
        scheduled_at: None,
        tag_ids: None,
        first_comment: None,
        sequence: None,
        overrides: None,
    };
    let errors = crate::api::validate_posts_for_integrations(state, &body, &integrations, &media);

    Ok(Json(crate::mcp::McpJsonValue(serde_json::json!({
        "valid": errors.is_empty(),
        "errors": errors,
    }))))
}

fn parse_rfc3339_opt(s: &Option<String>) -> Result<Option<chrono::DateTime<chrono::Utc>>, String> {
    match s {
        None => Ok(None),
        Some(s) => Ok(Some(
            chrono::DateTime::parse_from_rfc3339(s)
                .map_err(|e| format!("Invalid date (use RFC3339): {e}"))?
                .with_timezone(&chrono::Utc),
        )),
    }
}

/// Publish NOW through the ONE pipeline (audit §4.2): creates a `posts`
/// row for the integration (queued), then runs the shared engine via
/// PostService::publish. The platform `*_create_*` tools call this
/// instead of `provider.publish()` directly, so an agent-created post
/// gets what the direct path skipped: a posts row (analytics link),
/// idempotency key, retries with backoff, post_pending/finalize
/// lifecycle, first-comment, attempt audit, and events.
///
/// Returns `(post_id, platform_post_id, platform_post_url)`.
pub(crate) async fn publish_now_via_pipeline(
    state: &AppState,
    integration: &crate::db::models::Integration,
    content: &str,
    title: Option<&str>,
    media: Vec<crate::social::MediaAttachment>,
    settings: serde_json::Value,
    first_comment: Option<&str>,
) -> Result<(Uuid, String, String), String> {
    let media_json = serde_json::to_value(&media).unwrap_or(serde_json::json!([]));
    let post = crate::db::queries::create_post(
        &state.db,
        integration.user_id,
        integration.id,
        content,
        title,
        &media_json,
        &settings,
        None,
        Some(crate::db::models::PostState::Queued),
        first_comment,
        0,
    )
    .await
    .map_err(|e| format!("Failed to create post row: {e}"))?;

    let url = PostService::publish(
        &state.db,
        &state.providers,
        &state.broadcast,
        integration.user_id,
        post.id,
        state.token_key,
    )
    .await?;

    let row = crate::db::queries::get_post(&state.db, post.id, integration.user_id)
        .await
        .map_err(|e| format!("Failed to read back post: {e}"))?
        .ok_or_else(|| "Post row not found after publish".to_string())?;

    Ok((post.id, row.platform_post_id.unwrap_or_default(), url))
}

// ── Input/Output Types ──────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreatePostInput {
    pub integration_id: String,
    pub content: String,
    pub title: Option<String>,
    pub scheduled_at: Option<String>,
    pub media: Option<serde_json::Value>,
    pub settings: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreatePostOutput {
    pub id: String,
    pub state: String,
    pub scheduled_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListPostsInput {
    pub state: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PostSummary {
    pub id: String,
    pub integration_name: String,
    pub state: String,
    pub content: String,
    pub title: Option<String>,
    pub scheduled_at: Option<String>,
    pub platform_post_url: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListPostsOutput {
    pub posts: Vec<PostSummary>,
    pub total: i32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetPostInput {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetPostOutput {
    pub post: PostSummary,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SchedulePostInput {
    pub id: String,
    pub scheduled_at: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SchedulePostOutput {
    pub id: String,
    pub state: String,
    pub scheduled_at: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DeletePostInput {
    pub id: String,
    /// §8.2: also DELETE the published post on the platform (one verb for
    /// both halves). Default false: only the local row is removed. When
    /// true and the post is published, providers without a delete API
    /// (edit-restricted platforms) fail loudly instead of silently
    /// leaving a live post whose row is gone.
    #[serde(default)]
    pub delete_from_platform: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct UpdatePostInput {
    pub id: String,
    pub content: Option<String>,
    pub title: Option<String>,
    pub media: Option<serde_json::Value>,
    pub settings: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct UpdatePostOutput {
    pub id: String,
    pub state: String,
    pub content: String,
    pub title: Option<String>,
    pub scheduled_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FindSlotInput {
    pub integration_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FindSlotOutput {
    pub date: String,
}

// ── Tool Implementations ────────────────────────────────────

pub async fn create_post(
    state: &AppState,
    input: &CreatePostInput,
) -> Result<Json<CreatePostOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let integration_id = Uuid::parse_str(&input.integration_id)
        .map_err(|_| "Invalid integration_id format".to_string())?;

    let scheduled_at = match &input.scheduled_at {
        Some(s) => {
            let dt = DateTime::parse_from_rfc3339(s)
                .map_err(|_| "Invalid date format, use ISO8601".to_string())?;
            Some(dt.with_timezone(&Utc))
        }
        None => None,
    };

    let post = PostService::create(
        &state.db,
        &state.broadcast,
        crate::services::posts::CreatePostInput {
            user_id,
            integration_id,
            content: input.content.clone(),
            title: input.title.clone(),
            media_urls: input.media.clone().unwrap_or(serde_json::json!([])),
            scheduled_at,
            settings: input.settings.clone().unwrap_or(serde_json::json!({})),
            first_comment: None,
            state: None,
        },
    )
    .await?;

    Ok(Json(CreatePostOutput {
        id: post.id.to_string(),
        state: post.state.to_string(),
        scheduled_at: post.scheduled_at.map(|d| d.to_rfc3339()),
        created_at: post.created_at.to_rfc3339(),
    }))
}

pub async fn list_posts(
    state: &AppState,
    input: &ListPostsInput,
) -> Result<Json<ListPostsOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let limit = input.limit.unwrap_or(50).min(200) as i64;
    let offset = input.offset.unwrap_or(0) as i64;

    let posts = PostService::list(
        &state.db,
        user_id,
        input.state.as_deref(),
        limit,
        offset,
        true,
    )
    .await?;

    let mut summaries = Vec::with_capacity(posts.0.len());
    for p in &posts.0 {
        let integration_name =
            crate::db::queries::get_integration(&state.db, p.integration_id, user_id)
                .await
                .ok()
                .flatten()
                .map(|i| i.provider_name)
                .unwrap_or_else(|| "Unknown".into());

        summaries.push(PostSummary {
            id: p.id.to_string(),
            integration_name,
            state: p.state.to_string(),
            content: p.content.clone(),
            title: p.title.clone(),
            scheduled_at: p.scheduled_at.map(|d| d.to_rfc3339()),
            platform_post_url: p.platform_post_url.clone(),
            error_message: p.error_message.clone(),
            created_at: p.created_at.to_rfc3339(),
        });
    }

    Ok(Json(ListPostsOutput {
        total: posts.1.unwrap_or(0) as i32,
        posts: summaries,
    }))
}

pub async fn get_post(
    state: &AppState,
    input: &GetPostInput,
) -> Result<Json<GetPostOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let post_id = Uuid::parse_str(&input.id).map_err(|_| "Invalid post ID".to_string())?;

    let post = PostService::get(&state.db, user_id, post_id).await?;

    let integration_name =
        crate::db::queries::get_integration(&state.db, post.integration_id, user_id)
            .await
            .ok()
            .flatten()
            .map(|i| i.provider_name)
            .unwrap_or_else(|| "Unknown".into());

    Ok(Json(GetPostOutput {
        post: PostSummary {
            id: post.id.to_string(),
            integration_name,
            state: post.state.to_string(),
            content: post.content,
            title: post.title,
            scheduled_at: post.scheduled_at.map(|d| d.to_rfc3339()),
            platform_post_url: post.platform_post_url,
            error_message: post.error_message,
            created_at: post.created_at.to_rfc3339(),
        },
    }))
}

pub async fn schedule_post(
    state: &AppState,
    input: &SchedulePostInput,
) -> Result<Json<SchedulePostOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let post_id = Uuid::parse_str(&input.id).map_err(|_| "Invalid post ID".to_string())?;

    let scheduled_at = DateTime::parse_from_rfc3339(&input.scheduled_at)
        .map_err(|_| "Invalid date format, use ISO8601".to_string())?
        .with_timezone(&Utc);

    let post =
        PostService::schedule(&state.db, &state.broadcast, user_id, post_id, scheduled_at).await?;

    Ok(Json(SchedulePostOutput {
        id: post.id.to_string(),
        state: post.state.to_string(),
        scheduled_at: scheduled_at.to_rfc3339(),
    }))
}

pub async fn delete_post(
    state: &AppState,
    input: &DeletePostInput,
) -> Result<Json<crate::mcp::McpJsonValue>, String> {
    let user_id = resolve_first_user(state).await?;
    let post_id = Uuid::parse_str(&input.id).map_err(|_| "Invalid post ID".to_string())?;

    // §8.2: one verb for both halves — platform first (so a failure
    // leaves the row + its platform_post_id intact for retry), then the
    // local row.
    if input.delete_from_platform {
        let post = crate::db::queries::get_post_with_integration(&state.db, post_id, user_id)
            .await
            .map_err(|e| format!("DB error: {e}"))?
            .ok_or_else(|| "Post not found".to_string())?;
        if post.state != crate::db::models::PostState::Published {
            return Err(
                "delete_from_platform=true requires a PUBLISHED post — the local row is the only copy"
                    .into(),
            );
        }
        let platform_id = post
            .platform_post_id
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .to_string();
        if platform_id.is_empty() {
            return Err("post is published but carries no platform_post_id".into());
        }
        let provider = state
            .providers
            .get(&post.provider_identifier)
            .ok_or_else(|| format!("Provider '{}' not found", post.provider_identifier))?
        ;
        let token = crate::crypto::maybe_decrypt_token(
            &post.access_token,
            state.token_key.as_ref(),
        );
        provider
            .delete_published(&token, &platform_id)
            .await
            .map_err(|e| {
                format!(
                    "platform delete failed for {} {platform_id} — the local row was KEPT for retry: {e}",
                    post.provider_identifier
                )
            })?;
    }

    PostService::delete(&state.db, &state.broadcast, user_id, post_id).await?;

    Ok(Json(crate::mcp::McpJsonValue(
        serde_json::json!({
            "success": true,
            "deleted_from_platform": input.delete_from_platform,
            "message": if input.delete_from_platform {
                "Post deleted on the platform and locally"
            } else {
                "Post deleted"
            },
        }),
    )))
}

pub async fn find_slot(
    state: &AppState,
    input: &FindSlotInput,
) -> Result<Json<FindSlotOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let integration_id = input
        .integration_id
        .as_ref()
        .and_then(|s| Uuid::parse_str(s).ok());
    let slot = PostService::find_slot(&state.db, user_id, integration_id).await?;

    Ok(Json(FindSlotOutput {
        date: slot.to_rfc3339(),
    }))
}

pub async fn update_post(
    state: &AppState,
    input: &UpdatePostInput,
) -> Result<Json<UpdatePostOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let post_id = Uuid::parse_str(&input.id).map_err(|_| "Invalid post ID".to_string())?;

    let post = PostService::update(
        &state.db,
        &state.broadcast,
        user_id,
        post_id,
        SvcUpdatePostInput {
            content: input.content.clone(),
            title: input.title.clone(),
            media: input.media.clone(),
            settings: input.settings.clone(),
        },
    )
    .await?;

    Ok(Json(UpdatePostOutput {
        id: post.id.to_string(),
        state: post.state.to_string(),
        content: post.content,
        title: post.title,
        scheduled_at: post.scheduled_at.map(|d| d.to_rfc3339()),
        updated_at: post.updated_at.to_rfc3339(),
    }))
}

pub async fn publish_post(
    state: &AppState,
    input: &GetPostInput,
) -> Result<Json<SchedulePostOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let post_id = Uuid::parse_str(&input.id).map_err(|_| "Invalid post ID".to_string())?;

    let _platform_url = PostService::publish(
        &state.db,
        &state.providers,
        &state.broadcast,
        user_id,
        post_id,
        state.token_key,
    )
    .await?;

    Ok(Json(SchedulePostOutput {
        id: post_id.to_string(),
        state: "published".into(),
        scheduled_at: chrono::Utc::now().to_rfc3339(),
    }))
}

/// For MCP single-user mode: get the first user.
/// Delegates to shared auth module.
pub(crate) use super::auth::resolve_first_user;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateCarouselInput {
    pub integration_id: String,
    pub content: String,
    pub media_urls: Vec<String>,
    pub title: Option<String>,
    pub scheduled_at: Option<String>,
    pub settings: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateCarouselOutput {
    pub id: String,
    pub state: String,
    pub scheduled_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct StagePostInput {
    pub content: String,
    pub media: Option<serde_json::Value>,
    pub integration_ids: Vec<String>,
    /// Base settings shared by every integration. Provider-specific keys
    /// belong in `overrides` — a Reddit `subreddit` here would leak into
    /// every other platform (B14).
    pub settings: Option<serde_json::Value>,
    pub scheduled_at: Option<String>,
    /// "next-slot" (case-insensitive): queue each integration at ITS own
    /// next free posting slot (per-account posting queue, E4) instead of
    /// one shared time.
    pub first_comment: Option<String>,
    /// Per-provider overrides (§5.2): {"reddit": {"settings": {"subreddit":
    /// "rust", "title": "…"}}, "x": {"content": "short copy"}}.
    #[serde(default)]
    pub overrides: Option<serde_json::Value>,
    /// Report per-integration readiness and write NOTHING.
    #[serde(default)]
    pub dry_run: bool,
    /// Stage integrations whose readiness reported errors anyway.
    #[serde(default)]
    pub force: bool,
    /// Attach to an existing campaign group_id instead of minting one.
    #[serde(default)]
    pub group_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct StagedPostInfo {
    pub post_id: String,
    pub provider: String,
    pub sequence: usize,
    pub total_segments: usize,
    pub state: String,
    /// Campaign handle for every lifecycle verb (reschedule, unschedule,
    /// publish-now, delete, status).
    pub group_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct StagePostOutput {
    pub staged: Vec<StagedPostInfo>,
    pub total_posts: usize,
    pub warnings: Vec<String>,
    /// Readiness errors (rejected or force-staged integrations, B14).
    pub errors: Vec<String>,
    /// Providers rejected by readiness (empty when force staged them).
    pub rejected: Vec<String>,
    /// One handle for the whole multi-platform campaign.
    pub group_id: Option<String>,
}

pub async fn stage_post(
    state: &AppState,
    input: &StagePostInput,
) -> Result<Json<StagePostOutput>, String> {
    let user_id = resolve_first_user(state).await?;

    let integration_ids: Vec<Uuid> = input
        .integration_ids
        .iter()
        .map(|s| Uuid::parse_str(s).map_err(|_| format!("Invalid integration_id: {s}")))
        .collect::<Result<_, _>>()?;

    let next_slot = input
        .scheduled_at
        .as_deref()
        .is_some_and(|s| s.trim().eq_ignore_ascii_case("next-slot"));
    let scheduled_at = if next_slot {
        // E4: per-integration slots are resolved inside stage_post — one
        // shared time is meaningless for a multi-account queue.
        None
    } else {
        match &input.scheduled_at {
            Some(s) => {
                let dt = chrono::DateTime::parse_from_rfc3339(s)
                    .map_err(|_| "Invalid date format, use ISO8601 or 'next-slot'".to_string())?;
                Some(dt.with_timezone(&chrono::Utc))
            }
            None => None,
        }
    };

    let request = crate::services::staging::StagingRequest {
        content: input.content.clone(),
        media: input.media.clone().unwrap_or(serde_json::json!([])),
        integration_ids,
        settings: input.settings.clone().unwrap_or(serde_json::json!({})),
        scheduled_at,
        first_comment: input.first_comment.clone(),
        overrides: input.overrides.clone().unwrap_or(serde_json::Value::Null),
        dry_run: input.dry_run,
        force: input.force,
        next_slot,
        group_id: input
            .group_id
            .as_deref()
            .and_then(|g| Uuid::parse_str(g).ok()),
    };

    crate::services::staging::validate_staging_request(&request)?;

    let result =
        crate::services::staging::stage_post(&state.db, &state.providers, user_id, request).await?;

    let staged = result
        .staged
        .into_iter()
        .map(|s| StagedPostInfo {
            post_id: s.post_id.to_string(),
            provider: s.provider,
            sequence: s.sequence,
            total_segments: s.total_segments,
            state: s.state,
            group_id: Some(s.group_id.to_string()),
        })
        .collect();

    Ok(Json(StagePostOutput {
        staged,
        total_posts: result.total_posts,
        warnings: result.warnings,
        errors: result.errors,
        rejected: result.rejected,
        group_id: result.group_id.map(|g| g.to_string()),
    }))
}

pub async fn create_carousel(
    state: &AppState,
    input: &CreateCarouselInput,
) -> Result<Json<CreateCarouselOutput>, String> {
    if input.media_urls.len() < 2 {
        return Err("Carousel requires at least 2 media URLs".to_string());
    }

    let user_id = resolve_first_user(state).await?;
    let integration_id = Uuid::parse_str(&input.integration_id)
        .map_err(|_| "Invalid integration_id format".to_string())?;

    let scheduled_at = match &input.scheduled_at {
        Some(s) => {
            let dt = DateTime::parse_from_rfc3339(s)
                .map_err(|_| "Invalid date format, use ISO8601".to_string())?;
            Some(dt.with_timezone(&Utc))
        }
        None => None,
    };

    let post = PostService::create(
        &state.db,
        &state.broadcast,
        crate::services::posts::CreatePostInput {
            user_id,
            integration_id,
            content: input.content.clone(),
            title: input.title.clone(),
            media_urls: serde_json::json!(input.media_urls),
            scheduled_at,
            settings: input.settings.clone().unwrap_or(serde_json::json!({})),
            first_comment: None,
            state: None,
        },
    )
    .await?;

    Ok(Json(CreateCarouselOutput {
        id: post.id.to_string(),
        state: post.state.to_string(),
        scheduled_at: post.scheduled_at.map(|d| d.to_rfc3339()),
        created_at: post.created_at.to_rfc3339(),
    }))
}
