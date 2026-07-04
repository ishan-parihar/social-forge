// ─── MCP Post Tools ───────────────────────────────────────────
// Tool handlers for post CRUD and scheduling.
// Thin wrappers over the shared PostService.

use chrono::{DateTime, Utc};
use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::services::posts::{PostService, UpdatePostInput as SvcUpdatePostInput};

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

    let result = crate::services::staging::stage_post(&state.db, user_id, request).await?;

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
