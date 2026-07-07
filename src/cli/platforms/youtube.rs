// ── YouTube CLI Handler ───────────────────────────────────────
// Relocated from cli/run.rs to match the platforms/*.rs shim pattern.
// Calls YoutubeProvider directly for reply/search/video, and raw HTTP
// for YouTube Data API v3 endpoints (playlists/stats/analytics/etc.).

use crate::api::AppState;
use crate::cli::YoutubeAction;
use crate::cli::platforms::emit_result;
use crate::social::SocialProvider;

pub async fn handle(action: YoutubeAction, state: &AppState) -> anyhow::Result<()> {
    let user_id = crate::cli::run::resolve_user(state).await?;

    let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
    let yt = integrations.iter()
        .find(|i| i.provider_identifier == "youtube")
        .ok_or_else(|| anyhow::anyhow!("No YouTube integration found"))?;
    let token = crate::crypto::maybe_decrypt_token(&yt.access_token, state.token_key.as_ref());

    let provider = crate::social::youtube::YoutubeProvider::new(&state.config);

    let result: Result<serde_json::Value, String> = match action {
        YoutubeAction::Reply { comment_id, content } => {
            let post = crate::social::PostContent {
                content,
                media: vec![],
                settings: serde_json::Value::Object(serde_json::Map::new()),
            in_reply_to: None,
            idempotency_key: None,
            delay_minutes: None
            };
            provider.reply_to_comment(&token, &comment_id, &post).await
                .map(|r| serde_json::json!({"id": r.platform_post_id, "status": r.status}))
                .map_err(|e| e.to_string())
        }
        YoutubeAction::Search { query, limit } => {
            provider.search_videos(&token, &query, limit).await
                .map_err(|e| e.to_string())
        }
        YoutubeAction::Video { video_id } => {
            provider.get_video(&token, &video_id).await
                .map_err(|e| e.to_string())
        }
        YoutubeAction::Playlists { channel_id, limit } => {
            let client = reqwest::Client::new();
            let url = format!("https://www.googleapis.com/youtube/v3/playlists?part=snippet&channelId={channel_id}&maxResults={limit}");
            match client.get(&url).bearer_auth(&token).send().await {
                Ok(r) => r.json::<serde_json::Value>().await.map_err(|e| format!("Parse error: {e}")),
                Err(e) => Err(format!("YouTube playlists failed: {e}")),
            }
        }
        YoutubeAction::Stats { channel_id } => {
            let client = reqwest::Client::new();
            let url = format!("https://www.googleapis.com/youtube/v3/channels?part=statistics&id={channel_id}");
            match client.get(&url).bearer_auth(&token).send().await {
                Ok(r) => r.json::<serde_json::Value>().await.map_err(|e| format!("Parse error: {e}")),
                Err(e) => Err(format!("YouTube stats failed: {e}")),
            }
        }
        YoutubeAction::Analytics { channel_id, metrics, start_date, end_date } => {
            let client = reqwest::Client::new();
            let url = format!("https://youtubeanalytics.googleapis.com/v2/reports?ids=channel=={channel_id}&startDate={start_date}&endDate={end_date}&metrics={metrics}");
            match client.get(&url).bearer_auth(&token).send().await {
                Ok(r) => r.json::<serde_json::Value>().await.map_err(|e| format!("Parse error: {e}")),
                Err(e) => Err(format!("YouTube analytics failed: {e}")),
            }
        }
        YoutubeAction::Subscriptions { channel_id, limit } => {
            let client = reqwest::Client::new();
            let url = format!("https://www.googleapis.com/youtube/v3/subscriptions?part=snippet&channelId={channel_id}&maxResults={limit}");
            match client.get(&url).bearer_auth(&token).send().await {
                Ok(r) => r.json::<serde_json::Value>().await.map_err(|e| format!("Parse error: {e}")),
                Err(e) => Err(format!("YouTube subscriptions failed: {e}")),
            }
        }
        YoutubeAction::Creators { query, limit } => {
            let client = reqwest::Client::new();
            let url = format!("https://www.googleapis.com/youtube/v3/search?part=snippet&type=channel&q={query}&maxResults={limit}");
            match client.get(&url).bearer_auth(&token).send().await {
                Ok(r) => r.json::<serde_json::Value>().await.map_err(|e| format!("Parse error: {e}")),
                Err(e) => Err(format!("YouTube creators search failed: {e}")),
            }
        }
    };

    emit_result(result)
}
