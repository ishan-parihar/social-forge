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

/// Max posts processed per scheduler tick
const DUE_POSTS_LIMIT: i64 = 50;

/// Backoff delay between retries (seconds)
const RETRY_BACKOFF_SECS: u64 = 10;

/// How far ahead to consider a token "expired" and refresh preemptively
const TOKEN_REFRESH_BUFFER_SECS: i64 = 300; // 5 minutes

/// Start the scheduler background task.
/// Pass a watch::Receiver that resolves to `true` to trigger graceful shutdown.
pub fn start_scheduler(
    db: PgPool,
    providers: Arc<ProviderRegistry>,
    broadcaster: Broadcaster,
    token_key: Option<[u8; 32]>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    // Main post publishing scheduler
    let db1 = db.clone();
    let providers1 = providers.clone();
    let mut shutdown1 = shutdown.clone();
    tokio::spawn(async move {
        tracing::info!("Scheduler started (poll interval: {POLL_INTERVAL_SECS}s)");
        let mut interval = tokio::time::interval(Duration::from_secs(POLL_INTERVAL_SECS));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = shutdown1.changed() => {
                    if *shutdown1.borrow() {
                        tracing::info!("Scheduler shutting down...");
                        break;
                    }
                }
                _ = interval.tick() => {
                    if let Err(e) = process_due_posts(&db1, &providers1, &broadcaster, token_key).await {
                        tracing::error!("Scheduler tick error: {e}");
                    }
                }
            }
        }
    });

    // Background cleanup task for expired oauth_states (runs every 10 minutes)
    let db_cleanup = db.clone();
    let mut shutdown_cleanup = shutdown.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(600));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = shutdown_cleanup.changed() => {
                    if *shutdown_cleanup.borrow() {
                        tracing::info!("OAuth cleanup task shutting down...");
                        break;
                    }
                }
                _ = interval.tick() => {
                    match queries::cleanup_expired_oauth_states(&db_cleanup).await {
                        Ok(count) if count > 0 => {
                            tracing::info!("Cleaned up {count} expired OAuth state(s)");
                        }
                        Ok(_) => {}
                        Err(e) => {
                            tracing::error!("Failed to cleanup OAuth states: {e}");
                        }
                    }
                }
            }
        }
    });

    // Proactive token refresh task (runs every 6 hours)
    // Refreshes tokens expiring within 24h to prevent silent expiration
    let db2 = db.clone();
    let providers2 = providers.clone();
    let mut shutdown_token = shutdown.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(21600)); // 6 hours
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = shutdown_token.changed() => {
                    if *shutdown_token.borrow() {
                        tracing::info!("Proactive token refresh shutting down...");
                        break;
                    }
                }
                _ = interval.tick() => {
                    if let Err(e) = proactive_token_refresh(&db2, &providers2, token_key).await {
                        tracing::error!("Proactive token refresh error: {e}");
                    }
                }
            }
        }
    });
}

