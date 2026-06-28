// ─── Post Service ─────────────────────────────────────────────
// Shared business logic for post CRUD, scheduling, and slot-finding.
// Used by both `api/posts.rs` (HTTP) and `mcp/tools_posts.rs` (MCP).

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::models::{Post, PostState, PostWithIntegration};
use crate::db::queries;
use crate::realtime::Broadcaster;
use crate::social::registry::ProviderRegistry;
use crate::social::{PostContent, SocialProvider};

/// Result type for service operations
pub type ServiceResult<T> = Result<T, String>;

/// Input for creating a post
pub struct CreatePostInput {
    pub user_id: Uuid,
    pub integration_id: Uuid,
    pub content: String,
    pub title: Option<String>,
    pub media_urls: Value,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub settings: Value,
}

/// Input for updating a post
pub struct UpdatePostInput {
    pub content: Option<String>,
    pub title: Option<String>,
    pub media: Option<Value>,
    pub settings: Option<Value>,
}

/// Input for scheduling
pub struct SchedulePostInput {
    pub scheduled_at: DateTime<Utc>,
}

/// Shared post service
pub struct PostService;

impl PostService {
    /// Sanitize post content: strip HTML tags and limit length
    fn sanitize_content(raw: &str, max_len: usize) -> String {
        let mut clean = String::with_capacity(raw.len());
        let mut in_tag = false;
        for ch in raw.chars() {
            match ch {
                '<' => in_tag = true,
                '>' if in_tag => in_tag = false,
                _ if !in_tag => clean.push(ch),
                _ => {}
            }
        }
        let clean = clean.trim();
        if clean.len() > max_len {
            clean[..max_len].to_string()
        } else {
            clean.to_string()
        }
    }

    /// Create a new post (draft)
    pub async fn create(
        db: &PgPool,
        broadcaster: &Broadcaster,
        input: CreatePostInput,
    ) -> ServiceResult<Post> {
        let content = Self::sanitize_content(&input.content, 2000);
        if content.trim().is_empty() {
            return Err("Content cannot be empty".into());
        }

        let post = queries::create_post(
            db,
            input.user_id,
            input.integration_id,
            &content,
            input.title.as_deref(),
            &input.media_urls,
            &input.settings,
            input.scheduled_at,
            Some(PostState::Draft),
            None,
            0,
        )
        .await
        .map_err(|e| format!("Database error: {e}"))?;

        broadcaster.send(
            "post_created",
            &serde_json::json!({"id": post.id.to_string()}),
        );

        Ok(post)
    }

    /// List posts with optional state filtering and pagination
    pub async fn list(
        db: &PgPool,
        user_id: Uuid,
        state_filter: Option<&str>,
        limit: i64,
        offset: i64,
        include_total: bool,
    ) -> ServiceResult<(Vec<Post>, Option<i64>)> {
        let posts = queries::list_posts(db, user_id, state_filter, limit, offset)
            .await
            .map_err(|e| format!("Database error: {e}"))?;

        let total = if include_total {
            Some(
                queries::count_posts_by_user(db, user_id, state_filter)
                    .await
                    .map_err(|e| format!("Database error: {e}"))?,
            )
        } else {
            None
        };

        Ok((posts, total))
    }

    /// Get a single post by ID (verifies ownership)
    pub async fn get(
        db: &PgPool,
        user_id: Uuid,
        post_id: Uuid,
    ) -> ServiceResult<Post> {
        let post = queries::get_post(db, post_id, user_id)
            .await
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Post not found".to_string())?;

        Ok(post)
    }

    /// Update a post (verifies ownership)
    pub async fn update(
        db: &PgPool,
        broadcaster: &Broadcaster,
        user_id: Uuid,
        post_id: Uuid,
        input: UpdatePostInput,
    ) -> ServiceResult<Post> {
        // Verify ownership before update
        Self::get(db, user_id, post_id).await?;

        let post = queries::update_post_content(
            db,
            post_id,
            user_id,
            &input.content.unwrap_or_default(),
            input.title.as_deref(),
            &input.media.unwrap_or(Value::Null),
            &input.settings.unwrap_or(Value::Null),
        )
        .await
        .map_err(|e| format!("Database error: {e}"))?
        .ok_or_else(|| "Post not found after update".to_string())?;

        broadcaster.send(
            "post_updated",
            &serde_json::json!({"id": post_id.to_string()}),
        );

        Ok(post)
    }

    /// Schedule a post (verifies ownership)
    pub async fn schedule(
        db: &PgPool,
        broadcaster: &Broadcaster,
        user_id: Uuid,
        post_id: Uuid,
        scheduled_at: DateTime<Utc>,
    ) -> ServiceResult<Post> {
        // Verify ownership
        Self::get(db, user_id, post_id).await?;

        let post = queries::schedule_post(db, post_id, user_id, scheduled_at)
            .await
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Post not found after scheduling".to_string())?;

        broadcaster.send(
            "post_scheduled",
            &serde_json::json!({"id": post_id.to_string()}),
        );

        Ok(post)
    }

