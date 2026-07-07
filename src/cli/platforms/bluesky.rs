// ── Bluesky CLI Handler ───────────────────────────────────────
// Relocated from cli/run.rs to match the platforms/*.rs shim pattern.

use crate::api::AppState;
use crate::cli::BlueskyAction;
use crate::cli::platforms::emit_result;
use crate::social::SocialProvider;

pub async fn handle(action: BlueskyAction, state: &AppState) -> anyhow::Result<()> {
    let user_id = crate::cli::run::resolve_user(state).await?;

    let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
    let bs = integrations.iter()
        .find(|i| i.provider_identifier == "bluesky")
        .ok_or_else(|| anyhow::anyhow!("No Bluesky integration found"))?;
    let token = crate::crypto::maybe_decrypt_token(&bs.access_token, state.token_key.as_ref());

    let provider = crate::social::bluesky::BlueskyProvider::new(&state.config);

    let result: Result<serde_json::Value, String> = match action {
        BlueskyAction::Reply { post_uri, content } => {
            let post = crate::social::PostContent {
                content,
                media: vec![],
                settings: serde_json::Value::Object(serde_json::Map::new()),
            in_reply_to: None,
            idempotency_key: None,
            };
            provider.reply_to_comment(&token, &post_uri, &post).await
                .map(|r| serde_json::json!({"id": r.platform_post_id, "url": r.platform_post_url, "status": r.status}))
                .map_err(|e| e.to_string())
        }
        BlueskyAction::Profile { handle } => {
            match reqwest::Client::new()
                .get("https://bsky.social/xrpc/app.bsky.actor.getProfile")
                .query(&[("actor", &handle)])
                .header("Authorization", format!("Bearer {token}"))
                .send().await
            {
                Ok(resp) => resp.json::<serde_json::Value>().await
                    .map_err(|e| format!("Parse error: {e}")),
                Err(e) => Err(format!("Bluesky profile request failed: {e}")),
            }
        }
        BlueskyAction::Timeline { limit } => {
            match reqwest::Client::new()
                .get("https://bsky.social/xrpc/app.bsky.feed.getTimeline")
                .query(&[("limit", &limit.to_string())])
                .header("Authorization", format!("Bearer {token}"))
                .send().await
            {
                Ok(resp) => resp.json::<serde_json::Value>().await
                    .map_err(|e| format!("Parse error: {e}")),
                Err(e) => Err(format!("Bluesky timeline request failed: {e}")),
            }
        }
        BlueskyAction::Search { query, limit } => {
            match reqwest::Client::new()
                .get("https://bsky.social/xrpc/app.bsky.feed.searchPosts")
                .query(&[("q", &query), ("limit", &limit.to_string())])
                .header("Authorization", format!("Bearer {token}"))
                .send().await
            {
                Ok(resp) => resp.json::<serde_json::Value>().await
                    .map_err(|e| format!("Parse error: {e}")),
                Err(e) => Err(format!("Bluesky search request failed: {e}")),
            }
        }
        BlueskyAction::Post { text, images: _ } => {
            let post = crate::social::PostContent {
                content: text,
                media: vec![],
                settings: serde_json::Value::Object(serde_json::Map::new()),
            in_reply_to: None,
            idempotency_key: None,
            };
            provider.publish(&token, &post).await
                .map(|r| serde_json::json!({"id": r.platform_post_id, "url": r.platform_post_url, "status": r.status}))
                .map_err(|e| e.to_string())
        }
        BlueskyAction::Feed { feed_type: _, limit } => {
            match reqwest::Client::new()
                .get("https://bsky.social/xrpc/app.bsky.feed.getTimeline")
                .query(&[("limit", &limit.to_string())])
                .header("Authorization", format!("Bearer {token}"))
                .send().await
            {
                Ok(resp) => resp.json::<serde_json::Value>().await
                    .map_err(|e| format!("Parse error: {e}")),
                Err(e) => Err(format!("Bluesky feed request failed: {e}")),
            }
        }
    };

    emit_result(result)
}
