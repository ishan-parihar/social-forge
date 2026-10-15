// ─── Publishing Scheduler ─────────────────────────────────────
// In-process scheduler that polls for due posts every 30 seconds.
// Handles token refresh, retry with backoff, and event broadcasting.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;

use crate::db::models::{PostState, PostWithIntegration};
use crate::db::SqlitePool;
use crate::poll::{spawn_poll, FirstTick};
use crate::realtime::Broadcaster;
use crate::social::registry::ProviderRegistry;
use crate::social::ProviderError;
use crate::social::{PostContent, SocialProvider};
use uuid::Uuid;

use super::db::queries;
/// How often to poll for due posts
const POLL_INTERVAL_SECS: u64 = 30;

/// Maximum publish retries per post (pub(crate): PostService's tests
/// pin the unified engine's audit-row contract)
pub(crate) const MAX_RETRIES: u32 = 3;

/// Max posts processed per scheduler tick
const DUE_POSTS_LIMIT: i64 = 50;

/// Gap between polls of a platform that accepted a post but has not
/// published it yet (IG transcoding a reel, an async Reddit submit, X
/// encoding a video).
const PENDING_POLL_INTERVAL: Duration = crate::social::DEFAULT_POLL_INTERVAL;

/// How far ahead to consider a token "expired" and refresh preemptively
const TOKEN_REFRESH_BUFFER_SECS: i64 = 300; // 5 minutes

/// Grace window past `scheduled_at` before a post counts as a posting
/// gap. 3 hours: long enough to absorb a platform outage or an open
/// circuit breaker, short enough that the operator finds out the same day.
const POSTING_GAP_GRACE_SECS: i64 = 3 * 60 * 60;

/// Max posts reported per posting-gap check — bounds the alert burst
/// when a whole backlog is late.
const POSTING_GAP_LIMIT: i64 = 20;