    /// Delete a post (ownership verified via WHERE user_id = $2 in query)
    pub async fn delete(
        db: &PgPool,
        broadcaster: &Broadcaster,
        user_id: Uuid,
        post_id: Uuid,
    ) -> ServiceResult<bool> {
        let deleted = queries::delete_post(db, post_id, user_id)
            .await
            .map_err(|e| format!("Database error: {e}"))?;

        broadcaster.send(
            "post_deleted",
            &serde_json::json!({"id": post_id.to_string()}),
        );

        Ok(deleted)
    }

    /// Find next available time slot
    pub async fn find_slot(
        db: &PgPool,
        user_id: Uuid,
        integration_id: Option<Uuid>,
    ) -> ServiceResult<DateTime<Utc>> {
        queries::find_next_free_slot(db, user_id, integration_id)
            .await
            .map_err(|e| format!("Database error: {e}"))
            .map(|opt| opt.unwrap_or_else(Utc::now))
    }

    /// Publish a post immediately (or retry a failed post).
    /// Returns the platform_post_url on success.
    pub async fn publish(
        db: &PgPool,
        providers: &ProviderRegistry,
        broadcaster: &Broadcaster,
        user_id: Uuid,
        post_id: Uuid,
    ) -> ServiceResult<String> {
        // Fetch post with integration details
        let post = queries::get_post_with_integration(db, post_id, user_id)
            .await
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Post not found".to_string())?;

        // Only allow publishing queued or errored posts
        match post.state {
            PostState::Draft => return Err("Draft posts must be scheduled first".into()),
            PostState::Published => return Err("Post already published".into()),
            _ => {} // queued or error is OK
        }

        if post.integration_disabled {
            return Err("Integration is disabled. Reconnect the social account.".into());
        }

        let provider = providers
            .get(&post.provider_identifier)
            .ok_or_else(|| format!("Provider '{}' not found", post.provider_identifier))?;

        // Resolve token, refreshing if needed
        let access_token = Self::resolve_token(db, provider.as_ref(), &post).await?;

        // Build publish content (load media from post — matches scheduler behavior)
        let media: Vec<crate::social::MediaAttachment> =
            serde_json::from_value(post.media.clone()).unwrap_or_default();
        let content = PostContent {
            content: Self::sanitize_content(&post.content, 2000),
            media,
            settings: post.settings.clone(),
        };

        // Validate
        provider
            .validate_post(&content)
            .map_err(|e| format!("Content validation failed: {e}"))?;

        // Publish
        let result = provider
            .publish(&access_token, &content)
            .await
            .map_err(|e| format!("Publish failed: {e}"))?;

        // Update state
        queries::update_post_state(
            db,
            post_id,
            PostState::Published,
            Some(&result.platform_post_id),
            result.platform_post_url.as_deref(),
            None,
        )
        .await
        .map_err(|e| format!("Database error: {e}"))?;

        // Publish first_comment if present (matches scheduler behavior)
        if let Some(ref comment_text) = post.first_comment {
            if !comment_text.is_empty() {
                let comment_content = PostContent {
                    content: comment_text.clone(),
                    media: vec![],
                    settings: serde_json::json!({}),
                };
                if let Err(e) = provider
                    .comment(
                        &access_token,
                        &result.platform_post_id,
                        None,
                        &comment_content,
                    )
                    .await
                {
                    tracing::warn!("Failed to post first_comment for {}: {e}", post_id);
                }
            }
        }

        let platform_url = result.platform_post_url.unwrap_or_default();

        broadcaster.send(
            "post_published",
            &serde_json::json!({
                "id": post_id.to_string(),
                "platform_post_url": platform_url,
                "provider": post.provider_identifier,
            }),
        );

        Ok(platform_url)
    }

    /// Resolve an access token, refreshing if it's about to expire
    async fn resolve_token(
        db: &PgPool,
        provider: &dyn SocialProvider,
        post: &PostWithIntegration,
    ) -> ServiceResult<String> {
        const TOKEN_REFRESH_BUFFER: i64 = 300; // 5 minutes

        let needs_refresh = match post.token_expires_at {
            Some(exp) => Utc::now() + chrono::Duration::seconds(TOKEN_REFRESH_BUFFER) >= exp,
            None => false,
        };

        if needs_refresh {
            let token = provider
                .refresh_token(post.refresh_token.as_deref().unwrap_or(""))
                .await
                .map_err(|e| format!("Token refresh failed: {e}"))?;

            queries::update_integration_token(
                db,
                post.integration_id,
                &token.access_token,
                token.refresh_token.as_deref(),
                token
                    .expires_in
                    .map(|e| Utc::now() + chrono::Duration::seconds(e as i64)),
            )
            .await
            .map_err(|e| format!("Failed to save refreshed token: {e}"))?;

            Ok(token.access_token)
        } else {
            Ok(post.access_token.clone())
        }
    }

    /// Get calendar posts by date range
    pub async fn calendar(
        db: &PgPool,
        user_id: Uuid,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> ServiceResult<Vec<Post>> {
        queries::get_posts_by_date_range(db, user_id, start, end)
            .await
            .map_err(|e| format!("Database error: {e}"))
    }
}
