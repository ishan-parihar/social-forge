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

        // On startup, reclaim any posts stuck in `publishing` state
        // from a previous crash. 5 minutes is the threshold — anything
        // still publishing after 5 min is almost certainly a dead
        // process, not a slow API call.
        match queries::reclaim_stuck_publishing(&db1, 300).await {
            Ok(count) if count > 0 => {
                tracing::warn!(
                    "Reclaimed {count} post(s) stuck in 'publishing' state — marked as error for manual review"
                );
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Failed to reclaim stuck publishing posts: {e}");
            }
        }

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
                    if let Err(e) = queries::mark_integration_refresh_needed(db, integration.id).await {
                        tracing::warn!("DB operation failed: {e}");
                    }
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
                    if let Err(e) = queries::mark_integration_refresh_needed(db, integration.id).await {
                        tracing::warn!("DB operation failed: {e}");
                    }
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

/// Process all posts due for publishing.
///
/// Uses a tracked `JoinSet` instead of detached `tokio::spawn` so that:
///   (a) the scheduler tick doesn't return until all spawned publishes
///       complete — prevents ticks from stacking under load;
///   (b) on shutdown, we can wait for in-flight publishes to drain
///       instead of killing them mid-API-call (which would cause
///       double-publishes on restart).
///
/// Each publish acquires a permit from the per-provider `Semaphore`
/// before calling `provider.publish()`, so 30 queued posts for the
/// same X account serialize instead of all hitting the API at once.
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

    // Tracked task set — we await all of these before returning.
    let mut join_set: tokio::task::JoinSet<()> = tokio::task::JoinSet::new();

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

        // Acquire the per-provider concurrency permit BEFORE spawning
        // so that we serialize same-platform publishes. The permit is
        // moved into the task and released when the task completes.
        let semaphore = providers.concurrency(&post.provider_identifier);
        let permit = match semaphore.as_ref() {
            Some(sem) => match sem.clone().acquire_owned().await {
                Ok(p) => Some(p),
                // Semaphore closed — shouldn't happen, but proceed without throttling.
                Err(_) => None,
            },
            None => None,
        };

        let db_clone = db.clone();
        let provider_clone = provider.clone();
        let post_clone = post.clone();
        let bcast = broadcaster.clone();
        let tk = token_key; // Copy (Option<[u8; 32]> implements Copy)
        join_set.spawn(async move {
            // `_permit` is held for the duration of the publish call,
            // then dropped automatically when the task exits.
            let _permit = permit;

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

    // Wait for all spawned publishes to complete before returning.
    // This prevents the next scheduler tick from stacking on top of
    // the current one. If a publish takes longer than one tick
    // interval, the next tick simply starts later — no unbounded
    // task growth.
    while join_set.len() > 0 {
        // Use a timeout so we don't block forever if a single publish
        // hangs (e.g. a provider API that never responds). 5 minutes
        // is generous; most publishes complete in <10s.
        match tokio::time::timeout(Duration::from_secs(300), join_set.join_next()).await {
            Ok(Some(Err(e))) => {
                tracing::error!("Publish task panicked: {e}");
            }
            Ok(Some(Ok(()))) => { /* task completed normally */ }
            Ok(None) => break, // JoinSet empty
            Err(_) => {
                tracing::warn!("Publish task timed out after 300s — leaving it detached");
                break;
            }
        }
    }

    Ok(())
}

/// Publish a single post, with token refresh and retry logic.
///
/// Retry policy:
///   - `TokenExpired`: refresh once, retry immediately
///   - `RateLimited`: exponential backoff (2^attempt × 5s) with ±25% jitter
///   - `Network`: retry with exponential backoff (was: immediate fail —
///     this was a bug where a single transient reqwest error permanently
///     marked the post as `Error`)
///   - `Auth` / `Api` / `InvalidRequest`: no retry (won't succeed)
///
/// After `MAX_RETRIES` exhausted, the post is marked `Error` and the
/// caller broadcasts `post_failed` + fires webhooks.
async fn publish_post(
    db: &PgPool,
    provider: &dyn SocialProvider,
    post: &PostWithIntegration,
    broadcaster: &Broadcaster,
    token_key: Option<[u8; 32]>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Resolve access token, potentially refreshed
    let mut access_token = resolve_token(db, provider, post, token_key).await?;

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
        let attempt_start = Utc::now();
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

                // Record successful attempt in audit trail.
                let _ = queries::record_publish_attempt(
                    db,
                    post.id,
                    attempt as i32,
                    "success",
                    None,
                    attempt_start,
                ).await;

                tracing::info!(
                    "Post {} published on {}: {}",
                    post.id,
                    post.provider_identifier,
                    result.platform_post_url.as_deref().unwrap_or("(no URL)")
                );

                let event_payload = serde_json::json!({
                    "id": post.id.to_string(),
                    "platform_post_url": result.platform_post_url,
                    "provider": post.provider_identifier,
                });

                broadcaster.send("post_published", &event_payload);

                // Fire webhooks for post.published event (best-effort,
                // non-blocking — runs in a detached task so it doesn't
                // delay the scheduler tick).
                dispatch_webhook_background(db, post.user_id, "post.published", &event_payload);

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
                                .unwrap_or_else(|e| {
                                    tracing::warn!(
                                        "Failed to encrypt refreshed token for integration {}: {e}",
                                        post.integration_id
                                    );
                                    t.access_token.clone()
                                })
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
                        // Mark the integration as needing re-auth so the
                        // user gets a UI prompt to reconnect.
                        let _ = queries::mark_integration_refresh_needed(db, post.integration_id).await;
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
                let _ = queries::mark_integration_refresh_needed(db, post.integration_id).await;
                last_error = Some("Token expired and refresh did not resolve it".to_string());
                break;
            }
            Err(ProviderError::RateLimited(ref msg)) => {
                // Exponential backoff with jitter: 2^attempt × 5s ± 25%
                // (was: linear 10/20/30s with no jitter — caused thundering
                // herd when 50 posts all backed off in lockstep).
                let base = 5u64 * (1u64 << (attempt - 1)); // 5, 10, 20s
                let jitter = (base as f64 * 0.25 * rand::random::<f64>()) as u64;
                let wait = base + jitter;
                tracing::warn!(
                    "Rate limited for post {} (attempt {}): {}. Backing off {}s",
                    post.id, attempt, msg, wait
                );
                last_error = Some(msg.clone());
                tokio::time::sleep(Duration::from_secs(wait)).await;
                continue;
            }
            Err(ProviderError::Network(ref e)) => {
                // Transient network errors are now retried with the same
                // exponential backoff as rate limits. Previously a single
                // reqwest::Error permanently marked the post as `Error`.
                let base = 5u64 * (1u64 << (attempt - 1));
                let jitter = (base as f64 * 0.25 * rand::random::<f64>()) as u64;
                let wait = base + jitter;
                let err_str = e.to_string();
                tracing::warn!(
                    "Network error for post {} (attempt {}): {}. Retrying in {}s",
                    post.id, attempt, err_str, wait
                );
                last_error = Some(err_str);
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

    // Record the final failed attempt in the audit trail.
    let _ = queries::record_publish_attempt(
        db,
        post.id,
        MAX_RETRIES as i32,
        "failed",
        last_error.as_deref(),
        Utc::now(),
    ).await;

    // Fire webhook for post.failed before returning the error.
    let fail_payload = serde_json::json!({
        "id": post.id.to_string(),
        "provider": post.provider_identifier,
        "error": last_error.clone().unwrap_or_default(),
    });
    dispatch_webhook_background(db, post.user_id, "post.failed", &fail_payload);

    Err(last_error.unwrap_or_else(|| "Max retries exceeded".to_string()).into())
}

/// Fire a webhook event in the background without blocking the caller.
///
/// We can't use `services::webhook_dispatcher::dispatch_event` directly
/// because it requires a full `&AppState` (which the scheduler doesn't
/// have — it only has `&PgPool`). Instead we inline a minimal version
/// that does the same DB query + HTTP send, spawned as a detached
/// task so the scheduler tick isn't delayed by webhook delivery.
fn dispatch_webhook_background(db: &PgPool, user_id: uuid::Uuid, event_type: &str, payload: &serde_json::Value) {
    let db = db.clone();
    let user_id = user_id;
    let event_type = event_type.to_string();
    let payload = payload.clone();
    tokio::spawn(async move {
        // Fetch active webhooks matching this event type for the user.
        let webhooks: Vec<(uuid::Uuid, String, Option<String>)> = match sqlx::query_as(
            r#"SELECT id, url, secret FROM webhooks
               WHERE user_id = $1 AND is_active = true AND $2 = ANY(event_types)"#,
        )
        .bind(user_id)
        .bind(&event_type)
        .fetch_all(&db)
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::warn!("Failed to fetch webhooks for {event_type}: {e}");
                return;
            }
        };

        for (webhook_id, url, secret) in webhooks {
            let result = crate::services::webhook_dispatcher::send_webhook(
                &url,
                secret.as_deref(),
                &event_type,
                &payload,
            ).await;

            let (status, status_code, response_body) = match &result {
                Ok((code, body)) => {
                    if *code == 200 || *code == 201 {
                        ("delivered", Some(*code as i32), Some(body.clone()))
                    } else {
                        ("failed", Some(*code as i32), Some(body.clone()))
                    }
                }
                Err(e) => ("failed", None, Some(e.clone())),
            };

            // Record delivery attempt.
            let _ = sqlx::query(
                r#"INSERT INTO webhook_deliveries
                   (webhook_id, event_type, status, status_code, response_body, attempted_at)
                   VALUES ($1, $2, $3, $4, $5, NOW())"#,
            )
            .bind(webhook_id)
            .bind(&event_type)
            .bind(status)
            .bind(status_code)
            .bind(response_body)
            .execute(&db)
            .await;

            if result.is_err() {
                tracing::warn!("Webhook delivery to {url} for {event_type} failed");
            }
        }
    });
}

/// Resolve a valid access token, refreshing if necessary.
///
/// SECURITY: when `token_key` is `Some`, the refreshed access token is
/// AES-256-GCM encrypted before being written to the DB. Previously
/// this path stored the raw token — a silent at-rest encryption
/// downgrade on every scheduler-triggered refresh.
async fn resolve_token(
    db: &PgPool,
    provider: &dyn SocialProvider,
    post: &PostWithIntegration,
    token_key: Option<[u8; 32]>,
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

        // Encrypt before storing if encryption key is configured.
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
    token_key: Option<[u8; 32]>,
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
                refresh_cache_cycle(&db, &providers, token_key).await;
            }
        }
    }
}

async fn refresh_cache_cycle(db: &PgPool, providers: &ProviderRegistry, token_key: Option<[u8; 32]>) {
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

            // Decrypt token if encryption is enabled — without this,
            // every analytics call 401s when TOKEN_ENCRYPTION_KEY is set.
            let tok = crate::crypto::maybe_decrypt_token(&integration.access_token, token_key.as_ref());

            match provider
                .analytics(&tok, &integration.internal_id, 7)
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