/// Start the scheduler background task.
/// Pass a watch::Receiver that resolves to `true` to trigger graceful shutdown.
pub fn start_scheduler(
    db: SqlitePool,
    providers: Arc<ProviderRegistry>,
    broadcaster: Broadcaster,
    token_key: Option<[u8; 32]>,
    shutdown: tokio::sync::watch::Receiver<bool>,
) {
    // Main post publishing scheduler
    let db1 = db.clone();
    let providers1 = providers.clone();
    // The boot-time reclaim below runs once, before the loop's first
    // iteration. Gating it on a captured flag keeps it in the same place
    // in the sequence — before the per-tick reclaim, which also runs on
    // the first tick — without spawning a second task for it.
    let mut boot = true;
    spawn_poll(
        Duration::from_secs(POLL_INTERVAL_SECS),
        FirstTick::Now,
        shutdown.clone(),
        "Scheduler shutting down...",
        move || {
            let boot = std::mem::take(&mut boot);
            let db = db1.clone();
            let providers = providers1.clone();
            let broadcaster = broadcaster.clone();
            async move {
                if boot {
                    tracing::info!("Scheduler started (poll interval: {POLL_INTERVAL_SECS}s)");

                    // On startup, reclaim any posts stuck in `publishing` state
                    // from a previous crash. 5 minutes is the threshold — anything
                    // still publishing after 5 min is almost certainly a dead
                    // process, not a slow API call.
                    match queries::reclaim_stuck_publishing(&db, 300).await {
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
                }

                // v22 Phase 1 (D.5): reclaim stuck publishing posts
                // every tick (30s) instead of only on startup. This
                // catches publishes that hung mid-API-call without
                // waiting for a process restart. The UPDATE is cheap
                // (single statement, indexed by state).
                if let Err(e) = queries::reclaim_stuck_publishing(&db, 300).await {
                    tracing::warn!("reclaim_stuck_publishing failed: {e}");
                }
                // Posting-gap detection: alert on posts the scheduler
                // never got out the door. Runs on the existing tick —
                // no separate task — and the query is bounded by
                // POSTING_GAP_LIMIT and deduped against unread alerts,
                // so a 30s cadence is cheap.
                if let Err(e) = check_posting_gaps(&db, &broadcaster).await {
                    tracing::warn!("check_posting_gaps failed: {e}");
                }
                if let Err(e) = process_due_posts(&db, &providers, &broadcaster, token_key).await {
                    tracing::error!("Scheduler tick error: {e}");
                }
                // v23-2: drain the publish_outbox. This retries any
                // post-state updates that failed after a successful
                // platform publish (e.g. DB connection blip). The
                // drain is idempotent — completed rows are skipped.
                if let Err(e) = drain_publish_outbox(&db).await {
                    tracing::warn!("publish_outbox drain failed: {e}");
                }
            }
        },
    );

    // Background cleanup task for expired oauth_states (runs every 10 minutes)
    let db_cleanup = db.clone();
    spawn_poll(
        Duration::from_secs(600),
        FirstTick::Now,
        shutdown.clone(),
        "OAuth cleanup task shutting down...",
        move || {
            let db = db_cleanup.clone();
            async move {
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
        },
    );

    // Proactive token refresh task (runs every 6 hours)
    // Refreshes tokens expiring within 24h to prevent silent expiration
    let db2 = db.clone();
    let providers2 = providers.clone();
    spawn_poll(
        Duration::from_secs(21600), // 6 hours
        FirstTick::Now,
        shutdown.clone(),
        "Proactive token refresh shutting down...",
        move || {
            let db = db2.clone();
            let providers = providers2.clone();
            async move {
                if let Err(e) = proactive_token_refresh(&db, &providers, token_key).await {
                    tracing::error!("Proactive token refresh error: {e}");
                }
            }
        },
    );

    // v23: events_log cleanup task (runs every 6 hours).
    // Trims events_log rows older than 7 days to prevent unbounded growth.
    // The dashboard only queries the last 10-50 events, so 7 days is
    // generous headroom.
    let db_events = db.clone();
    spawn_poll(
        Duration::from_secs(21600), // 6 hours
        FirstTick::Now,
        shutdown.clone(),
        "Events log cleanup shutting down...",
        move || {
            let db = db_events.clone();
            async move {
                let retention_days: i64 = std::env::var("EVENTS_LOG_RETENTION_DAYS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(7);
                // Timestamps are INTEGER unix-epoch seconds, so the cutoff is
                // computed in Rust and bound as one integer. (The old
                // `make_interval(days => ?::int)` was a Postgres-only function
                // whose bigint->int cast was itself load-bearing.)
                let cutoff = Utc::now().timestamp() - retention_days * 86_400;
                match sqlx::query("DELETE FROM events_log WHERE created_at < ?")
                    .bind(cutoff)
                    .execute(&db)
                    .await
                {
                    Ok(res) => {
                        let n = res.rows_affected();
                        if n > 0 {
                            tracing::debug!(
                                "Cleaned up {n} events_log rows older than {retention_days} days"
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!("events_log cleanup failed: {e}");
                    }
                }
            }
        },
    );
}

/// Proactive token refresh: refreshes tokens expiring within 24h for providers that need cron refresh
async fn proactive_token_refresh(
    db: &SqlitePool,
    providers: &ProviderRegistry,
    token_key: Option<[u8; 32]>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    const PROACTIVE_REFRESH_WINDOW_HOURS: i64 = 24;

    // Get all providers that need cron refresh
    let providers_needing_refresh: Vec<_> = providers
        .list()
        .into_iter()
        .filter(|id| {
            providers
                .get(id)
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
                    if let Err(e) =
                        queries::mark_integration_refresh_needed(db, integration.id).await
                    {
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
                    // B8: update_integration_token encrypts internally.
                    let expires_at = new_token
                        .expires_in
                        .map(|e| Utc::now() + chrono::Duration::seconds(e as i64));

                    queries::update_integration_token(
                        db,
                        integration.id,
                        &new_token.access_token,
                        new_token.refresh_token.as_deref(),
                        expires_at,
                        token_key,
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
                    if let Err(e) =
                        queries::mark_integration_refresh_needed(db, integration.id).await
                    {
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
    db: &SqlitePool,
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
                    "UPDATE posts SET state = 'queued', updated_at = unixepoch() WHERE id = ?",
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
    let mut join_set: tokio::task::JoinSet<uuid::Uuid> = tokio::task::JoinSet::new();

    // v22 Phase 2 (BUG #1): track post IDs that are still in-flight so
    // that on scheduler timeout we can abort the JoinSet AND mark the
    // remaining posts as `error` (instead of leaving them detached in
    // `publishing` state until the next process restart).
    // B1: post ids whose tasks are still in flight. Each spawned task
    // returns its own post id when it completes, so the drain loop below
    // can remove finished posts BEFORE the timeout arm — a timeout then
    // only marks the posts that are genuinely still running as `error`
    // (instead of overwriting already-published ones).
    let mut inflight_post_ids: std::collections::HashSet<uuid::Uuid> =
        std::collections::HashSet::with_capacity(posts_to_publish.len());

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
        let semaphore = providers.concurrency(&post.provider_identifier).clone();

        let db_clone = db.clone();
        let provider_clone = provider.clone();
        let post_clone = post.clone();
        let bcast = broadcaster.clone();
        let tk = token_key; // Copy (Option<[u8; 32]> implements Copy)
        let cb = providers.circuit_breaker(&post.provider_identifier);
        inflight_post_ids.insert(post.id);
        join_set.spawn(async move {
            // Acquire the per-provider concurrency permit INSIDE the
            // spawned task so the scheduler doesn't block on a slow
            // provider. The permit is released automatically when the
            // task exits (success or failure).
            // Semaphore closed — shouldn't happen, but proceed
            // without throttling rather than skipping the post.
            let _permit = match semaphore.as_ref() {
                Some(sem) => sem.clone().acquire_owned().await.ok(),
                None => None,
            };

            match publish_post(
                &db_clone,
                provider_clone.as_ref(),
                &post_clone,
                &bcast,
                tk,
                PublishOpts::default(),
            )
            .await
            {
                Ok(_) => {
                    // Publish succeeded (or the part was left queued for a
                    // later tick) — record success in circuit breaker
                    if let Some(ref breaker) = cb {
                        breaker.record_success();
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to publish post {}: {e}", post_clone.id);
                    let err_str = e.to_string();
                    mark_post_error(&db_clone, post_clone.id, &err_str).await;
                    let fail_payload = serde_json::json!({
                        "id": post_clone.id.to_string(),
                        "error": err_str,
                    });
                    bcast.send("post_failed", &fail_payload);
                    // v23: persist to events_log for the dashboard.
                    let _ = bcast
                        .send_and_log(&db_clone, post_clone.user_id, "post_failed", &fail_payload)
                        .await;
                    // Publish failed — record failure in circuit breaker.
                    // After N consecutive failures, the circuit opens and
                    // subsequent posts for this provider are skipped.
                    if let Some(ref breaker) = cb {
                        breaker.record_failure();
                    }
                }
            }
            post_clone.id
        });
    }

    // Wait for all spawned publishes to complete before returning.
    // This prevents the next scheduler tick from stacking on top of
    // the current one. If a publish takes longer than one tick
    // interval, the next tick simply starts later — no unbounded
    // task growth.
    while !join_set.is_empty() {
        // Use a timeout so we don't block forever if a single publish
        // hangs (e.g. a provider API that never responds). 5 minutes
        // is generous; most publishes complete in <10s.
        match tokio::time::timeout(Duration::from_secs(300), join_set.join_next()).await {
            Ok(Some(Err(e))) => {
                // Task panicked mid-publish: its post is (almost always)
                // still `publishing`. Remove it from the in-flight set and
                // mark it errored now — the gated write is a no-op if the
                // task somehow wrote a terminal state before panicking.
                tracing::error!("Publish task panicked: {e}");
            }
            Ok(Some(Ok(pid))) => {
                // B1: this post finished on its own — it wrote its own
                // terminal state. NEVER let the timeout arm touch it.
                inflight_post_ids.remove(&pid);
            }
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
                // B1: only the posts STILL in flight (their tasks never
                // returned) get marked `error` — posts that finished earlier
                // in the drain already wrote `published`/`error` and were
                // removed from this set above. mark_post_error's
                // `AND state='publishing'` gate is the second line of
                // defense against racing a just-completed task.
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

/// v23-2: Drain the publish_outbox.
///
/// After a successful platform publish, the scheduler writes the result
/// to BOTH `posts` (source of truth) and `publish_outbox` (durability
/// log). If the `posts` write fails (e.g. DB connection blip), the
/// outbox row remains `completed_at IS NULL` and this drain loop
/// retries it.
///
/// The drain is idempotent:
/// - Rows with `completed_at IS NULL` and `attempts < 5` are retried.
/// - Each retry attempts the `posts` UPDATE again.
/// - On success, `completed_at` is set.
/// - On failure, `attempts` is incremented and `next_attempt_at` is
///   set to now + 30s (exponential backoff could be added later).
/// - Rows with `attempts >= 5` are abandoned (logged as CRITICAL).
///
/// This closes the "publish succeeded but DB write failed" publish-
/// orphan gap identified in the v22 audit (Part D.2).
async fn drain_publish_outbox(db: &SqlitePool) -> Result<(), sqlx::Error> {
    // Claim pending rows. Postgres used `FOR UPDATE SKIP LOCKED`; sqlite has
    // no row locks, so the claim is a compare-and-set on the `posts` UPDATE
    // itself: every drain writes `WHERE id = ? AND state != 'published'`, so
    // whichever instance lands first wins and the loser matches 0 rows and
    // leaves the row for the next tick. `attempts` is bumped only by the
    // winner's failure path, so a contended row is never double-counted.
    let pending: Vec<PublishOutboxRow> = sqlx::query_as(
        r#"SELECT id, post_id, platform_post_id, platform_post_url,
                  published_at, error_message
           FROM publish_outbox
           WHERE completed_at IS NULL
             AND next_attempt_at <= unixepoch()
             AND attempts < 5
           ORDER BY created_at ASC
           LIMIT 50"#,
    )
    .fetch_all(db)
    .await?;

    if pending.is_empty() {
        return Ok(());
    }

    for row in pending {
        // If the row has a platform_post_id, the publish succeeded —
        // try to apply the state to posts.
        if let Some(ref platform_post_id) = row.platform_post_id {
            let result = sqlx::query(
                r#"UPDATE posts SET
                     state = 'published',
                     platform_post_id = ?,
                     platform_post_url = ?,
                     published_at = ?,
                     error_message = NULL,
                     updated_at = unixepoch()
                   WHERE id = ? AND state != 'published'"#,
            )
            .bind(row.post_id)
            .bind(platform_post_id)
            .bind(&row.platform_post_url)
            .bind(row.published_at)
            .execute(db)
            .await;

            match result {
                Ok(_) => {
                    // Success — mark the outbox row as completed.
                    let _ = sqlx::query(
                        "UPDATE publish_outbox SET completed_at = unixepoch() WHERE id = ?",
                    )
                    .bind(row.id)
                    .execute(db)
                    .await;
                    tracing::info!(
                        "Outbox drain: applied publish result for post {} (platform_post_id={})",
                        row.post_id,
                        platform_post_id
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "Outbox drain: failed to apply publish result for post {}: {e}",
                        row.post_id
                    );
                    // Increment attempts + backoff.
                    let _ = sqlx::query(
                        r#"UPDATE publish_outbox
                           SET attempts = attempts + 1,
                               next_attempt_at = unixepoch() + 30
                           WHERE id = ?"#,
                    )
                    .bind(row.id)
                    .execute(db)
                    .await;
                }
            }
        } else if let Some(ref err) = row.error_message {
            // The publish failed — apply the error state to posts.
            let result = sqlx::query(
                r#"UPDATE posts SET
                     state = 'error',
                     error_message = ?,
                     updated_at = unixepoch()
                   WHERE id = ? AND state NOT IN ('error', 'published')"#,
            )
            .bind(row.post_id)
            .bind(err)
            .execute(db)
            .await;

            match result {
                Ok(_) => {
                    let _ = sqlx::query(
                        "UPDATE publish_outbox SET completed_at = unixepoch() WHERE id = ?",
                    )
                    .bind(row.id)
                    .execute(db)
                    .await;
                }
                Err(e) => {
                    tracing::warn!(
                        "Outbox drain: failed to apply error state for post {}: {e}",
                        row.post_id
                    );
                    let _ = sqlx::query(
                        r#"UPDATE publish_outbox
                           SET attempts = attempts + 1,
                               next_attempt_at = unixepoch() + 30
                           WHERE id = ?"#,
                    )
                    .bind(row.id)
                    .execute(db)
                    .await;
                }
            }
        }
    }

    Ok(())
}

// v23-2: row type for the outbox drain query.
#[derive(Debug, sqlx::FromRow)]
struct PublishOutboxRow {
    id: Uuid,
    post_id: Uuid,
    platform_post_id: Option<String>,
    platform_post_url: Option<String>,
    published_at: Option<chrono::DateTime<chrono::Utc>>,
    error_message: Option<String>,
}

/// v23-2: Write a publish result to the outbox.
/// Called by the scheduler after a successful publish (or after a
/// final failure). The drain loop later applies the result to `posts`.
async fn write_to_outbox(
    db: &SqlitePool,
    post_id: Uuid,
    idempotency_key: Uuid,
    platform_post_id: Option<&str>,
    platform_post_url: Option<&str>,
    error_message: Option<&str>,
) {
    let published_at = if platform_post_id.is_some() {
        Some(chrono::Utc::now())
    } else {
        None
    };
    // `publish_outbox.id` is TEXT with no DB-side default — SQLite has no
    // gen_random_uuid(), so Rust generates the value.
    let _ = sqlx::query(
        r#"INSERT INTO publish_outbox
           (id, post_id, idempotency_key, platform_post_id, platform_post_url, published_at, error_message)
           VALUES (?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(Uuid::new_v4())
    .bind(post_id)
    .bind(idempotency_key)
    .bind(platform_post_id)
    .bind(platform_post_url)
    .bind(published_at)
    .bind(error_message)
    .execute(db)
    .await
    .map_err(|e| {
        tracing::warn!("Failed to write to publish_outbox for post {post_id}: {e}");
        e
    });
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
/// Caller-adjustable behavior for [`publish_post`] (audit §4.1: one
/// pipeline, two callers).
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PublishOpts {
    /// TRUE for user-initiated publishes (REST publish-now, MCP,
    /// `posts publish`). A manual publish REFUSES when the thread
    /// predecessor is not published yet instead of silently
    /// requeueing, and its errors keep the row's incoming state
    /// (the scheduler's caller claims `publishing` and owns the
    /// terminal `error` write).
    pub manual: bool,
}

/// What [`publish_post`] did.
#[derive(Debug, Clone)]
pub(crate) enum PublishOutcome {
    Published {
        platform_post_url: Option<String>,
    },
    /// Nothing was attempted (thread predecessor not published yet —
    /// the part was left queued for a later tick).
    Skipped,
}

/// Publish a single post end to end: token resolve/refresh, thread
/// linking, validation, retry loop with post_pending/finalize lifecycle,
/// terminal-state and empty-id guards, outbox durability, first comment,
/// attempt audit, events and webhooks.
///
/// The ONE publish pipeline (audit §4.1): the scheduler and
/// PostService::publish (REST publish-now, MCP, CLI) both call it, so a
/// fix lands once. Error-state policy stays with the caller: the
/// scheduler's caller marks `publishing → error`; the manual path keeps
/// the row's incoming state and writes `error_message`.
///
/// caller broadcasts `post_failed` + fires webhooks.
pub(crate) async fn publish_post(
    db: &SqlitePool,
    provider: &dyn SocialProvider,
    post: &PostWithIntegration,
    broadcaster: &Broadcaster,
    token_key: Option<[u8; 32]>,
    opts: PublishOpts,
) -> Result<PublishOutcome, Box<dyn std::error::Error + Send + Sync>> {
    // Resolve access token, potentially refreshed
    let mut access_token = resolve_token(db, provider, post, token_key).await?;

    // Build post content - resolve media URLs to absolute paths
    let media: Vec<crate::social::MediaAttachment> =
        serde_json::from_value(post.media.clone()).unwrap_or_default();
    let app_url = std::env::var("APP_URL").unwrap_or_else(|_| "https://localhost:6543".into());
    let resolved_media: Vec<crate::social::MediaAttachment> = media
        .into_iter()
        .map(|m| {
            // Resolve relative media URLs to absolute URLs for providers that need them
            // (Instagram, Facebook, Threads, Reddit require absolute URLs)
            provider.resolve_media_url(&m, &app_url)
        })
        .collect();

    // ── Thread linking ──────────────────────────────────────
    // If this post is part of a thread (has a group_id and sequence > 1),
    // look up the PREVIOUS post in the same group and use its
    // platform_post_id as in_reply_to. This makes the scheduler publish
    // real linked threads on X/Bluesky/Mastodon/Threads instead of
    // standalone posts.
    let in_reply_to: Option<String> = if let Some(group_id) = post.group_id {
        let seq = post.sequence;
        if seq > 1 {
            // B3: scoped to THIS integration (group_id is shared across
            // integrations in a multi-platform thread).
            match queries::find_thread_predecessor(db, group_id, post.integration_id, seq - 1).await
            {
                Ok(Some(pid)) => {
                    tracing::info!(
                        "Thread link: post {} (seq {}) replying to platform_post_id {}",
                        post.id,
                        seq,
                        pid
                    );
                    Some(pid)
                }
                Ok(None) => {
                    // B3: predecessor not published yet. Scheduler: leave
                    // this part QUEUED for a later tick (a standalone
                    // reply-less part breaks the thread). Manual: refuse
                    // with an actionable message — the caller shows it.
                    if opts.manual {
                        return Err(format!(
                            "Thread predecessor (part {}) is not published yet. Publish part {} first — or wait for the scheduler, which links parts automatically.",
                            seq - 1,
                            seq - 1
                        )
                        .into());
                    }
                    tracing::info!(
                        "Thread link: post {} (seq {}) predecessor not published yet — leaving queued",
                        post.id, seq
                    );
                    if let Err(e) =
                        queries::update_post_state(db, post.id, PostState::Queued, None, None, None)
                            .await
                    {
                        tracing::warn!(
                            "Failed to requeue unpublished thread part {}: {e} — skipping this tick",
                            post.id
                        );
                    }
                    return Ok(PublishOutcome::Skipped);
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
                let workspace = std::env::var("DUB_CO_WORKSPACE")
                    .ok()
                    .filter(|s| !s.is_empty());
                c = crate::services::short_link::shorten_urls(&c, &api_key, workspace.as_deref())
                    .await;
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
        // v24-5: per-row delay for thread parts. If this post is part of
        // a thread (sequence > 1) and has a delay_minutes set in its
        // settings, the scheduler sleeps for that duration before
        // publishing. This allows spaced-out thread publishing (e.g.
        // "part 1, wait 30min, part 2"). The delay is stored in the
        // post's settings JSONB under 'delay_minutes'.
        delay_minutes: post
            .settings
            .get("delay_minutes")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32),
    };

    // B2: the in-tick delay sleep is GONE. `create_thread_posts` already
    // offsets each part's `scheduled_at` by `delay_minutes` (db/queries.rs),
    // so the post only becomes due when the delay has elapsed. The old
    // sleep applied the delay a SECOND time — a 30-minute thread part
    // slept 30 minutes inside the tick, blocking every other due post on
    // the JoinSet until the 300s drain timeout fired (which, pre-B1,
    // marked the whole tick `error`).

    // Validate content against provider limits before publishing
    provider
        .validate_post(&content)
        .map_err(|e| format!("Content validation failed: {e}"))?;

    provider
        .validate_media(&content)
        .map_err(|e| format!("Media validation failed: {e}"))?;

    // Publish with retry
    let mut last_error: Option<String> = None;
    let mut did_refresh = false; // guard against token refresh recursion

    for attempt in 1..=MAX_RETRIES {
        let attempt_start = Utc::now();
        match provider.post_pending(&access_token, &content).await {
            Ok(result) => {
                // A provider that only *accepted* the post (Instagram reel
                // still transcoding, async Reddit submit, X video still
                // encoding) has not published anything yet. Recording it
                // published now loses the post silently: the id is stored, no
                // media is ever live, and nothing in the state machine
                // notices. Poll it to a terminal state first.
                //
                // This runs *inside* the success arm on purpose — it is not a
                // publish retry. Once the platform has the post, re-running
                // `publish` would duplicate it, so a failed poll breaks out
                // to the error path instead of continuing the loop.
                let result = if result.is_pending() {
                    tracing::info!(
                        "Post {} accepted-but-pending on {}; polling until terminal",
                        post.id,
                        post.provider_identifier
                    );
                    match provider
                        .finalize_post(
                            &access_token,
                            &result.platform_post_id,
                            PENDING_POLL_INTERVAL,
                        )
                        .await
                    {
                        Ok(settled) => settled,
                        Err(e) => {
                            tracing::error!(
                                "Post {} never finished publishing on {}: {e}",
                                post.id,
                                post.provider_identifier
                            );
                            last_error = Some(crate::error::annotate_scope_loss(
                                &post.provider_identifier,
                                &e.to_string(),
                            ));
                            break;
                        }
                    }
                } else {
                    result
                };

                if !result.is_published() {
                    // Terminal but not live: the platform rejected the post
                    // after accepting it. Never write a non-live post as
                    // published.
                    tracing::error!(
                        "Post {} on {} ended in terminal state '{}', not published",
                        post.id,
                        post.provider_identifier,
                        result.status
                    );
                    last_error = Some(format!(
                        "{} rejected the post after accepting it (status: {})",
                        post.provider_identifier, result.status
                    ));
                    break;
                }

                // B6: a published result with an EMPTY platform id is not a
                // publish. X's cookie path returns status:"published" with
                // rest_id:"" on a malformed GraphQL 200 — recording that as
                // published stores a post with no platform id, silently
                // breaking analytics, first-comment, and thread linking.
                // (PostService::publish has the same guard; this covers the
                // scheduler path until Phase 2 unifies them into publish_one.)
                if result.platform_post_id.trim().is_empty() {
                    tracing::error!(
                        "Post {} on {} reported published with an empty platform_post_id",
                        post.id,
                        post.provider_identifier
                    );
                    last_error = Some(format!(
                        "{} returned status '{}' but no platform post id — not recording as published",
                        post.provider_identifier, result.status
                    ));
                    break;
                }

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
                    )
                    .await
                    {
                        Ok(_) => {
                            db_updated = true;
                            break;
                        }
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

                // v23-2: write to the publish_outbox so the drain loop
                // can retry the DB update if it failed above. This is
                // the durability safety net — even if all 3 direct DB
                // write attempts failed, the outbox row will be picked
                // up by drain_publish_outbox on the next tick (30s).
                write_to_outbox(
                    db,
                    post.id,
                    post.idempotency_key,
                    Some(&result.platform_post_id),
                    result.platform_post_url.as_deref(),
                    None,
                )
                .await;

                // Record successful attempt in audit trail.
                let _ = queries::record_publish_attempt(
                    db,
                    post.id,
                    attempt as i32,
                    "success",
                    None,
                    attempt_start,
                )
                .await;

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
                // v23: persist to events_log so the dashboard's "Recent
                // Activity" widget shows publishes even if no SSE client
                // was connected at the time.
                let _ = broadcaster
                    .send_and_log(db, post.user_id, "post_published", &event_payload)
                    .await;

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
                            // B7: the post is live — a failed first comment
                            // must not fail it. But it must be VISIBLE: event
                            // + events_log so the dashboard/operator sees
                            // the reply that never landed.
                            tracing::warn!("Failed to post first_comment for {}: {e}", post.id);
                            let payload = serde_json::json!({
                                "id": post.id.to_string(),
                                "platform_post_id": result.platform_post_id,
                                "error": e.to_string(),
                            });
                            broadcaster.send("first_comment_failed", &payload);
                            let _ = broadcaster
                                .send_and_log(db, post.user_id, "first_comment_failed", &payload)
                                .await;
                            dispatch_webhook_background(
                                db,
                                post.user_id,
                                "post.first_comment_failed",
                                &payload,
                            );                        }
                    }
                }

                return Ok(PublishOutcome::Published {
                    platform_post_url: result.platform_post_url,
                });
            }
            Err(ProviderError::TokenExpired) if !did_refresh => {
                tracing::warn!(
                    "Token expired for post {} (attempt {}). Refreshing...",
                    post.id,
                    attempt
                );
                did_refresh = true;
                // B8: refresh tokens are encrypted at rest — decrypt first.
                let old_refresh = crate::crypto::maybe_decrypt_token(
                    post.refresh_token.as_deref().unwrap_or(""),
                    token_key.as_ref(),
                );
                let new_token = match provider.refresh_token(&old_refresh).await {
                    Ok(t) => {
                        // B8: update_integration_token encrypts internally
                        // and fails (never downgrades) on encrypt error.
                        queries::update_integration_token(
                            db,
                            post.integration_id,
                            &t.access_token,
                            t.refresh_token.as_deref(),
                            t.expires_in
                                .map(|e| Utc::now() + chrono::Duration::seconds(e as i64)),
                            token_key,
                        )
                        .await?;
                        t.access_token
                    }
                    Err(e) => {
                        tracing::error!("Token refresh failed for post {}: {e}", post.id);
                        // Mark the integration as needing re-auth so the
                        // user gets a UI prompt to reconnect.
                        let _ =
                            queries::mark_integration_refresh_needed(db, post.integration_id).await;
                        // A lost grant gets an actionable message; every other
                        // refresh failure keeps its original text.
                        return Err(crate::error::annotate_scope_loss(
                            &post.provider_identifier,
                            &e.to_string(),
                        )
                        .into());
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
                    post.id,
                    attempt,
                    msg,
                    wait
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
                    post.id,
                    attempt,
                    err_str,
                    wait
                );
                last_error = Some(err_str);
                tokio::time::sleep(Duration::from_secs(wait)).await;
                continue;
            }
            Err(e) => {
                tracing::error!("Publish error for post {}: {e}", post.id);
                // Stored in post.error_message + the post.failed webhook, so
                // a lost grant has to name the provider and the fix itself.
                last_error = Some(crate::error::annotate_scope_loss(
                    &post.provider_identifier,
                    &e.to_string(),
                ));
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
    )
    .await;

    // Fire webhook for post.failed before returning the error.
    let fail_payload = serde_json::json!({
        "id": post.id.to_string(),
        "provider": post.provider_identifier,
        "error": last_error.clone().unwrap_or_default(),
    });
    dispatch_webhook_background(db, post.user_id, "post.failed", &fail_payload);

    Err(last_error
        .unwrap_or_else(|| "Max retries exceeded".to_string())
        .into())
}

/// Fire a webhook event in the background without blocking the caller.
///
/// We can't use `services::webhook_dispatcher::dispatch_event` directly
/// because it requires a full `&AppState` (which the scheduler doesn't
/// have — it only has `&SqlitePool`). Instead we inline a minimal version
/// that does the same DB query + HTTP send, spawned as a detached
/// task so the scheduler tick isn't delayed by webhook delivery.
fn dispatch_webhook_background(
    db: &SqlitePool,
    user_id: uuid::Uuid,
    event_type: &str,
    payload: &serde_json::Value,
) {
    let db = db.clone();
    let event_type = event_type.to_string();
    let payload = payload.clone();
    tokio::spawn(async move {
        // Fetch active webhooks matching this event type for the user.
        let webhooks: Vec<(uuid::Uuid, String, Option<String>)> = match sqlx::query_as(
            r#"SELECT w.id, w.url, w.secret FROM webhooks w
               WHERE w.user_id = ? AND w.is_active = 1
                 AND EXISTS (
                   SELECT 1 FROM webhook_event_types t
                   WHERE t.webhook_id = w.id AND t.event_type = ?
                 )"#,
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
            )
            .await;

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
                   (id, webhook_id, event_type, status, status_code, response_body, attempted_at)
                   VALUES (?, ?, ?, ?, ?, ?, unixepoch())"#,
            )
            .bind(Uuid::new_v4())
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
    db: &SqlitePool,
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
        // B8: refresh tokens are encrypted at rest once a key is
        // configured — decrypt before handing to the provider.
        let old_refresh = crate::crypto::maybe_decrypt_token(
            post.refresh_token.as_deref().unwrap_or(""),
            token_key.as_ref(),
        );
        let token = provider
            .refresh_token(&old_refresh)
            .await
            .map_err(|e| {
                crate::error::annotate_scope_loss(&post.provider_identifier, &e.to_string())
            })?;

        // B8: update_integration_token encrypts internally and fails
        // (never downgrades) on encrypt error.
        queries::update_integration_token(
            db,
            post.integration_id,
            &token.access_token,
            token.refresh_token.as_deref(),
            token
                .expires_in
                .map(|e| Utc::now() + chrono::Duration::seconds(e as i64)),
            token_key,
        )
        .await?;

        Ok(token.access_token)
    } else {
        Ok(crate::crypto::maybe_decrypt_token(
            &post.access_token,
            token_key.as_ref(),
        ))
    }
}

/// Mark a post as error — but only while it is still `publishing` (B1).
///
/// The scheduler drain-timeout path previously overwrote EVERY in-flight
/// post, including ones whose task had already written `published` (with
/// their platform_post_id) — flipping them to `error` while keeping the
/// platform id, so an operator retry created duplicates on the platform.
/// Gating on `state = 'publishing'` makes the late error write a no-op for
/// any post that already reached a terminal state.
async fn mark_post_error(db: &SqlitePool, post_id: uuid::Uuid, error: &str) {
    let result = sqlx::query(
        r#"UPDATE posts SET state = 'error', error_message = ?, updated_at = unixepoch()
         WHERE id = ? AND state = 'publishing'"#,
    )
    .bind(error)
    .bind(post_id)
    .execute(db)
    .await;
    if let Err(e) = result {
        tracing::error!("Failed to mark post {post_id} as error: {e}");
    }
}

// ── Posting-Gap Detection ─────────────────────────────────────

/// How long a post scheduled at `scheduled_at` is still on time, measured
/// at `now`. `Some(overdue)` once it is past the
/// [`POSTING_GAP_GRACE_SECS`] window, `None` while it is still within it.
///
/// Pure and `now`-parameterised so the grace boundary is unit-testable
/// with fabricated timestamps instead of a live database.
pub fn posting_gap_overdue(
    scheduled_at: chrono::DateTime<Utc>,
    now: chrono::DateTime<Utc>,
) -> Option<chrono::Duration> {
    let overdue = now - scheduled_at;
    (overdue > chrono::Duration::seconds(POSTING_GAP_GRACE_SECS)).then_some(overdue)
}

/// Render an overdue duration as a compact "3h 12m" label for the log
/// line and notification body.
fn format_overdue(overdue: chrono::Duration) -> String {
    let mins = overdue.num_minutes();
    if mins < 60 {
        format!("{mins}m")
    } else {
        format!("{}h {}m", mins / 60, mins % 60)
    }
}

/// One posting-gap check: find posts still `queued`/`publishing` past
/// their schedule and surface each as an in-app notification plus a warn
/// log. The Postiz `missing.post.workflow` equivalent, minus the email
/// transport.
///
/// Read-only — it never advances the state machine, so a stuck post is
/// still the operator's to re-queue or cancel.
async fn check_posting_gaps(
    db: &SqlitePool,
    broadcast: &Broadcaster,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let stuck = queries::find_stuck_posts(db, POSTING_GAP_GRACE_SECS, POSTING_GAP_LIMIT).await?;
    if stuck.is_empty() {
        return Ok(());
    }

    let now = Utc::now();
    for post in &stuck {
        // The query already filtered on the grace window; this re-check
        // is what the unit test pins, and it keeps the boundary in one
        // place if the SQL is ever loosened.
        let Some(overdue) = posting_gap_overdue(post.scheduled_at, now) else {
            continue;
        };
        let overdue_label = format_overdue(overdue);
        let preview = post.content_preview.trim();
        let preview = if preview.is_empty() {
            "(no text)"
        } else {
            preview
        };

        tracing::warn!(
            "Posting gap: post {} for user {} is still '{}' {} past its scheduled_at — \"{}\"",
            post.id,
            post.user_id,
            post.state,
            overdue_label,
            preview
        );

        let title = format!("Post not published ({overdue_label} late)");
        let body = format!(
            "Still '{}' {} after its scheduled time. Scheduled {}. \"{}\"",
            post.state,
            overdue_label,
            post.scheduled_at.to_rfc3339(),
            preview
        );
        if let Err(e) = crate::services::notifications::NotificationService::create_via(
            db,
            broadcast,
            post.user_id,
            &title,
            &body,
            "post_publishing_gap",
            Some("post"),
            Some(&post.id.to_string()),
        )
        .await
        {
            // A failed alert must not stall the rest of the batch — the
            // next tick retries it.
            tracing::warn!(
                "Failed to raise posting-gap notification for post {}: {e}",
                post.id
            );
        }
    }

    Ok(())
}

// ── Posting Streak ───────────────────────────────────────────

/// Update the user's posting streak when a post is published.
///
/// - If `streak_since` is NULL (never posted) → set to NOW, streak_days = 1
/// - If last post was yesterday (streak_since < NOW - 24h but < NOW - 0h) →
///   streak_days += 1, streak_since = NOW
/// - If last post was today (streak_since > NOW - 24h) → no change (already counted)
/// - The daily reset cron handles the "no post in 24h → reset" case
async fn update_streak_on_publish(db: &SqlitePool, user_id: uuid::Uuid) {
    let now = Utc::now();
    let row = sqlx::query("SELECT streak_since, streak_days FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(db)
        .await;

    match row {
        Ok(Some(r)) => {
            use sqlx::Row;
            let streak_since: Option<chrono::DateTime<chrono::Utc>> =
                r.try_get("streak_since").ok();
            let streak_days: i32 = r.try_get("streak_days").unwrap_or(0);

            if streak_since.is_none() {
                // First ever post
                let _ =
                    sqlx::query("UPDATE users SET streak_since = ?, streak_days = 1 WHERE id = ?")
                        .bind(now)
                        .bind(user_id)
                        .execute(db)
                        .await;
                tracing::info!("Streak: first post for user {user_id}, streak = 1");
            } else if let Some(since) = streak_since {
                let elapsed = now - since;
                if elapsed.num_hours() >= 24 {
                    let new_streak = streak_days + 1;
                    let _ = sqlx::query(
                        "UPDATE users SET streak_since = ?, streak_days = ? WHERE id = ?",
                    )
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
pub fn start_streak_reset(db: SqlitePool, shutdown: tokio::sync::watch::Receiver<bool>) {
    tracing::info!("Streak reset checker started (interval: 1 hour)");
    spawn_poll(
        Duration::from_secs(3600), // every hour
        FirstTick::AfterDelay,
        shutdown,
        "Streak reset checker shutting down",
        move || {
            let db = db.clone();
            async move {
                if let Err(e) = reset_expired_streaks(&db).await {
                    tracing::error!("Streak reset error: {e}");
                }
            }
        },
    );
}

async fn reset_expired_streaks(
    db: &SqlitePool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Reset streaks where no post has been published in the last 48 hours.
    // (48h gives a grace period so a streak isn't lost if you post at 11pm
    // one day and 1am the next — those are ~2h apart but count as 2 days.)
    let result = sqlx::query(
        "UPDATE users
         SET streak_days = 0, streak_since = NULL
         WHERE streak_since IS NOT NULL
           AND streak_since < unixepoch() - 172800",
    )
    .execute(db)
    .await?;

    if result.rows_affected() > 0 {
        tracing::info!(
            "Streak: reset {} user(s) with expired streaks",
            result.rows_affected()
        );
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
    db: SqlitePool,
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

async fn refresh_cache_cycle(
    db: &SqlitePool,
    providers: &ProviderRegistry,
    token_key: Option<[u8; 32]>,
) {
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
                    user.id,
                    integration.provider_identifier
                );
                continue;
            }

            let provider = match providers.get(&integration.provider_identifier) {
                Some(p) => p,
                None => continue,
            };

            // Decrypt token if encryption is enabled — without this,
            // every analytics call 401s when TOKEN_ENCRYPTION_KEY is set.
            let tok =
                crate::crypto::maybe_decrypt_token(&integration.access_token, token_key.as_ref());

            match provider.analytics(&tok, &integration.internal_id, 7).await {
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    /// Fixed reference instant so every boundary assertion is exact
    /// rather than dependent on when the test runs.
    fn base() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 30, 12, 0, 0).unwrap()
    }

    #[test]
    fn post_lagging_more_than_three_hours_is_a_gap() {
        let now = base();
        let scheduled_at = now - Duration::seconds(POSTING_GAP_GRACE_SECS + 60);
        assert!(posting_gap_overdue(scheduled_at, now).is_some());
    }

    #[test]
    fn post_lagging_just_under_three_hours_is_not_a_gap() {
        let now = base();
        let scheduled_at = now - Duration::seconds(POSTING_GAP_GRACE_SECS - 60);
        assert!(posting_gap_overdue(scheduled_at, now).is_none());
    }

    #[test]
    fn post_exactly_at_the_grace_boundary_is_not_a_gap() {
        let now = base();
        let scheduled_at = now - Duration::seconds(POSTING_GAP_GRACE_SECS);
        assert!(posting_gap_overdue(scheduled_at, now).is_none());
    }

    #[test]
    fn post_scheduled_in_the_future_is_not_a_gap() {
        let now = base();
        let scheduled_at = now + Duration::hours(2);
        assert!(posting_gap_overdue(scheduled_at, now).is_none());
    }

    #[test]
    fn overdue_duration_is_returned_for_a_gap() {
        let now = base();
        let scheduled_at = now - Duration::seconds(POSTING_GAP_GRACE_SECS + 720);
        assert_eq!(
            posting_gap_overdue(scheduled_at, now),
            Some(Duration::seconds(POSTING_GAP_GRACE_SECS + 720))
        );
    }

    #[test]
    fn grace_window_is_three_hours() {
        assert_eq!(POSTING_GAP_GRACE_SECS, 10_800);
    }

    #[test]
    fn overdue_label_renders_hours_and_minutes() {
        assert_eq!(format_overdue(Duration::minutes(192)), "3h 12m");
    }

    #[test]
    fn overdue_label_renders_minutes_under_an_hour() {
        assert_eq!(format_overdue(Duration::minutes(47)), "47m");
    }

    // ── B1: gated error write (audit 2026-10-14) ────────────────

    /// In-memory pool carrying the real schema, plus a user and an
    /// integration wired to a seeded post in the given state.
    async fn pool_with_post(state: &str, platform_post_id: Option<&str>) -> (crate::db::SqlitePool, uuid::Uuid) {
        let pool = crate::db::create_pool("sqlite::memory:").await.unwrap();
        let user_id = crate::auth::middleware::DEFAULT_USER_ID;
        crate::db::ensure_local_user(&pool).await.unwrap();
        let integration_id = uuid::Uuid::new_v4();
        sqlx::query(
            "INSERT INTO integrations
                (id, user_id, provider_identifier, provider_name, internal_id,
                 access_token, disabled, refresh_needed, posting_times, auth_method)
             VALUES (?, ?, 'x', 'X', 'x', 'tok', 0, 0, '[]', 'oauth')",
        )
        .bind(integration_id)
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();
        let post_id = uuid::Uuid::new_v4();
        sqlx::query(
            "INSERT INTO posts
                (id, user_id, integration_id, state, content, media, settings,
                 platform_post_id, idempotency_key)
             VALUES (?, ?, ?, ?, 'content', '[]', '{}', ?, ?)",
        )
        .bind(post_id)
        .bind(user_id)
        .bind(integration_id)
        .bind(state)
        .bind(platform_post_id)
        .bind(uuid::Uuid::new_v4())
        .execute(&pool)
        .await
        .unwrap();
        (pool, post_id)
    }

    /// B1: a post still `publishing` when the drain timeout fires gets its
    /// error state — the gated write must actually fire.
    #[tokio::test]
    async fn timeout_error_write_lands_for_a_still_publishing_post() {
        let (pool, post_id) = pool_with_post("publishing", None).await;
        mark_post_error(&pool, post_id, "Publish timed out after 300s").await;
        let state: String = sqlx::query_scalar("SELECT state FROM posts WHERE id = ?")
            .bind(post_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(state, "error", "an in-flight post must become error");
    }

    /// B1: the regression. A post whose task FINISHED (wrote `published`
    /// with its platform id) before the timeout arm ran used to be
    /// overwritten to `error` while keeping the platform id — the operator
    /// republished and duplicated it on the platform. The gate makes the
    /// late write a no-op.
    #[tokio::test]
    async fn timeout_error_write_spares_an_already_published_post() {
        let (pool, post_id) = pool_with_post("published", Some("tweet-123")).await;
        mark_post_error(&pool, post_id, "Publish timed out after 300s").await;
        let (state, pid): (String, Option<String>) =
            sqlx::query_as("SELECT state, platform_post_id FROM posts WHERE id = ?")
                .bind(post_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            state, "published",
            "a finished post must never be flipped to error by the timeout arm"
        );
        assert_eq!(
            pid.as_deref(),
            Some("tweet-123"),
            "platform_post_id must survive"
        );
    }
}