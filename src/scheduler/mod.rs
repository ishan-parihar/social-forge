// ─── Publishing Scheduler ─────────────────────────────────────
// In-process scheduler that polls for due posts every 30 seconds.
// Handles token refresh, retry with backoff, and event broadcasting.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;

use crate::db::models::{PostState, PostWithIntegration};
use crate::db::PgPool;
use crate::realtime::Broadcaster;
use crate::social::registry::ProviderRegistry;
use crate::social::ProviderError;
use crate::social::{PostContent, SocialProvider};

use super::db::queries;

/// How often to poll for due posts
const POLL_INTERVAL_SECS: u64 = 30;

/// Maximum publish retries per post
const MAX_RETRIES: u32 = 3;

/// Backoff delay between retries (seconds)
const RETRY_BACKOFF_SECS: u64 = 10;

/// How far ahead to consider a token "expired" and refresh preemptively
const TOKEN_REFRESH_BUFFER_SECS: i64 = 300; // 5 minutes

/// Start the scheduler background task
pub fn start_scheduler(
    db: PgPool,
    providers: Arc<ProviderRegistry>,
    broadcaster: Broadcaster,
) {
    // Main post publishing scheduler
    let db1 = db.clone();
    tokio::spawn(async move {
        tracing::info!("Scheduler started (poll interval: {POLL_INTERVAL_SECS}s)");
        let mut interval = tokio::time::interval(Duration::from_secs(POLL_INTERVAL_SECS));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            if let Err(e) = process_due_posts(&db1, &providers, &broadcaster).await {
                tracing::error!("Scheduler tick error: {e}");
            }
        }
    });

    // Background cleanup task for expired oauth_states (runs every 10 minutes)
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(600));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            match queries::cleanup_expired_oauth_states(&db).await {
                Ok(count) if count > 0 => {
                    tracing::info!("Cleaned up {count} expired OAuth state(s)");
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("Failed to cleanup OAuth states: {e}");
                }
            }
        }
    });
}

/// Process all posts due for publishing
async fn process_due_posts(
    db: &PgPool,
    providers: &ProviderRegistry,
    broadcaster: &Broadcaster,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let posts = queries::get_due_posts(db).await?;

    if posts.is_empty() {
        return Ok(());
    }

    tracing::info!("Processing {} due post(s)", posts.len());

    for post in &posts {
        let provider = match providers.get(&post.provider_identifier) {
            Some(p) => p,
            None => {
                tracing::warn!(
                    "No provider found for {} on post {}",
                    post.provider_identifier,
                    post.id
                );
                continue;
            }
        };

        // Spawn each post's publishing as an isolated task to prevent a
        // single panic from killing the entire scheduler loop.
        let db_clone = db.clone();
        let provider_clone = provider.clone();
        let post_clone = post.clone();
        let bcast = broadcaster.clone();
        tokio::spawn(async move {
            if let Err(e) = publish_post(&db_clone, provider_clone.as_ref(), &post_clone, &bcast).await {
                tracing::error!("Failed to publish post {}: {e}", post_clone.id);
                let err_str = e.to_string();
                mark_post_error(&db_clone, post_clone.id, &err_str).await;
                bcast.send(
                    "post_failed",
                    &serde_json::json!({
                        "id": post_clone.id.to_string(),
                        "error": err_str,
                    }),
                );
            }
        });
    }

    // Brief pause to let spawned tasks make progress before next tick
    tokio::time::sleep(Duration::from_millis(100)).await;

    Ok(())
}