/// Proactive token refresh: refreshes tokens expiring within 24h for providers that need cron refresh
async fn proactive_token_refresh(
    db: &PgPool,
    providers: &ProviderRegistry,
    token_key: Option<[u8; 32]>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    const PROACTIVE_REFRESH_WINDOW_HOURS: i64 = 24;
    
    // Get all providers that need cron refresh
    let providers_needing_refresh: Vec<_> = providers
        .list()
        .into_iter()
        .filter(|id| {
            providers.get(id)
                .map(|p| p.needs_cron_refresh())
                .unwrap_or(false)
        })
        .collect();
    
    if providers_needing_refresh.is_empty() {
        return Ok(());
    }
    
    tracing::info!(
        "Running proactive token refresh for {} provider(s): {:?}",
        providers_needing_refresh.len(),
        providers_needing_refresh
    );
    
    let mut refreshed_count = 0;
    let mut failed_count = 0;
    
    for provider_id in &providers_needing_refresh {
        let provider = match providers.get(provider_id) {
            Some(p) => p,
            None => continue,
        };
        
        // Get integrations with tokens expiring within 24h
        let integrations = queries::get_integrations_needing_refresh(
            db,
            provider_id,
            PROACTIVE_REFRESH_WINDOW_HOURS,
        )
        .await?;
        
        if integrations.is_empty() {
            continue;
        }
        
        tracing::info!(
            "Proactive refresh: {} {} integration(s) need refresh",
            integrations.len(),
            provider_id
        );
        
        for integration in &integrations {
            // Skip if integration already has refresh_needed flag
            if integration.refresh_needed {
                continue;
            }
            
            let refresh_token = match &integration.refresh_token {
                Some(rt) => rt,
                None => {
                    tracing::warn!(
                        "Integration {} has no refresh token, marking as refresh_needed",
                        integration.id
                    );
                    let _ = queries::mark_integration_refresh_needed(db, integration.id).await;
                    failed_count += 1;
                    continue;
                }
            };
            
            // Decrypt refresh token if encrypted
            let decrypted_refresh_token = if let Some(key) = token_key {
                crate::crypto::decrypt_string(refresh_token, &key)
                    .unwrap_or_else(|_| refresh_token.clone())
            } else {
                refresh_token.clone()
            };
            
            // Attempt token refresh
            match provider.refresh_token(&decrypted_refresh_token).await {
                Ok(new_token) => {
                    // Encrypt new tokens if encryption is enabled
                    let (access_token, refresh_token_to_store) = if let Some(key) = token_key {
                        let enc_access = crate::crypto::encrypt_string(&new_token.access_token, &key)?;
                        let enc_refresh = new_token.refresh_token.as_ref()
                            .map(|rt| crate::crypto::encrypt_string(rt, &key))
                            .transpose()?;
                        (enc_access, enc_refresh)
                    } else {
                        (new_token.access_token.clone(), new_token.refresh_token.clone())
                    };
                    
                    // Atomic persistence of the rotating refresh token
                    let expires_at = new_token.expires_in
                        .map(|e| Utc::now() + chrono::Duration::seconds(e as i64));
                    
                    queries::update_integration_token(
                        db,
                        integration.id,
                        &access_token,
                        refresh_token_to_store.as_deref(),
                        expires_at,
                    )
                    .await?;
                    
                    tracing::debug!(
                        "Proactively refreshed token for {} integration {}",
                        provider_id,
                        integration.id
                    );
                    
                    // LinkedIn needs 10s propagation delay after refresh
                    if provider.refresh_wait() {
                        tokio::time::sleep(Duration::from_secs(10)).await;
                    }
                    
                    refreshed_count += 1;
                }
                Err(e) => {
                    tracing::warn!(
                        "Proactive refresh failed for {} integration {}: {e}",
                        provider_id,
                        integration.id
                    );
                    // Mark as needing reconnection
                    let _ = queries::mark_integration_refresh_needed(db, integration.id).await;
                    failed_count += 1;
                }
            }
        }
    }
    
    if refreshed_count > 0 || failed_count > 0 {
        tracing::info!(
            "Proactive token refresh complete: {} refreshed, {} failed",
            refreshed_count,
            failed_count
        );
    }
    
    Ok(())
}

