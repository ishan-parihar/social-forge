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
                    // v22 Phase 1 (D.5): reclaim stuck publishing posts
                    // every tick (30s) instead of only on startup. This
                    // catches publishes that hung mid-API-call without
                    // waiting for a process restart. The UPDATE is cheap
                    // (single statement, indexed by state).
                    if let Err(e) = queries::reclaim_stuck_publishing(&db1, 300).await {
                        tracing::warn!("reclaim_stuck_publishing failed: {e}");
                    }
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

    // ── Circuit breaker check ──────────────────────────────────
    // For each provider with an open circuit, push all its claimed
    // posts back to 'queued' so they get retried after the cooldown.
    // This prevents burning through all queued posts when a platform
    // API is down (e.g. X 5xx for 10 minutes).
    let mut skipped = 0u64;
    let mut posts_to_publish: Vec<PostWithIntegration> = Vec::new();
    for post in posts {
        if let Some(cb) = providers.circuit_breaker(&post.provider_identifier) {
            if !cb.allow_request() {
                // Circuit is open — push this post back to queued
                let _ = sqlx::query(
                    "UPDATE posts SET state = 'queued', updated_at = NOW() WHERE id = $1",
                )
                .bind(post.id)
                .execute(db)
                .await;
                skipped += 1;
                continue;
            }
        }
        posts_to_publish.push(post);
    }

    if skipped > 0 {
        tracing::warn!(
            "Circuit breaker: skipped {} post(s) (pushed back to queued)",
            skipped
        );
    }

    if posts_to_publish.is_empty() {
        return Ok(());
    }

    // Tracked task set — we await all of these before returning.
    let mut join_set: tokio::task::JoinSet<()> = tokio::task::JoinSet::new();

    // v22 Phase 2 (BUG #1): track post IDs that are still in-flight so
    // that on scheduler timeout we can abort the JoinSet AND mark the
    // remaining posts as `error` (instead of leaving them detached in
    // `publishing` state until the next process restart).
    let mut inflight_post_ids: Vec<uuid::Uuid> = Vec::with_capacity(posts_to_publish.len());

    for post in &posts_to_publish {
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

        // v22 Phase 2 (BUG #2): clone the Semaphore Arc and move it
        // into the spawned task instead of acquiring the permit
        // serially here. Previously `acquire_owned().await` happened
        // in the main scheduler task BEFORE `join_set.spawn`, so if
        // one provider's semaphore was exhausted (limit=1, in-flight
        // publish slow), the scheduler BLOCKED on that single acquire
        // and never spawned the other providers' publishes. For 30
        // queued posts across 3 providers, if X's permit was held for
        // 60s, the Reddit/LinkedIn posts waited 60s before they even
        // started. Now each task acquires its own permit inside the
        // spawned future, so all providers make progress in parallel.
        let semaphore = providers.concurrency(&post.provider_identifier).cloned();

        let db_clone = db.clone();
        let provider_clone = provider.clone();
        let post_clone = post.clone();
        let bcast = broadcaster.clone();
        let tk = token_key; // Copy (Option<[u8; 32]> implements Copy)
        let cb = providers.circuit_breaker(&post.provider_identifier);
        inflight_post_ids.push(post.id);
        join_set.spawn(async move {
            // Acquire the per-provider concurrency permit INSIDE the
            // spawned task so the scheduler doesn't block on a slow
            // provider. The permit is released automatically when the
            // task exits (success or failure).
            let _permit = match semaphore.as_ref() {
                Some(sem) => match sem.acquire_owned().await {
                    Ok(p) => Some(p),
                    // Semaphore closed — shouldn't happen, but proceed
                    // without throttling rather than skipping the post.
                    Err(_) => None,
                },
                None => None,
            };

            match publish_post(&db_clone, provider_clone.as_ref(), &post_clone, &bcast, tk).await {
                Ok(()) => {
                    // Publish succeeded — record success in circuit breaker
                    if let Some(ref breaker) = cb {
                        breaker.record_success();
                    }
                }
                Err(e) => {
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
                    // Publish failed — record failure in circuit breaker.
                    // After N consecutive failures, the circuit opens and
                    // subsequent posts for this provider are skipped.
                    if let Some(ref breaker) = cb {
                        breaker.record_failure();
                    }
                }
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
                // v22 Phase 2 (BUG #1): on timeout, ABORT all remaining
                // tasks (don't leave them detached) and mark their posts
                // as `error`. Previously the code just `break`ed, leaving
                // the still-running tasks detached — they'd eventually
                // write `published`/`error` from a context the operator
                // had no visibility into, and the posts sat in
                // `publishing` until the next process restart.
                tracing::warn!(
                    "Scheduler drain timed out after 300s — aborting {} remaining publish task(s)",
                    join_set.len()
                );
                join_set.abort_all();
                // Mark the posts we know are still in-flight as error.
                // (Tasks that completed normally already wrote their own
                // state; the abort_all above stops the rest.)
                for pid in &inflight_post_ids {
                    let _ = mark_post_error(
                        db,
                        *pid,
                        "Publish timed out after 300s — aborted by scheduler",
                    )
                    .await;
                    broadcaster.send(
                        "post_failed",
                        &serde_json::json!({
                            "id": pid.to_string(),
                            "error": "Publish timed out after 300s — aborted by scheduler",
                        }),
                    );
                }
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

    // ── Thread linking ──────────────────────────────────────
    // If this post is part of a thread (has a group_id and sequence > 1),
    // look up the PREVIOUS post in the same group and use its
    // platform_post_id as in_reply_to. This makes the scheduler publish
    // real linked threads on X/Bluesky/Mastodon/Threads instead of
    // standalone posts.
    let in_reply_to: Option<String> = if let Some(ref group_id) = post.group_id.as_ref() {
        let seq = post.sequence;
        if seq > 1 {
            // Find the previous post in this thread (sequence - 1)
            // that was successfully published and has a platform_post_id.
            match sqlx::query_scalar::<_, Option<String>>(
                r#"SELECT platform_post_id
                   FROM posts
                   WHERE group_id = $1
                     AND sequence = $2
                     AND state = 'published'
                     AND platform_post_id IS NOT NULL
                   LIMIT 1"#,
            )
            .bind(group_id)
            .bind(seq - 1)
            .fetch_optional(db)
            .await
            {
                Ok(Some(Some(pid))) => {
                    tracing::info!(
                        "Thread link: post {} (seq {}) replying to platform_post_id {}",
                        post.id, seq, pid
                    );
                    Some(pid)
                }
                Ok(Some(None)) | Ok(None) => {
                    tracing::warn!(
                        "Thread link: post {} (seq {}) has no published predecessor in group {} — publishing as standalone",
                        post.id, seq, group_id
                    );
                    None
                }
                Err(e) => {
                    tracing::warn!(
                        "Thread link: failed to look up predecessor for post {} (seq {}): {e} — publishing as standalone",
                        post.id, seq
                    );
                    None
                }
            }
        } else {
            None // First post in thread — no predecessor to reply to
        }
    } else {
        None // Not part of a thread
    };

    // ── Short-link processing ────────────────────────────────
    // If STRIP_LINKS_FROM_X is enabled and this is an X post, remove
    // all URLs from the content (X downranks posts with external links).
    // If DUB_CO_API_KEY is configured, shorten URLs in the content.
    let processed_content = {
        let mut c = post.content.clone();
        let provider_id = &post.provider_identifier;

        // Strip links from X if configured
        let strip_x = std::env::var("STRIP_LINKS_FROM_X")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        if provider_id == "x" && strip_x {
            c = crate::services::short_link::strip_links(&c);
        }

        // Shorten URLs if Dub.co is configured
        if let Ok(api_key) = std::env::var("DUB_CO_API_KEY") {
            if !api_key.is_empty() && crate::services::short_link::has_urls(&c) {
                let workspace = std::env::var("DUB_CO_WORKSPACE").ok().filter(|s| !s.is_empty());
                c = crate::services::short_link::shorten_urls(&c, &api_key, workspace.as_deref()).await;
            }
        }

        c
    };

    let content = PostContent {
        content: processed_content,
        media: resolved_media,
        settings: post.settings.clone(),
        in_reply_to,
        // Phase v22: pass the post's stable idempotency key so providers
        // can deduplicate retry attempts (prevents double-publish after
        // crash-recovery). The key is generated when the post is created
        // and stays stable across retries; only a re-publish (action=
        // 'schedule' on reschedule) generates a new key.
        idempotency_key: Some(post.idempotency_key.to_string()),
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
                // Phase v22: robustness fix for the publish-orphan problem.
                //
                // Previously, if update_post_state failed here (DB connection
                // drop, pool exhaustion, etc.), the `?` propagated the error
                // and the post stayed in 'publishing' state — even though it
                // was already live on the platform. On next startup,
                // reclaim_stuck_publishing would mark it 'error', and the
                // user could retry → double-publish (now mitigated by
                // idempotency keys, but still messy: the post shows as
                // 'error' even though it's actually published).
                //
                // Now: the publish SUCCEEDED on the platform, so the post IS
                // published. We retry the DB update a few times. If all DB
                // retries fail, we log a CRITICAL warning (manual intervention
                // needed) but return Ok — the source of truth at this point
                // is the platform, not our DB. The reclaim_stuck_publishing
                // path won't touch it because we set state='published'
                // (eventually consistent).
                let mut db_updated = false;
                for db_attempt in 1..=3 {
                    match queries::update_post_state(
                        db,
                        post.id,
                        PostState::Published,
                        Some(&result.platform_post_id),
                        result.platform_post_url.as_deref(),
                        None,
                    ).await {
                        Ok(_) => { db_updated = true; break; }
                        Err(e) => {
                            tracing::warn!(
                                "Post {} published on {} but DB update failed (attempt {}/3): {}. Retrying in 1s...",
                                post.id, post.provider_identifier, db_attempt, e
                            );
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        }
                    }
                }
                if !db_updated {
                    // Critical: the post is live on the platform but our DB
                    // still shows 'publishing'. Log loudly for manual triage.
                    // We do NOT return Err — that would trigger reclaim→error
                    // and a potential double-publish on retry. The idempotency
                    // key would protect against the double-publish, but the
                    // 'error' state would mislead the user.
                    tracing::error!(
                        "CRITICAL: Post {} published on {} (platform_post_id={}) but DB update failed after 3 attempts. \
                         The post IS live on the platform. Manual DB reconciliation needed: \
                         UPDATE posts SET state='published', platform_post_id='{}', platform_post_url={} WHERE id='{}';",
                        post.id,
                        post.provider_identifier,
                        result.platform_post_id,
                        result.platform_post_id,
                        result.platform_post_url.as_deref().map(|u| format!("'{}'", u)).unwrap_or_else(|| "NULL".to_string()),
                        post.id,
                    );
                    // Still record the attempt + broadcast so the UI updates.
                    // The reclaim path will eventually mark it 'error', but
                    // the operator has been warned via the CRITICAL log.
                }

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

                // Update posting streak — the solo founder's daily
                // posting motivation. If this is the first post today,
                // increment streak_days. The daily reset cron (below)
                // handles the "no post in 24h → reset" case.
                update_streak_on_publish(db, post.user_id).await;

                // Publish first_comment if present
                if let Some(ref comment_text) = post.first_comment {
                    if !comment_text.is_empty() {
                        let comment_content = PostContent {
                            content: comment_text.clone(),
                            media: vec![],
                            settings: serde_json::json!({}),
                        in_reply_to: None,
                        idempotency_key: None,
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

// ── Posting Streak ───────────────────────────────────────────

/// Update the user's posting streak when a post is published.
///
/// - If `streak_since` is NULL (never posted) → set to NOW, streak_days = 1
/// - If last post was yesterday (streak_since < NOW - 24h but < NOW - 0h) →
///   streak_days += 1, streak_since = NOW
/// - If last post was today (streak_since > NOW - 24h) → no change (already counted)
/// - The daily reset cron handles the "no post in 24h → reset" case
async fn update_streak_on_publish(db: &PgPool, user_id: uuid::Uuid) {
    let now = Utc::now();
    let row = sqlx::query("SELECT streak_since, streak_days FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(db)
        .await;

    match row {
        Ok(Some(r)) => {
            use sqlx::Row;
            let streak_since: Option<chrono::DateTime<chrono::Utc>> = r.try_get("streak_since").ok();
            let streak_days: i32 = r.try_get("streak_days").unwrap_or(0);

            if streak_since.is_none() {
                // First ever post
                let _ = sqlx::query("UPDATE users SET streak_since = $1, streak_days = 1 WHERE id = $2")
                    .bind(now)
                    .bind(user_id)
                    .execute(db)
                    .await;
                tracing::info!("Streak: first post for user {user_id}, streak = 1");
            } else if let Some(since) = streak_since {
                let elapsed = now - since;
                if elapsed.num_hours() >= 24 {
                    let new_streak = streak_days + 1;
                    let _ = sqlx::query("UPDATE users SET streak_since = $1, streak_days = $2 WHERE id = $3")
                        .bind(now)
                        .bind(new_streak)
                        .bind(user_id)
                        .execute(db)
                        .await;
                    tracing::info!("Streak: user {user_id} now at {new_streak} days");
                }
            }
        }
        Ok(None) => {}
        Err(e) => {
            tracing::warn!("Streak: failed to query user {user_id}: {e}");
        }
    }
}

/// Daily streak reset: checks all users. If streak_since is more than
/// 48 hours ago (missed a full day), reset streak_days to 0.
/// Runs every hour (lightweight query).
pub fn start_streak_reset(db: PgPool, mut shutdown: tokio::sync::watch::Receiver<bool>) {
    tokio::spawn(async move {
        let interval = Duration::from_secs(3600); // every hour
        tracing::info!("Streak reset checker started (interval: 1 hour)");
        loop {
            tokio::select! {
                _ = tokio::time::sleep(interval) => {
                    if let Err(e) = reset_expired_streaks(&db).await {
                        tracing::error!("Streak reset error: {e}");
                    }
                }
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        tracing::info!("Streak reset checker shutting down");
                        break;
                    }
                }
            }
        }
    });
}

async fn reset_expired_streaks(db: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Reset streaks where no post has been published in the last 48 hours.
    // (48h gives a grace period so a streak isn't lost if you post at 11pm
    // one day and 1am the next — those are ~2h apart but count as 2 days.)
    let result = sqlx::query(
        "UPDATE users
         SET streak_days = 0, streak_since = NULL
         WHERE streak_since IS NOT NULL
           AND streak_since < NOW() - INTERVAL '48 hours'",
    )
    .execute(db)
    .await?;

    if result.rows_affected() > 0 {
        tracing::info!("Streak: reset {} user(s) with expired streaks", result.rows_affected());
    }
    Ok(())
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