/// Publish a single post, with token refresh and retry logic
async fn publish_post(
    db: &PgPool,
    provider: &dyn SocialProvider,
    post: &PostWithIntegration,
    broadcaster: &Broadcaster,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Resolve access token, potentially refreshed
    let mut access_token = resolve_token(db, provider, post).await?;

    // Build post content
    let content = PostContent {
        content: post.content.clone(),
        media: vec![],
        settings: post.settings.clone(),
    };

    // Validate content against provider limits before publishing
    provider.validate_post(&content)
        .map_err(|e| format!("Content validation failed: {e}"))?;

    // Publish with retry
    let mut last_error: Option<String> = None;
    let mut did_refresh = false; // guard against token refresh recursion

    for attempt in 1..=MAX_RETRIES {
        match provider.publish(&access_token, &content).await {
            Ok(result) => {
                queries::update_post_state(
                    db,
                    post.id,
                    PostState::Published,
                    Some(&result.platform_post_id),
                    result.platform_post_url.as_deref(),
                    None,
                )
                .await?;

                tracing::info!(
                    "Post {} published on {}: {}",
                    post.id,
                    post.provider_identifier,
                    result.platform_post_url.as_deref().unwrap_or("(no URL)")
                );

                broadcaster.send(
                    "post_published",
                    &serde_json::json!({
                        "id": post.id.to_string(),
                        "platform_post_url": result.platform_post_url,
                        "provider": post.provider_identifier,
                    }),
                );

                return Ok(());
            }
            Err(ProviderError::TokenExpired) if !did_refresh => {
                tracing::warn!("Token expired for post {} (attempt {}). Refreshing...", post.id, attempt);
                did_refresh = true;
                let new_token = match provider.refresh_token(
                    post.refresh_token.as_deref().unwrap_or(""),
                ).await {
                    Ok(t) => {
                        queries::update_integration_token(
                            db,
                            post.integration_id,
                            &t.access_token,
                            t.refresh_token.as_deref(),
                            t.expires_in.map(|e| Utc::now() + chrono::Duration::seconds(e as i64)),
                        ).await?;
                        t.access_token
                    }
                    Err(e) => {
                        tracing::error!("Token refresh failed for post {}: {e}", post.id);
                        return Err(e.into());
                    }
                };
                // Retry with new token (still inside the retry loop)
                access_token = new_token;
                continue;
            }
            Err(ProviderError::TokenExpired) => {
                // Already refreshed once; second TokenExpired means refresh didn't help
                tracing::error!("Token still expired after refresh for post {}", post.id);
                last_error = Some("Token expired and refresh did not resolve it".to_string());
                break;
            }
            Err(ProviderError::RateLimited(ref msg)) => {
                let wait = RETRY_BACKOFF_SECS * attempt as u64;
                tracing::warn!("Rate limited for post {}: {}. Retrying in {}s", post.id, msg, wait);
                last_error = Some(msg.clone());
                tokio::time::sleep(Duration::from_secs(wait)).await;
                continue;
            }
            Err(e) => {
                tracing::error!("Publish error for post {}: {e}", post.id);
                last_error = Some(e.to_string());
                break;
            }
        }
    }

    Err(last_error.unwrap_or_else(|| "Max retries exceeded".to_string()).into())
}

/// Publish with a known-good access token (used after refresh — now integrated into publish_post)
#[allow(dead_code)]
async fn publish_with_token(
    db: &PgPool,
    provider: &dyn SocialProvider,
    access_token: &str,
    post: &PostWithIntegration,
    broadcaster: &Broadcaster,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let content = PostContent {
        content: post.content.clone(),
        media: vec![],
        settings: post.settings.clone(),
    };

    match provider.publish(access_token, &content).await {
        Ok(result) => {
            queries::update_post_state(
                db,
                post.id,
                PostState::Published,
                Some(&result.platform_post_id),
                result.platform_post_url.as_deref(),
                None,
            )
            .await?;

            broadcaster.send(
                "post_published",
                &serde_json::json!({
                    "id": post.id.to_string(),
                    "platform_post_url": result.platform_post_url,
                    "provider": post.provider_identifier,
                }),
            );

            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

/// Resolve a valid access token, refreshing if necessary
async fn resolve_token(
    db: &PgPool,
    provider: &dyn SocialProvider,
    post: &PostWithIntegration,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let needs_refresh = match post.token_expires_at {
        Some(exp) => Utc::now() + chrono::Duration::seconds(TOKEN_REFRESH_BUFFER_SECS) >= exp,
        None => false,
    };

    if needs_refresh {
        tracing::info!("Token for post {} is expiring, refreshing", post.id);
        let token = provider
            .refresh_token(post.refresh_token.as_deref().unwrap_or(""))
            .await?;

        queries::update_integration_token(
            db,
            post.integration_id,
            &token.access_token,
            token.refresh_token.as_deref(),
            token
                .expires_in
                .map(|e| Utc::now() + chrono::Duration::seconds(e as i64)),
        )
        .await?;

        Ok(token.access_token)
    } else {
        Ok(post.access_token.clone())
    }
}

/// Mark a post as error
async fn mark_post_error(db: &PgPool, post_id: uuid::Uuid, error: &str) {
    if let Err(e) = queries::update_post_state(db, post_id, PostState::Error, None, None, Some(error)).await {
        tracing::error!("Failed to mark post {post_id} as error: {e}");
    }
}
