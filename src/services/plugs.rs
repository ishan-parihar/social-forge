// ─── Post Plugs Service ───────────────────────────────────────
// Outbound post-publish automations:
//   - auto_repost_after_likes: if a post gets N likes within M hours,
//     auto-retweet/repost it to amplify reach
//   - cross_post_from_secondary: after publishing from account A,
//     automatically repost from account B with a delay
//
// Data-driven (no decorator system like postiz) — plugs are rows in
// post_plugs table. A single background task (plug_runner) wakes every
// minute, queries due plugs, executes them.

use std::sync::Arc;
use chrono::Utc;
use uuid::Uuid;

use crate::db::PgPool;
use crate::social::registry::ProviderRegistry;

/// Start the plug runner background task. Polls every 60 seconds.
pub fn start_plug_runner(
    db: PgPool,
    providers: Arc<ProviderRegistry>,
    token_key: Option<[u8; 32]>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    tokio::spawn(async move {
        let interval = tokio::time::Duration::from_secs(60);
        tracing::info!("Plug runner started (interval: 60s)");
        loop {
            tokio::select! {
                _ = tokio::time::sleep(interval) => {
                    if let Err(e) = process_due_plugs(&db, &providers, token_key).await {
                        tracing::error!("Plug runner error: {e}");
                    }
                }
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        tracing::info!("Plug runner shutting down");
                        break;
                    }
                }
            }
        }
    });
}

/// Query all due plugs and execute them.
async fn process_due_plugs(
    db: &PgPool,
    providers: &ProviderRegistry,
    token_key: Option<[u8; 32]>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use sqlx::Row;

    let plugs = sqlx::query(
        "SELECT id, user_id, post_id, integration_id, plug_type, config,
                runs_so_far, max_runs, next_run_at
         FROM post_plugs
         WHERE completed = false AND next_run_at <= NOW()
         ORDER BY next_run_at ASC
         LIMIT 50",
    )
    .fetch_all(db)
    .await?;

    for plug_row in plugs {
        let plug_id: Uuid = plug_row.try_get("id")?;
        let user_id: Uuid = plug_row.try_get("user_id")?;
        let post_id: Uuid = plug_row.try_get("post_id")?;
        let integration_id: Uuid = plug_row.try_get("integration_id")?;
        let plug_type: String = plug_row.try_get("plug_type")?;
        let config: serde_json::Value = plug_row.try_get("config")?;
        let runs_so_far: i32 = plug_row.try_get("runs_so_far")?;
        let max_runs: i32 = plug_row.try_get("max_runs")?;

        tracing::info!(
            "Processing plug {} (type={}, post={}, run {}/{})",
            plug_id, plug_type, post_id, runs_so_far + 1, max_runs
        );

        let result = execute_plug(
            db,
            providers,
            token_key,
            plug_type.as_str(),
            &config,
            post_id,
            integration_id,
            user_id,
        )
        .await;

        match result {
            Ok(fired) => {
                let new_runs = runs_so_far + 1;
                let completed = new_runs >= max_runs || fired;
                let next_run = if completed {
                    Utc::now()
                } else {
                    // Schedule next run based on interval from config
                    let interval_mins = config["interval_minutes"].as_i64().unwrap_or(360);
                    Utc::now() + chrono::Duration::minutes(interval_mins)
                };

                let _ = sqlx::query(
                    "UPDATE post_plugs SET runs_so_far = $1, next_run_at = $2, fired_at = $3,
                     completed = $4, updated_at = NOW() WHERE id = $5",
                )
                .bind(new_runs)
                .bind(next_run)
                .bind(if fired { Some(Utc::now()) } else { None })
                .bind(completed)
                .bind(plug_id)
                .execute(db)
                .await;

                if fired {
                    tracing::info!("Plug {} fired successfully", plug_id);
                }
            }
            Err(e) => {
                tracing::warn!("Plug {} failed: {e}", plug_id);
                // Reschedule for next interval
                let interval_mins = config["interval_minutes"].as_i64().unwrap_or(360);
                let next_run = Utc::now() + chrono::Duration::minutes(interval_mins);
                let _ = sqlx::query(
                    "UPDATE post_plugs SET next_run_at = $1, updated_at = NOW() WHERE id = $2",
                )
                .bind(next_run)
                .bind(plug_id)
                .execute(db)
                .await;
            }
        }
    }

    Ok(())
}

