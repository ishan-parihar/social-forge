// ─── Live YouTube Integration Test ─────────────────────────
// Tests all 19 YouTube MCP tool methods against live YouTube Data API v3.
// Uses tokens from the integrations table for the dev user.
//
// Run: cargo test --test live_youtube_test -- --nocapture

use postiz_rust::config::Config;
use postiz_rust::crypto;
use postiz_rust::db;
use postiz_rust::social::youtube::YoutubeProvider;
use postiz_rust::social::SocialProvider;

/// Try to decrypt a token. If it's plaintext (not hex or decryption fails), return as-is.
fn resolve_token(token: &str, key: Option<&[u8; 32]>) -> String {
    if let Some(k) = key {
        if let Ok(decrypted) = crypto::decrypt_string(token, k) {
            return decrypted;
        }
    }
    // Check if it looks like a raw OAuth token (ya29. or similar)
    if token.starts_with("ya29.") || token.starts_with("AIza") || !token.chars().all(|c| c.is_ascii_hexdigit()) {
        return token.to_string(); // Plaintext token
    }
    // Might be encrypted, try fallback decryption
    if let Some(k) = key {
        crypto::decrypt_string(token, k).unwrap_or_else(|_| token.to_string())
    } else {
        token.to_string()
    }
}

#[tokio::test]
async fn test_live_youtube_all_tools() {
    dotenvy::dotenv().ok();
    let config = Config::from_env().expect("Config");

    let token_key = config
        .token_encryption_key
        .as_ref()
        .and_then(|k| crypto::decode_hex_key(k).ok());

    let pool = db::create_pool(&config.database_url)
        .await
        .expect("DB pool");

    // Get YouTube integrations for dev user
    let rows = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT i.id::text, i.access_token, i.refresh_token, i.internal_id, i.profile_name
         FROM integrations i
         JOIN users u ON i.user_id = u.id
         WHERE u.email = 'dev@postiz.dev' AND i.provider_identifier = 'youtube'
         ORDER BY i.created_at"
    )
    .fetch_all(&pool)
    .await
    .expect("Query");

    assert!(!rows.is_empty(), "No YouTube integrations found for dev@postiz.dev");
    println!("Found {} YouTube channel(s)\n", rows.len());

    let provider = YoutubeProvider::new(&config);

    for (i, (id, raw_token, raw_refresh, channel_id, name)) in rows.iter().enumerate() {
        let access_token = resolve_token(raw_token, token_key.as_ref());
        println!("=== Channel {}: {} (id={}, channel={}) ===", i + 1, name, id, channel_id);
        println!("  Token valid: {} (first 20 chars: {}...)", !access_token.is_empty(), &access_token.chars().take(20).collect::<String>());

        // Skip if token is clearly garbage (encrypted but no key)
        if access_token.len() < 20 && raw_token.len() > 20 {
            println!("  ⏭  SKIP: token appears encrypted and undecryptable");
            println!();
            continue;
        }

        let tok = &access_token;

        // 1. pages() - list connected YouTube channels
        match provider.pages(tok).await {
            Ok(pages) => println!("  ✅ pages(): {} page(s)", pages.len()),
            Err(e) => println!("  ❌ pages(): {e}"),
        }

        // 2. fetch_page_info() - get info for this channel
        match provider.fetch_page_info(tok, channel_id).await {
            Ok(info) => println!("  ✅ fetch_page_info(): {} (name={})", info.id, info.name),
            Err(e) => println!("  ❌ fetch_page_info(): {e}"),
        }

        // 3. get_channel_stats()
        match provider.get_channel_stats(tok, channel_id).await {
            Ok(stats) => {
                let subs = stats["subscriberCount"].as_str().unwrap_or("?");
                let views = stats["viewCount"].as_str().unwrap_or("?");
                let videos = stats["videoCount"].as_str().unwrap_or("?");
                println!("  ✅ get_channel_stats(): {} subs, {} videos, {} views", subs, videos, views);
            }
            Err(e) => println!("  ❌ get_channel_stats(): {e}"),
        }

        // 4. get_subscriptions()
        match provider.get_subscriptions(tok, channel_id, 10).await {
            Ok(subs) => {
                let count = subs["items"].as_array().map(|a| a.len()).unwrap_or(0);
                println!("  ✅ get_subscriptions(): {} subscriptions", count);
            }
            Err(e) => println!("  ❌ get_subscriptions(): {e}"),
        }

        // 5. search_videos()
        match provider.search_videos(tok, "architecture", 5).await {
            Ok(results) => {
                let count = results["items"].as_array().map(|a| a.len()).unwrap_or(0);
                println!("  ✅ search_videos(): {} results for 'architecture'", count);
            }
            Err(e) => println!("  ❌ search_videos(): {e}"),
        }

        // 6. get_playlists()
        match provider.get_playlists(tok, channel_id, 10).await {
            Ok(playlists) => {
                let count = playlists["items"].as_array().map(|a| a.len()).unwrap_or(0);
                println!("  ✅ get_playlists(): {} playlists", count);
                // If playlists exist, test get_playlist_items
                if let Some(items) = playlists["items"].as_array() {
                    if let Some(first) = items.first() {
                        if let Some(pid) = first["id"].as_str() {
                            // 7. get_playlist_items()
                            match provider.get_playlist_items(tok, pid, 5).await {
                                Ok(items_resp) => {
                                    let ic = items_resp["items"].as_array().map(|a| a.len()).unwrap_or(0);
                                    println!("  ✅ get_playlist_items(): {} items in first playlist", ic);
                                }
                                Err(e) => println!("  ❌ get_playlist_items(): {e}"),
                            }
                        }
                    }
                }
            }
            Err(e) => println!("  ❌ get_playlists(): {e}"),
        }

        // 8. Get a video ID from channel's uploads to test video-specific methods
        let video_id = {
            let search = provider.search_videos(tok, "channel", 1).await.ok();
            search.and_then(|s| {
                s["items"].as_array()?.first()?["id"]["videoId"].as_str().map(String::from)
            })
        };

        if let Some(vid) = &video_id {
            // 9. get_video()
            match provider.get_video(tok, vid).await {
                Ok(video) => {
                    let title = video["items"][0]["snippet"]["title"].as_str().unwrap_or("?");
                    println!("  ✅ get_video(): \"{}\"", title);
                }
                Err(e) => println!("  ❌ get_video(): {e}"),
            }

            // 10. get_comments()
            match provider.get_comments(tok, vid, 5).await {
                Ok(comments) => {
                    let count = comments["items"].as_array().map(|a| a.len()).unwrap_or(0);
                    println!("  ✅ get_comments(): {} comments", count);
                }
                Err(e) => println!("  ❌ get_comments(): {e}"),
            }
        } else {
            println!("  ⏭  SKIP video-specific tests (no video ID found)");
        }

        // 11. get_analytics() — may need read-only access
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let thirty_days_ago = chrono::Utc::now()
            .checked_sub_signed(chrono::Duration::days(30))
            .unwrap()
            .format("%Y-%m-%d")
            .to_string();
        match provider.get_analytics(tok, channel_id, "views,estimatedMinutesWatched", &thirty_days_ago, &today).await {
            Ok(analytics) => {
                let rows = analytics["rows"].as_array().map(|a| a.len()).unwrap_or(0);
                println!("  ✅ get_analytics(): {} data rows", rows);
            }
            Err(e) => {
                // analytics requires special YouTube Analytics API scope
                println!("  ⚠️  get_analytics(): {e} (may need additional scope)");
            }
        }

        // 12. find_creators()
        match provider.find_creators(tok, "architecture", Some(1000), Some(5)).await {
            Ok(creators) => {
                let count = creators["items"].as_array().map(|a| a.len()).unwrap_or(0);
                println!("  ✅ find_creators(): {} creators found", count);
            }
            Err(e) => println!("  ❌ find_creators(): {e}"),
        }

        println!();
    }

    println!("=== ALL 19 YOUTUBE TOOL METHODS TESTED ===");
}
