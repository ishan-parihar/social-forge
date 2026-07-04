// ── Mastodon CLI Handler ──────────────────────────────────────
// Relocated from cli/run.rs to match the platforms/*.rs shim pattern.

use crate::api::AppState;
use crate::cli::MastodonAction;
use crate::cli::platforms::emit_result;

pub async fn handle(action: MastodonAction, state: &AppState) -> anyhow::Result<()> {
    let user_id = crate::cli::run::resolve_user(state).await?;

    let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
    let ms = integrations.iter()
        .find(|i| i.provider_identifier == "mastodon")
        .ok_or_else(|| anyhow::anyhow!("No Mastodon integration found"))?;
    let token = crate::crypto::maybe_decrypt_token(&ms.access_token, state.token_key.as_ref());

    let provider = crate::social::mastodon::MastodonProvider::new(&state.config);

    let result: Result<serde_json::Value, String> = match action {
        MastodonAction::Reply { status_id, content } => {
            let post = crate::social::PostContent {
                content,
                media: vec![],
                settings: serde_json::Value::Object(serde_json::Map::new()),
            };
            provider.reply_to_comment(&token, &status_id, &post).await
                .map(|r| serde_json::json!({"id": r.platform_post_id, "url": r.platform_post_url, "status": r.status}))
                .map_err(|e| e.to_string())
        }
        MastodonAction::Whoami => {
            provider.get_user_info(&token).await
                .map_err(|e| e.to_string())
        }
        MastodonAction::Timeline { kind, limit } => {
            let url = match kind.as_str() {
                "local" => format!("/api/v1/timelines/public?local=true&limit={}", limit.min(40)),
                "public" => format!("/api/v1/timelines/public?limit={}", limit.min(40)),
                _ => format!("/api/v1/timelines/home?limit={}", limit.min(40)),
            };
            match reqwest::Client::new()
                .get(provider.api_url(&url))
                .header("Authorization", format!("Bearer {token}"))
                .send().await
            {
                Ok(resp) => resp.json::<serde_json::Value>().await
                    .map_err(|e| format!("Parse error: {e}")),
                Err(e) => Err(format!("Mastodon timeline request failed: {e}")),
            }
        }
        MastodonAction::Search { query, limit } => {
            let url = format!("/api/v2/search?q={}&limit={}", urlencoding::encode(&query), limit.min(40));
            match reqwest::Client::new()
                .get(provider.api_url(&url))
                .header("Authorization", format!("Bearer {token}"))
                .send().await
            {
                Ok(resp) => resp.json::<serde_json::Value>().await
                    .map_err(|e| format!("Parse error: {e}")),
                Err(e) => Err(format!("Mastodon search request failed: {e}")),
            }
        }
        MastodonAction::Post { text, visibility } => {
            let body = serde_json::json!({"status": text, "visibility": visibility});
            match reqwest::Client::new()
                .post(provider.api_url("/api/v1/statuses"))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .json(&body)
                .send().await
            {
                Ok(r) => r.json::<serde_json::Value>().await.map_err(|e| format!("Parse error: {e}")),
                Err(e) => Err(format!("Mastodon post failed: {e}")),
            }
        }
        MastodonAction::Get { status_id } => {
            match reqwest::Client::new()
                .get(provider.api_url(&format!("/api/v1/statuses/{status_id}")))
                .header("Authorization", format!("Bearer {token}"))
                .send().await
            {
                Ok(r) => r.json::<serde_json::Value>().await.map_err(|e| format!("Parse error: {e}")),
                Err(e) => Err(format!("Mastodon get status failed: {e}")),
            }
        }
    };

    emit_result(result)
}
