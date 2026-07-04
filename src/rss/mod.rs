use std::sync::Arc;

use sha2::{Digest, Sha256};
use tokio::sync::watch;
use tracing;

use crate::config::Config;
use crate::db::models::PostState;
use crate::db::PgPool;
use crate::social::registry::ProviderRegistry;

pub fn start_rss_poller(
    db: PgPool,
    _providers: Arc<ProviderRegistry>,
    _config: Arc<Config>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    tokio::spawn(async move {
        let interval = tokio::time::Duration::from_secs(15 * 60); // every 15 min
        tracing::info!("RSS poller started (interval: 15 min)");
        loop {
            tokio::select! {
                _ = tokio::time::sleep(interval) => {
                    if let Err(e) = poll_all_feeds(&db).await {
                        tracing::error!("RSS poller error: {e}");
                    }
                }
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        tracing::info!("RSS poller shutting down");
                        break;
                    }
                }
            }
        }
    });
}

async fn poll_all_feeds(db: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let feeds = crate::db::queries::get_feeds_due_for_polling(db).await?;

    for feed in feeds {
        tracing::info!("Polling RSS feed: {} ({})", feed.title, feed.feed_url);

        let response = match reqwest::get(&feed.feed_url).await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("Failed to fetch RSS feed {}: {e}", feed.feed_url);
                continue;
            }
        };
        let xml = match response.text().await {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("Failed to read RSS response {}: {e}", feed.feed_url);
                continue;
            }
        };

        let parsed = match feed_rs::parser::parse(xml.as_bytes()) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Failed to parse RSS XML for {}: {e}", feed.feed_url);
                continue;
            }
        };

        for entry in parsed.entries {
            let title = entry.title.map(|t| t.content).unwrap_or_default();
            let url = entry
                .links
                .first()
                .map(|l| l.href.clone())
                .unwrap_or_default();
            let guid = entry.id;
            let published = entry
                .published
                .and_then(|d| chrono::DateTime::from_timestamp(d.timestamp(), 0));
            let content = entry
                .content
                .and_then(|c| c.body)
                .or_else(|| entry.summary.map(|s| s.content))
                .unwrap_or_default();

            let mut hasher = Sha256::new();
            hasher.update(content.as_bytes());
            let content_hash = format!("{:x}", hasher.finalize());

            let exists = crate::db::queries::check_rss_post_exists(db, feed.id, &content_hash)
                .await
                .unwrap_or(false);
            if exists {
                continue;
            }

            if let Err(e) = crate::db::queries::insert_rss_post(
                db,
                feed.id,
                &guid,
                &title,
                &url,
                published,
                &content_hash,
            )
            .await
            {
                tracing::warn!("Failed to insert RSS post for feed {}: {e}", feed.id);
                continue;
            }

            // Auto-create a queued post so the scheduler publishes it
            let post_content = format!("{}\n\n{}", title, url);
            match crate::db::queries::create_post(
                db,
                feed.user_id,
                feed.integration_id,
                &post_content,
                Some(&title),
                &serde_json::json!({}),
                &serde_json::json!({"rss_auto": true}),
                None,
                Some(PostState::Queued),
                None,
                0,
            )
            .await
            {
                Ok(post) => {
                    // Link the rss_post to the created post
                    if let Ok(rss_post) =
                        crate::db::queries::get_rss_post_by_hash(db, feed.id, &content_hash).await
                    {
                        if let Some(rp) = rss_post {
                            if let Err(e) = crate::db::queries::update_rss_post_post_id(
                                db, rp.id, post.id,
                            )
                            .await
                            {
                                tracing::warn!("Failed to update RSS post post_id: {e}");
                            }
                        }
                    }
                    tracing::info!(
                        "RSS autopost: created queued post {} for feed {}",
                        post.id,
                        feed.id
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "RSS autopost: failed to create post for feed {}: {e}",
                        feed.id
                    );
                }
            }
        }

        if let Err(e) = crate::db::queries::update_feed_last_polled(db, feed.id).await {
            tracing::warn!("Failed to update last_polled for feed {}: {e}", feed.id);
        }
    }

    Ok(())
}