/// Process all posts due for publishing
async fn process_due_posts(
    db: &PgPool,
    providers: &ProviderRegistry,
    broadcaster: &Broadcaster,
    token_key: Option<[u8; 32]>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut posts = queries::get_due_posts(db, DUE_POSTS_LIMIT).await?;

    // Decrypt tokens in-place if token_key is configured
    if let Some(key) = token_key {
        for post in &mut posts {
            if let Ok(decrypted) = crate::crypto::decrypt_string(&post.access_token, &key) {
                post.access_token = decrypted;
            }
            if let Some(ref rt) = post.refresh_token {
                if let Ok(decrypted) = crate::crypto::decrypt_string(rt, &key) {
                    post.refresh_token = Some(decrypted);
                }
            }
        }
    }

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
        let tk = token_key; // Copy (Option<[u8; 32]> implements Copy)
        tokio::spawn(async move {
            if let Err(e) = publish_post(&db_clone, provider_clone.as_ref(), &post_clone, &bcast, tk).await {
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
    token_key: Option<[u8; 32]>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Resolve access token, potentially refreshed
    let mut access_token = resolve_token(db, provider, post).await?;

    // Build post content - resolve media URLs to absolute paths
    let media: Vec<crate::social::MediaAttachment> = serde_json::from_value(post.media.clone()).unwrap_or_default();
    let app_url = std::env::var("APP_URL").unwrap_or_else(|_| "https://localhost:6543".into());
    let resolved_media: Vec<crate::social::MediaAttachment> = media.into_iter().map(|m| {
        // Resolve relative media URLs to absolute URLs for providers that need them
        // (Instagram, Facebook, Threads, Reddit require absolute URLs)
        provider.resolve_media_url(&m, &app_url)
    }).collect();
    let content = PostContent {
        content: post.content.clone(),
        media: resolved_media,
        settings: post.settings.clone(),
    };

    // Validate content against provider limits before publishing
    provider.validate_post(&content)
        .map_err(|e| format!("Content validation failed: {e}"))?;

    provider.validate_media(&content)
        .map_err(|e| format!("Media validation failed: {e}"))?;

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

                // Publish first_comment if present
                if let Some(ref comment_text) = post.first_comment {
                    if !comment_text.is_empty() {
                        let comment_content = PostContent {
                            content: comment_text.clone(),
                            media: vec![],
                            settings: serde_json::json!({}),
                        };
                        if let Err(e) = provider.comment(
                            &access_token,
                            &result.platform_post_id,
                            None,
                            &comment_content,
                        ).await {
                            tracing::warn!("Failed to post first_comment for {}: {e}", post.id);
                        }
                    }
                }

                return Ok(());
            }
            Err(ProviderError::TokenExpired) if !did_refresh => {
                tracing::warn!("Token expired for post {} (attempt {}). Refreshing...", post.id, attempt);
                did_refresh = true;
                let new_token = match provider.refresh_token(
                    post.refresh_token.as_deref().unwrap_or(""),
                ).await {
                    Ok(t) => {
                                        // Encrypt token before storing if encryption key is configured
                        let enc_access_token = if let Some(ref k) = token_key {
                            crate::crypto::encrypt_string(&t.access_token, k)
                                .unwrap_or_else(|_| t.access_token.clone())
                        } else {
                            t.access_token.clone()
                        };
                        queries::update_integration_token(
                            db,
                            post.integration_id,
                            &enc_access_token,
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

// ── Analytics Cache Refresh ──────────────────────────────────

const ANALYTICS_REFRESH_INTERVAL_SECS: u64 = 1800; // 30 minutes

/// Background analytics cache refresh: polls every 30 minutes,
/// iterates all users + integrations, fetches account-level analytics,
/// and upserts them into the analytics_cache.
/// Also cleans up expired cache entries each cycle.
pub async fn run_analytics_cache_refresh(
    db: PgPool,
    providers: Arc<ProviderRegistry>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    tracing::info!(
        "Analytics cache refresher started (interval: {ANALYTICS_REFRESH_INTERVAL_SECS}s)"
    );
    let mut interval = tokio::time::interval(Duration::from_secs(ANALYTICS_REFRESH_INTERVAL_SECS));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    tracing::info!("Analytics cache refresher shutting down...");
                    break;
                }
            }
            _ = interval.tick() => {
                refresh_cache_cycle(&db, &providers).await;
            }
        }
    }
}

async fn refresh_cache_cycle(db: &PgPool, providers: &ProviderRegistry) {
    // Clean up expired entries first
    if let Ok(count) = queries::delete_expired_analytics_cache(db).await {
        if count > 0 {
            tracing::info!("Cleaned up {count} expired analytics cache entries");
        }
    }

    let users = match queries::list_all_users(db).await {
        Ok(users) => users,
        Err(e) => {
            tracing::error!("Failed to list users for analytics refresh: {e}");
            return;
        }
    };

    for user in &users {
        let integrations = match queries::list_integrations(db, user.id).await {
            Ok(integrations) => integrations,
            Err(e) => {
                tracing::warn!("Failed to list integrations for user {}: {e}", user.id);
                continue;
            }
        };

        for integration in &integrations {
            if integration.refresh_needed {
                tracing::debug!(
                    "Skipping analytics refresh for user {} provider {}: refresh_needed flag set",
                    user.id, integration.provider_identifier
                );
                continue;
            }

            let provider = match providers.get(&integration.provider_identifier) {
                Some(p) => p,
                None => continue,
            };

            match provider
                .analytics(&integration.access_token, &integration.internal_id, 7)
                .await
            {
                Ok(analytics) => {
                    let data = serde_json::to_value(&analytics).unwrap_or(serde_json::Value::Null);
                    if let Err(e) = queries::upsert_analytics_cache(
                        db,
                        user.id,
                        &integration.provider_identifier,
                        None,
                        &data,
                    )
                    .await
                    {
                        tracing::warn!(
                            "Failed to cache analytics for user {} provider {}: {e}",
                            user.id,
                            integration.provider_identifier
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to fetch analytics for user {} provider {}: {e}",
                        user.id,
                        integration.provider_identifier
                    );
                }
            }
        }
    }
}
