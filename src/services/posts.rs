// ─── Post Service ─────────────────────────────────────────────
// Shared business logic for post CRUD, scheduling, and slot-finding.
// Used by both `api/posts.rs` (HTTP) and `mcp/tools_posts.rs` (MCP).

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::SqlitePool;
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
    pub first_comment: Option<String>,
    /// Optional state hint. If `None`, defaults to `Draft`. Pass `Some(Queued)` after `--schedule` to arm the post.
    pub state: Option<PostState>,
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
    pub fn sanitize_content(raw: &str, max_len: usize) -> String {
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
        if clean.chars().count() > max_len {
            // Use char-boundary-safe truncation to avoid panicking on
            // multi-byte UTF-8 sequences (e.g., emoji). `String::len()`
            // returns byte length, and slicing `clean[..max_len]` would
            // panic if `max_len` falls inside a multi-byte sequence.
            clean.chars().take(max_len).collect()
        } else {
            clean.to_string()
        }
    }

    /// Create a new post
    pub async fn create(
        db: &SqlitePool,
        broadcaster: &Broadcaster,
        input: CreatePostInput,
    ) -> ServiceResult<Post> {
        let content = Self::sanitize_content(&input.content, 2000);
        if content.trim().is_empty() {
            return Err("Content cannot be empty".into());
        }
        if content.chars().count() > 2000 {
            return Err(format!("Content exceeds 2000 chars (got {})", content.chars().count()));
        }

        let state = input.state.unwrap_or(PostState::Draft);

        let post = queries::create_post(
            db,
            input.user_id,
            input.integration_id,
            &content,
            input.title.as_deref(),
            &input.media_urls,
            &input.settings,
            input.scheduled_at,
            Some(state),
            input.first_comment.as_deref(),
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
        db: &SqlitePool,
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
        db: &SqlitePool,
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
        db: &SqlitePool,
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
        db: &SqlitePool,
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

    /// Delete a post (ownership verified via `WHERE user_id = ?` in the query)
    pub async fn delete(
        db: &SqlitePool,
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
        db: &SqlitePool,
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
    ///
    /// `token_key` is the optional AES-256 key used for at-rest token
    /// encryption. When `Some`, freshly refreshed access tokens are
    /// encrypted before being written to the DB — matching the
    /// scheduler's behavior. When `None`, tokens are stored as-is
    /// (legacy/dev mode).
    pub async fn publish(
        db: &SqlitePool,
        providers: &ProviderRegistry,
        broadcaster: &Broadcaster,
        user_id: Uuid,
        post_id: Uuid,
        token_key: Option<[u8; 32]>,
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
        let access_token = Self::resolve_token(db, provider.as_ref(), &post, token_key).await?;

        // Build publish content (load media from post — matches scheduler behavior)
        let media: Vec<crate::social::MediaAttachment> =
            serde_json::from_value(post.media.clone()).unwrap_or_default();
        // Resolve relative media URLs to absolute URLs for providers that need them
        // (Instagram, Facebook, Threads, Reddit require absolute URLs)
        let app_url = std::env::var("APP_URL").unwrap_or_else(|_| "https://localhost:6543".into());
        let resolved_media: Vec<crate::social::MediaAttachment> = media.into_iter().map(|m| {
            provider.resolve_media_url(&m, &app_url)
        }).collect();

        // ── Thread linking ──────────────────────────────────
        // If this post is part of a thread (group_id + sequence > 1),
        // look up the predecessor's platform_post_id for in_reply_to.
        let in_reply_to: Option<String> = if let Some(ref group_id) = post.group_id.as_ref() {
            let seq = post.sequence;
            if seq > 1 {
                match sqlx::query_scalar::<_, Option<String>>(
                    r#"SELECT platform_post_id FROM posts
                       WHERE group_id = ? AND sequence = ?
                         AND state = 'published' AND platform_post_id IS NOT NULL
                       LIMIT 1"#,
                )
                .bind(group_id)
                .bind(seq - 1)
                .fetch_optional(db)
                .await
                {
                    Ok(Some(Some(pid))) => Some(pid),
                    _ => None,
                }
            } else {
                None
            }
        } else {
            None
        };

        let content = PostContent {
            content: Self::sanitize_content(&post.content, provider.max_content_length()),
            media: resolved_media,
            settings: post.settings.clone(),
            in_reply_to,
            // v22 Phase 1 (BUG #15): pass the post's idempotency_key so
            // providers that support the `Idempotency-Key` HTTP header
            // (X v2, LinkedIn, Reddit, Slack) can deduplicate a retry
            // after a crash. Previously this was `None`, meaning a
            // manual "Post Now" after a stuck-publishing reclaim would
            // create a duplicate post on the platform.
            idempotency_key: Some(post.idempotency_key.to_string()),
            delay_minutes: post.settings.get("delay_minutes")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32),
        };

        // Validate
        provider
            .validate_post(&content)
            .map_err(|e| format!("Content validation failed: {e}"))?;

        provider
            .validate_media(&content)
            .map_err(|e| format!("Media validation failed: {e}"))?;

        // Publish
        let result = provider
            .publish(&access_token, &content)
            .await
            .map_err(|e| format!("Publish failed: {e}"))?;

        // G-01: a provider can report success and hand back no post id.
        // X's GraphQL CreateTweet answers 200 with no `rest_id` when the
        // tweet was never created, and the provider maps that to
        // `platform_post_id: ""` with `status: "published"`. The
        // scheduler's terminal-state check cannot catch this — the status
        // literally reads "published" — so the empty id is the only signal
        // left. Recording that row as published is the false success: the
        // DB claims the post is live, nothing is live, and no retry runs.
        //
        // An empty id is never a publish receipt, so refuse to transition
        // state and leave the row retryable.
        if result.platform_post_id.trim().is_empty() {
            let err = format!(
                "Publish failed: {} reported success but returned no platform_post_id",
                post.provider_identifier
            );
            tracing::error!("Post {}: {err}", post_id);

            // Same audit contract as the scheduler: one row per publish call.
            let _ = queries::record_publish_attempt(db, post_id, 1, "failed", Some(&err), Utc::now())
                .await;

            // Write the reason onto the row while keeping its incoming
            // state (queued/error) — the post was never published, and the
            // scheduler or the operator still has to retry it.
            if let Err(e) = queries::update_post_state(
                db,
                post_id,
                post.state.clone(),
                None,
                None,
                Some(&err),
            )
            .await
            {
                tracing::error!("Failed to record publish error for post {post_id}: {e}");
            }

            return Err(err);
        }

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
                    ..Default::default()
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

    /// Resolve an access token, refreshing if it's about to expire.
    ///
    /// SECURITY: when `token_key` is `Some`, the freshly-refreshed
    /// access token is AES-256-GCM encrypted before being written to
    /// the DB — matching the scheduler's behavior. Previously this
    /// path stored the raw token, silently downgrading at-rest
    /// encryption on every manual publish.
    async fn resolve_token(
        db: &SqlitePool,
        provider: &dyn SocialProvider,
        post: &PostWithIntegration,
        token_key: Option<[u8; 32]>,
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

            // Encrypt the access token before storing if encryption is configured.
            // On encryption failure we fall back to the raw token (with a warning)
            // rather than failing the publish — matching scheduler behavior.
            let enc_access_token = if let Some(ref k) = token_key {
                crate::crypto::encrypt_string(&token.access_token, k)
                    .unwrap_or_else(|e| {
                        tracing::warn!(
                            "Failed to encrypt refreshed access token for integration {}: {e}. \
                             Storing unencrypted (downgrade).",
                            post.integration_id
                        );
                        token.access_token.clone()
                    })
            } else {
                token.access_token.clone()
            };

            queries::update_integration_token(
                db,
                post.integration_id,
                &enc_access_token,
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
        db: &SqlitePool,
        user_id: Uuid,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> ServiceResult<Vec<Post>> {
        queries::get_posts_by_date_range(db, user_id, start, end)
            .await
            .map_err(|e| format!("Database error: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::social::{AuthToken, AuthUrlResponse, PageInfo, ProviderError, PublishResult};
    use std::sync::Arc;

    /// Returns whatever `PublishResult` the test hands it — including the
    /// G-01 shape, where the provider claims `published` but has no id.
    struct FakeProvider {
        result: PublishResult,
    }

    #[async_trait::async_trait]
    impl SocialProvider for FakeProvider {
        fn identifier(&self) -> &'static str {
            "fake"
        }
        fn name(&self) -> &'static str {
            "Fake"
        }
        fn scopes(&self) -> Vec<String> {
            vec![]
        }
        fn max_content_length(&self) -> usize {
            5000
        }

        async fn generate_auth_url(
            &self,
            _s: &str,
            _v: &str,
            _r: &str,
        ) -> Result<AuthUrlResponse, ProviderError> {
            unimplemented!()
        }
        async fn exchange_code(
            &self,
            _c: &str,
            _v: &str,
            _r: &str,
        ) -> Result<AuthToken, ProviderError> {
            unimplemented!()
        }
        async fn refresh_token(&self, _r: &str) -> Result<AuthToken, ProviderError> {
            unimplemented!()
        }
        async fn fetch_page_info(
            &self,
            _t: &str,
            _p: &str,
        ) -> Result<PageInfo, ProviderError> {
            unimplemented!()
        }
        async fn publish(
            &self,
            _t: &str,
            _p: &PostContent,
        ) -> Result<PublishResult, ProviderError> {
            Ok(self.result.clone())
        }
    }

    /// In-memory SQLite carrying the real schema, so `publish` runs against
    /// the same columns and constraints production uses.
    async fn test_pool() -> SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(sqlx::sqlite::SqliteConnectOptions::new().in_memory(true))
            .await
            .expect("in-memory sqlite pool");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");
        pool
    }

    /// A user + integration + `queued` post wired to the `fake` provider,
    /// i.e. exactly what `posts publish` is pointed at.
    async fn queued_post(pool: &SqlitePool) -> (Uuid, Uuid) {
        let user = queries::create_user(pool, "publish-guard@test.local", "hash", "Guard")
            .await
            .expect("user");
        let integration = queries::create_integration(
            pool,
            user.id,
            "fake",
            "Fake",
            "acct-1",
            "access-token",
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("integration");
        let post = queries::create_post(
            pool,
            user.id,
            integration.id,
            "hello from the forge",
            None,
            &serde_json::json!([]),
            &serde_json::json!({}),
            None,
            Some(PostState::Queued),
            None,
            0,
        )
        .await
        .expect("post");
        (user.id, post.id)
    }

    fn registry(result: PublishResult) -> ProviderRegistry {
        ProviderRegistry::with_providers(vec![(
            "fake",
            Arc::new(FakeProvider { result }) as Arc<dyn SocialProvider>,
        )])
    }

    /// G-01: X's GraphQL CreateTweet answered 200 with no `rest_id`, so the
    /// provider returned `platform_post_id: ""` alongside
    /// `status: "published"`. The post was never created, but before the
    /// empty-id guard this wrote the row as published with a phantom id —
    /// a silent loss the operator only found by searching X.
    #[tokio::test]
    async fn publish_rejects_a_success_that_carries_no_platform_post_id() {
        let pool = test_pool().await;
        let (user_id, post_id) = queued_post(&pool).await;
        let providers = registry(PublishResult {
            platform_post_id: String::new(),
            platform_post_url: None,
            status: "published".into(),
        });

        let err = PostService::publish(&pool, &providers, &Broadcaster::new(), user_id, post_id, None)
            .await
            .expect_err("an empty platform_post_id is not a publish receipt");
        assert!(err.contains("platform_post_id"), "error must name the cause: {err}");

        let post = queries::get_post(&pool, post_id, user_id)
            .await
            .expect("read back")
            .expect("row still exists");

        // The post never went live, so the row must not claim it did.
        assert_ne!(
            post.state,
            PostState::Published,
            "row was marked published for a post that was never created"
        );
        assert!(post.published_at.is_none(), "published_at must stay unset");
        assert!(
            post.platform_post_id.as_deref().unwrap_or("").is_empty(),
            "platform_post_id must stay empty, got {:?}",
            post.platform_post_id
        );
        // Row keeps its retryable incoming state, with the reason attached.
        assert_eq!(post.state, PostState::Queued);
        assert!(
            post.error_message.as_deref().unwrap_or("").contains("platform_post_id"),
            "error_message must explain the failure, got {:?}",
            post.error_message
        );

        // Audit trail: one failed attempt, so this is not invisible.
        let attempts: Vec<(i32, String, Option<String>)> =
            sqlx::query_as("SELECT attempt_number, status, error_message FROM publish_attempts WHERE post_id = ?")
                .bind(post_id)
                .fetch_all(&pool)
                .await
                .expect("attempts");
        assert_eq!(attempts.len(), 1, "expected one attempt row, got {attempts:?}");
        assert_eq!(attempts[0].0, 1);
        assert_eq!(attempts[0].1, "failed");
        assert!(attempts[0].2.as_deref().unwrap_or("").contains("platform_post_id"));
    }

    /// The control: a real id still publishes, and the id is what lands.
    #[tokio::test]
    async fn publish_records_the_platform_post_id_when_the_platform_returns_one() {
        let pool = test_pool().await;
        let (user_id, post_id) = queued_post(&pool).await;
        let providers = registry(PublishResult {
            platform_post_id: "tweet-123".into(),
            platform_post_url: Some("https://x.com/i/status/tweet-123".into()),
            status: "published".into(),
        });

        let url = PostService::publish(&pool, &providers, &Broadcaster::new(), user_id, post_id, None)
            .await
            .expect("a real platform_post_id must publish");
        assert_eq!(url, "https://x.com/i/status/tweet-123");

        let post = queries::get_post(&pool, post_id, user_id)
            .await
            .expect("read back")
            .expect("row still exists");
        assert_eq!(post.state, PostState::Published);
        assert_eq!(post.platform_post_id.as_deref(), Some("tweet-123"));
        assert!(post.published_at.is_some(), "published_at must be stamped");
    }
}
