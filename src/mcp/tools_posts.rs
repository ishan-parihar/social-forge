// ─── MCP Post Tools ───────────────────────────────────────────
// Tool handlers for post CRUD and scheduling.
// Thin wrappers over the shared PostService.

use chrono::{DateTime, Utc};
use rmcp::{Json, schemars::JsonSchema};
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

/// First integration row for a provider (audit §4.2 — the create-tools
/// need the ROW, not just a token, so the post can be attached to it).
pub(crate) async fn integration_for_provider(
    state: &AppState,
    user_id: Uuid,
    provider: &str,
) -> Result<crate::db::models::Integration, String> {
    crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?
        .into_iter()
        .find(|i| i.provider_identifier == provider)
        .ok_or_else(|| {
            format!("{provider} account not connected. Connect it via the onboarding page first.")
        })
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
    ).await?;

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
    ).await?;

    let mut summaries = Vec::with_capacity(posts.0.len());
    for p in &posts.0 {
        let integration_name = crate::db::queries::get_integration(
            &state.db,
            p.integration_id,
            user_id,
        )
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

    let integration_name = crate::db::queries::get_integration(
        &state.db,
        post.integration_id,
        user_id,
    )
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

    let post = PostService::schedule(
        &state.db,
        &state.broadcast,
        user_id,
        post_id,
        scheduled_at,
    ).await?;

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

    PostService::delete(
        &state.db,
        &state.broadcast,
        user_id,
        post_id,
    ).await?;

    Ok(Json(crate::mcp::McpJsonValue(serde_json::json!({"success": true, "message": "Post deleted",}))))
}

pub async fn find_slot(
    state: &AppState,
    input: &FindSlotInput,
) -> Result<Json<FindSlotOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let integration_id = input.integration_id.as_ref()
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
    ).await?;

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
    ).await?;

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
    pub settings: Option<serde_json::Value>,
    pub scheduled_at: Option<String>,
    pub first_comment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct StagedPostInfo {
    pub post_id: String,
    pub provider: String,
    pub sequence: usize,
    pub total_segments: usize,
    pub state: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct StagePostOutput {
    pub staged: Vec<StagedPostInfo>,
    pub total_posts: usize,
    pub warnings: Vec<String>,
}

pub async fn stage_post(
    state: &AppState,
    input: &StagePostInput,
) -> Result<Json<StagePostOutput>, String> {
    let user_id = resolve_first_user(state).await?;

    let integration_ids: Vec<Uuid> = input.integration_ids.iter()
        .map(|s| Uuid::parse_str(s).map_err(|_| format!("Invalid integration_id: {s}")))
        .collect::<Result<_, _>>()?;

    let scheduled_at = match &input.scheduled_at {
        Some(s) => {
            let dt = chrono::DateTime::parse_from_rfc3339(s)
                .map_err(|_| "Invalid date format, use ISO8601".to_string())?;
            Some(dt.with_timezone(&chrono::Utc))
        }
        None => None,
    };

    let request = crate::services::staging::StagingRequest {
        content: input.content.clone(),
        media: input.media.clone().unwrap_or(serde_json::json!([])),
        integration_ids,
        settings: input.settings.clone().unwrap_or(serde_json::json!({})),
        scheduled_at,
        first_comment: input.first_comment.clone(),
    };

    crate::services::staging::validate_staging_request(&request)?;

    let result = crate::services::staging::stage_post(&state.db, &state.providers, user_id, request).await?;

    let staged = result.staged.into_iter().map(|s| StagedPostInfo {
        post_id: s.post_id.to_string(),
        provider: s.provider,
        sequence: s.sequence,
        total_segments: s.total_segments,
        state: s.state,
    }).collect();

    Ok(Json(StagePostOutput {
        staged,
        total_posts: result.total_posts,
        warnings: result.warnings,
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
    ).await?;

    Ok(Json(CreateCarouselOutput {
        id: post.id.to_string(),
        state: post.state.to_string(),
        scheduled_at: post.scheduled_at.map(|d| d.to_rfc3339()),
        created_at: post.created_at.to_rfc3339(),
    }))
}
