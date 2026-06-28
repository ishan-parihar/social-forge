// ─── Staging Service ──────────────────────────────────────────
// Orchestrates multi-platform post staging with content splitting.
// Takes a single post intent and creates appropriate posts per platform.

use uuid::Uuid;
use sqlx::PgPool;
use crate::db::queries;
use crate::services::content_splitter;

#[derive(Debug)]
pub struct StagingRequest {
    pub content: String,
    pub media: serde_json::Value,
    pub integration_ids: Vec<Uuid>,
    pub settings: serde_json::Value,
    pub scheduled_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug)]
pub struct StagedPost {
    pub post_id: Uuid,
    pub provider: String,
    pub sequence: usize,
    pub total_segments: usize,
    pub state: String,
}

#[derive(Debug)]
pub struct StagingResult {
    pub staged: Vec<StagedPost>,
    pub total_posts: usize,
    pub warnings: Vec<String>,
}

pub async fn stage_post(
    pool: &PgPool,
    user_id: Uuid,
    request: StagingRequest,
) -> Result<StagingResult, String> {
    let mut staged = Vec::new();
    let mut warnings = Vec::new();

    for integration_id in &request.integration_ids {
        let integration = queries::get_integration(pool, *integration_id, user_id)
            .await
            .map_err(|e| format!("Integration not found: {e}"))?
            .ok_or_else(|| format!("Integration {} not found", integration_id))?;

        let provider = integration.provider_identifier.clone();
        let segments = content_splitter::split_content(&request.content, &provider, 4);

        if segments.len() > 1 {
            warnings.push(format!(
                "Content split into {} posts for {} (limit: {} chars)",
                segments.len(),
                provider,
                content_splitter::platform_limit(&provider)
            ));
        }

        for segment in &segments {
            let media_json = if segment.sequence == 1 {
                request.media.clone()
            } else {
                serde_json::json!([])
            };

            let state = if request.scheduled_at.is_some() {
                "queued"
            } else {
                "draft"
            };

            let post = queries::create_post(
                pool,
                user_id,
                *integration_id,
                &segment.content,
                None,
                &media_json,
                &request.settings,
                request.scheduled_at,
                None,
                None,
                segment.sequence as i32,
            )
            .await
            .map_err(|e| format!("Failed to create post: {e}"))?;

            if let Some(scheduled_at) = request.scheduled_at {
                queries::schedule_post(pool, post.id, user_id, scheduled_at)
                    .await
                    .map_err(|e| format!("Failed to schedule post: {e}"))?;
            }

            staged.push(StagedPost {
                post_id: post.id,
                provider: provider.clone(),
                sequence: segment.sequence,
                total_segments: segment.total,
                state: state.to_string(),
            });
        }
    }

    Ok(StagingResult {
        total_posts: staged.len(),
        staged,
        warnings,
    })
}

pub fn validate_staging_request(request: &StagingRequest) -> Result<(), String> {
    if request.content.trim().is_empty() {
        return Err("Content cannot be empty".into());
    }
    if request.integration_ids.is_empty() {
        return Err("At least one integration_id is required".into());
    }
    if request.content.len() > 100_000 {
        return Err("Content exceeds maximum length (100,000 chars)".into());
    }
    Ok(())
}