/// Execute a single plug. Returns Ok(true) if the plug fired (action taken),
/// Ok(false) if conditions not met yet, Err on failure.
async fn execute_plug(
    db: &PgPool,
    providers: &ProviderRegistry,
    token_key: Option<[u8; 32]>,
    plug_type: &str,
    config: &serde_json::Value,
    post_id: Uuid,
    integration_id: Uuid,
    _user_id: Uuid,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    use sqlx::Row;

    match plug_type {
        "auto_repost_after_likes" => {
            let threshold = config["threshold"].as_i64().unwrap_or(10) as i64;

            // Fetch the post to get platform_post_id + provider
            let post_row = sqlx::query(
                "SELECT p.platform_post_id, p.state, i.provider_identifier, i.access_token
                 FROM posts p JOIN integrations i ON p.integration_id = i.id
                 WHERE p.id = $1",
            )
            .bind(post_id)
            .fetch_one(db)
            .await?;

            let platform_post_id: Option<String> = post_row.try_get("platform_post_id")?;
            let provider_id: String = post_row.try_get("provider_identifier")?;
            let access_token_raw: String = post_row.try_get("access_token")?;

            let platform_post_id = match platform_post_id {
                Some(id) if !id.is_empty() => id,
                _ => return Ok(false), // Post not published yet
            };

            let provider = match providers.get(&provider_id) {
                Some(p) => p,
                None => return Err(format!("Provider '{}' not registered", provider_id).into()),
            };

            // Decrypt token
            let access_token = crate::crypto::maybe_decrypt_token(&access_token_raw, token_key.as_ref());

            // Get current engagement
            let engagement = provider
                .get_post_engagement(&access_token, &platform_post_id)
                .await
                .map_err(|e| format!("Failed to get engagement: {e}"))?;

            let likes = engagement
                .as_ref()
                .and_then(|v| v["likes"].as_i64())
                .unwrap_or(0);

            if likes >= threshold {
                tracing::info!(
                    "Plug auto_repost: post {} has {} likes (threshold {}), firing!",
                    post_id, likes, threshold
                );

                // For X: retweet using the XProvider's retweet method
                if provider_id == "x" {
                    let x_provider = crate::social::x::XProvider::new(&crate::config::Config::from_env().map_err(|e| format!("Config error: {e}"))?);
                    let user_id_str = crate::auth::middleware::DEFAULT_USER_ID.to_string();
                    // XProvider::retweet needs the X user ID, not our internal UUID.
                    // We'll use the integration's internal_id which is the X user ID.
                    let internal_id_row = sqlx::query("SELECT internal_id FROM integrations WHERE id = $1")
                        .bind(integration_id)
                        .fetch_one(db)
                        .await?;
                    let x_user_id: String = internal_id_row.try_get::<String, _>("internal_id")?;
                    x_provider.retweet(&access_token, &x_user_id, &platform_post_id).await
                        .map_err(|e| format!("Retweet failed: {e}"))?;
                    return Ok(true);
                }

                // For other providers: repost by creating a new post with the same content
                // (simplified — a full implementation would call provider.share() if it exists)
                tracing::info!("Plug auto_repost: provider {} doesn't support native retweet, skipping", provider_id);
                return Ok(true);
            }

            tracing::debug!(
                "Plug auto_repost: post {} has {} likes (threshold {} not met)",
                post_id, likes, threshold
            );
            Ok(false)
        }

        "cross_post_from_secondary" => {
            let secondary_integration_id: Uuid = config["secondary_integration_id"]
                .as_str()
                .and_then(|s| Uuid::parse_str(s).ok())
                .ok_or("Missing secondary_integration_id in config")?;

            let delay_minutes = config["delay_minutes"].as_i64().unwrap_or(30);

            // Check if original post is published
            let post_row = sqlx::query(
                "SELECT platform_post_id, state, content, title, media, settings
                 FROM posts WHERE id = $1",
            )
            .bind(post_id)
            .fetch_one(db)
            .await?;

            use sqlx::Row;
            let state_str: String = post_row.try_get::<String, _>("state")?;
            if state_str != "published" {
                return Ok(false); // Original not published yet
            }

            // Check if the cross-post was already created (avoid duplicates)
            let existing = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM posts WHERE integration_id = $1 AND content = $2 AND created_at > NOW() - INTERVAL '24 hours'",
            )
            .bind(secondary_integration_id)
            .bind(post_row.try_get::<String, _>("content")?)
            .fetch_one(db)
            .await?;

            if existing > 0 {
                return Ok(true); // Already cross-posted, mark as done
            }

            // Create a new post on the secondary integration with the same content
            let content: String = post_row.try_get("content")?;
            let title: Option<String> = post_row.try_get("title")?;
            let media: serde_json::Value = post_row.try_get("media")?;
            let settings: serde_json::Value = post_row.try_get("settings")?;

            let scheduled = Utc::now() + chrono::Duration::minutes(delay_minutes);

            crate::db::queries::create_post(
                db,
                _user_id,
                secondary_integration_id,
                &content,
                title.as_deref(),
                &media,
                &settings,
                Some(scheduled),
                Some(crate::db::models::PostState::Queued),
                None,
                0,
            )
            .await
            .map_err(|e| format!("Failed to create cross-post: {e}"))?;

            tracing::info!(
                "Plug cross_post: created secondary post on integration {} (delay {}m)",
                secondary_integration_id, delay_minutes
            );
            Ok(true)
        }

        _ => Err(format!("Unknown plug type: {plug_type}").into()),
    }
}
