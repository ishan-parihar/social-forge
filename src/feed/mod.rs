// ─── Feed Refresher ─────────────────────────────────────────────

use std::sync::Arc;
use std::time::Duration;

use uuid::Uuid;

use crate::crypto;
use crate::db::PgPool;
use crate::realtime::Broadcaster;
use crate::social::registry::ProviderRegistry;
use crate::social::SocialProvider;

/// Default interval between full refresh cycles (seconds)
const DEFAULT_REFRESH_INTERVAL_SECS: u64 = 300; // 5 minutes

/// Default interval between engagement poll cycles (seconds)
const DEFAULT_ENGAGEMENT_INTERVAL_SECS: u64 = 1800; // 30 minutes

/// How many recent posts to fetch per integration per cycle
const RECENT_POSTS_LIMIT: u32 = 50;

/// Start the feed refresher background task.
/// Polls all integrations for new posts and periodically fetches engagement data.
pub fn start_feed_refresher(
    db: PgPool,
    providers: Arc<ProviderRegistry>,
    broadcaster: Broadcaster,
    token_key: Option<[u8; 32]>,
    shutdown: tokio::sync::watch::Receiver<bool>,
) {
    let db1 = db.clone();
    let mut shutdown1 = shutdown.clone();
    tokio::spawn(async move {
        let interval_secs = std::env::var("FEED_REFRESH_INTERVAL_SECONDS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_REFRESH_INTERVAL_SECS);
        tracing::info!(
            "Feed refresher started (interval: {interval_secs}s, engagement interval: {DEFAULT_ENGAGEMENT_INTERVAL_SECS}s)"
        );
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let mut engagement_interval = tokio::time::interval(Duration::from_secs(
            DEFAULT_ENGAGEMENT_INTERVAL_SECS,
        ));
        engagement_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = shutdown1.changed() => {
                    if *shutdown1.borrow() {
                        tracing::info!("Feed refresher shutting down...");
                        break;
                    }
                }
                _ = interval.tick() => {
                    if let Err(e) = refresh_all_posts(&db1, &providers, &broadcaster, token_key).await {
                        tracing::error!("Feed refresh error: {e}");
                    }
                }
                _ = engagement_interval.tick() => {
                    if let Err(e) = refresh_all_engagement(&db1, &providers, &broadcaster, token_key).await {
                        tracing::error!("Feed engagement refresh error: {e}");
                    }
                }
            }
        }
    });
    // Shutdown subscription stays alive through `shutdown`
}

/// Poll all non-disabled integrations for a specific user and import their recent posts.
/// Returns the count of newly imported posts.
pub async fn refresh_user_posts(
    db: &PgPool,
    user_id: Uuid,
    providers: &ProviderRegistry,
    broadcaster: &Broadcaster,
    token_key: Option<[u8; 32]>,
) -> anyhow::Result<u32> {
    let integrations = crate::db::queries::list_integrations(db, user_id).await?;
    let mut total_new = 0u32;

    for integration in &integrations {
        if integration.disabled {
            continue;
        }

        let provider = match providers.get(&integration.provider_identifier) {
            Some(p) => p,
            None => {
                tracing::debug!(
                    "Skipping integration {}: provider '{}' not in registry",
                    integration.id,
                    integration.provider_identifier
                );
                continue;
            }
        };

        let token = token_key
            .as_ref()
            .and_then(|key| crypto::decrypt_string(&integration.access_token, key).ok())
            .unwrap_or_else(|| integration.access_token.clone());

        let posts = match provider
            .get_recent_posts(&token, &integration.internal_id, RECENT_POSTS_LIMIT)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(
                    "Failed to fetch posts for integration {} ({}): {e}",
                    integration.id,
                    integration.provider_identifier
                );
                continue;
            }
        };

        let mut new_count = 0u32;
        for post in &posts {
            let media_val = serde_json::to_value(&post.media).unwrap_or_default();
            let metadata_val = post.metadata.clone().unwrap_or_default();
            match crate::db::queries::insert_external_post(
                db,
                integration.user_id,
                &integration.provider_identifier,
                &post.platform_post_id,
                &post.text,
                post.author_name.as_deref(),
                post.author_handle.as_deref(),
                post.author_avatar.as_deref(),
                post.created_at,
                post.url.as_deref(),
                &media_val,
                &metadata_val,
            )
            .await
            {
                Ok(Some(_)) => new_count += 1,
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!(
                        "Failed to import post {}: {e}",
                        post.platform_post_id
                    );
                }
            }
        }

        if new_count > 0 {
            total_new += new_count;
            tracing::info!(
                "Feed: imported {new_count} new {} post(s) for user {}",
                integration.provider_identifier,
                integration.user_id,
            );
        }
    }

    if total_new > 0 {
        broadcaster.send("feed:new_posts", &serde_json::json!({ "count": total_new }));
    }

    Ok(total_new)
}

/// Poll all non-disabled integrations for new posts and import them.
async fn refresh_all_posts(
    db: &PgPool,
    providers: &ProviderRegistry,
    broadcaster: &Broadcaster,
    token_key: Option<[u8; 32]>,
) -> anyhow::Result<()> {
    let integrations = crate::db::queries::list_all_integrations_across_users(db).await?;
    let mut total_new = 0u32;

    for integration in &integrations {
        if integration.disabled {
            continue;
        }

        let provider = match providers.get(&integration.provider_identifier) {
            Some(p) => p,
            None => {
                tracing::debug!(
                    "Skipping integration {}: provider '{}' not in registry",
                    integration.id,
                    integration.provider_identifier
                );
                continue;
            }
        };

        // Resolve token (decrypt if needed)
        let token = token_key
            .as_ref()
            .and_then(|key| crypto::decrypt_string(&integration.access_token, key).ok())
            .unwrap_or_else(|| integration.access_token.clone());

        // Fetch recent posts from provider
        let posts = match provider
            .get_recent_posts(&token, &integration.internal_id, RECENT_POSTS_LIMIT)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(
                    "Failed to fetch posts for integration {} ({}): {e}",
                    integration.id,
                    integration.provider_identifier
                );
                continue;
            }
        };

        let mut new_count = 0u32;
        for post in &posts {
            let media_val = serde_json::to_value(&post.media).unwrap_or_default();
            let metadata_val = post.metadata.clone().unwrap_or_default();
            tracing::debug!(
                "FEED INSERT DEBUG: provider={} post_id={} name={:?} handle={:?} avatar={} media_len={} media_json_len={}",
                integration.provider_identifier,
                post.platform_post_id,
                post.author_name,
                post.author_handle,
                post.author_avatar.is_some(),
                post.media.len(),
                media_val.as_array().map(|a| a.len()).unwrap_or(0),
            );
            match crate::db::queries::insert_external_post(
                db,
                integration.user_id,
                &integration.provider_identifier,
                &post.platform_post_id,
                &post.text,
                post.author_name.as_deref(),
                post.author_handle.as_deref(),
                post.author_avatar.as_deref(),
                post.created_at,
                post.url.as_deref(),
                &media_val,
                &metadata_val,
            )
            .await
            {
                Ok(Some(_)) => new_count += 1,
                Ok(None) => {} // already exists
                Err(e) => {
                    tracing::warn!(
                        "Failed to import post {}: {e}",
                        post.platform_post_id
                    );
                }
            }
        }

        if new_count > 0 {
            total_new += new_count;
            tracing::info!(
                "Feed: imported {new_count} new {} post(s) for user {}",
                integration.provider_identifier,
                integration.user_id,
            );
        }
    }

    if total_new > 0 {
        broadcaster.send("feed:new_posts", &serde_json::json!({ "count": total_new }));
    }

    Ok(())
}

/// Poll engagement data for existing external posts using the canonical fetch_engagement() pipeline.
/// Uses the SocialProvider::fetch_engagement() method to get normalized EngagementData,
/// then upserts into the post_engagement table.
async fn refresh_all_engagement(
    db: &PgPool,
    providers: &ProviderRegistry,
    broadcaster: &Broadcaster,
    token_key: Option<[u8; 32]>,
) -> anyhow::Result<()> {
    use crate::social::EngagementRow;

    let integrations = crate::db::queries::list_all_integrations_across_users(db).await?;
    let mut updated_count = 0u32;

    for integration in &integrations {
        if integration.disabled {
            continue;
        }

        let provider = match providers.get(&integration.provider_identifier) {
            Some(p) => p,
            None => continue,
        };

        let token = token_key
            .as_ref()
            .and_then(|key| crypto::decrypt_string(&integration.access_token, key).ok())
            .unwrap_or_else(|| integration.access_token.clone());

        // Fetch recent posts from DB to get their platform_post_ids
        let recent_posts =
            crate::db::queries::list_external_posts(db, integration.user_id, &integration.provider_identifier, 50)
                .await?;

        for post in &recent_posts {
            // Fetch normalized engagement data from provider
            let engagement = match provider
                .fetch_engagement(&token, &post.platform_post_id)
                .await
            {
                Ok(Some(e)) => e,
                Ok(None) => continue,
                Err(e) => {
                    tracing::debug!(
                        "Engagement fetch failed for {} post {}: {e}",
                        integration.provider_identifier,
                        post.platform_post_id
                    );
                    continue;
                }
            };

            // Convert to DB row and upsert into post_engagement table
            let row: EngagementRow = engagement.into();
            match crate::db::queries::upsert_post_engagement(db, post.id, &row).await {
                Ok(_) => updated_count += 1,
                Err(e) => {
                    tracing::warn!("Failed to upsert engagement for post {}: {e}", post.id);
                }
            }
        }
    }

    if updated_count > 0 {
        broadcaster.send(
            "feed:engagement",
            &serde_json::json!({ "updated": updated_count }),
        );
    }

    Ok(())
}
