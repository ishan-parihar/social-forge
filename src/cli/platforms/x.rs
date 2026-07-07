// ── X/Twitter CLI Handler ─────────────────────────────────────
// Relocated from cli/run.rs to match the platforms/*.rs shim pattern.

use crate::api::AppState;
use crate::cli::XAction;
use crate::cli::platforms::emit_result;
use crate::cli::run::find_x_token;
use crate::cli::run::resolve_user;
use crate::social::SocialProvider;

pub async fn handle(action: XAction, state: &AppState) -> anyhow::Result<()> {
    let user_id = resolve_user(state).await?;
    let (token, my_id) = find_x_token(state, user_id).await?;

    let mut provider = crate::social::x::XProvider::new(&state.config);
    provider.prepare_from_token(&token);

    let result: Result<serde_json::Value, String> = match action {
        XAction::Post { text } => {
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
        XAction::Timeline { count } => {
            provider.home_timeline(&token, &my_id, count, None).await
                .map_err(|e| e.to_string())
        }
        XAction::Search { query } => {
            provider.search_tweets(&token, &query, 20, None).await
                .map_err(|e| e.to_string())
        }
        XAction::Like { tweet_id } => {
            provider.like_tweet(&token, &my_id, &tweet_id).await
                .map_err(|e| e.to_string())
        }
        XAction::Retweet { tweet_id } => {
            provider.retweet(&token, &my_id, &tweet_id).await
                .map_err(|e| e.to_string())
        }
        XAction::Delete { tweet_id } => {
            provider.delete_tweet(&token, &tweet_id).await
                .map_err(|e| e.to_string())
        }
        XAction::Bookmark { tweet_id } => {
            provider.bookmark_tweet(&token, &my_id, &tweet_id).await
                .map_err(|e| e.to_string())
        }
        XAction::User { username } => {
            if username.chars().all(|c| c.is_ascii_digit()) {
                provider.user_lookup(&token, &username).await
                    .map_err(|e| e.to_string())
            } else {
                provider.user_lookup_by_username(&token, &username).await
                    .map_err(|e| e.to_string())
            }
        }
        XAction::Reply { tweet_id, text } => {
            let post = crate::social::PostContent {
                content: text,
                media: vec![],
                settings: serde_json::Value::Object(serde_json::Map::new()),
            in_reply_to: None,
            idempotency_key: None,
            };
            provider.reply_to_comment(&token, &tweet_id, &post).await
                .map(|r| serde_json::json!({"id": r.platform_post_id, "url": r.platform_post_url, "status": r.status}))
                .map_err(|e| e.to_string())
        }
        XAction::Dm { recipient, text } => {
            let post = crate::social::PostContent {
                content: text,
                media: vec![],
                settings: serde_json::Value::Object(serde_json::Map::new()),
            in_reply_to: None,
            idempotency_key: None,
            };
            provider.send_dm(&token, &recipient, &post).await
                .map(|r| serde_json::json!({"id": r.platform_post_id, "status": r.status}))
                .map_err(|e| e.to_string())
        }
        XAction::DmList { count } => {
            provider.get_dm_conversations(&token, count).await
                .map(|convs| {
                    let list: Vec<serde_json::Value> = convs.into_iter().map(|c| {
                        serde_json::json!({
                            "id": c.id,
                            "participant": c.participant,
                            "last_message": c.last_message,
                            "last_message_at": c.last_message_at.map(|dt| dt.to_rfc3339()),
                        })
                    }).collect();
                    serde_json::json!({"conversations": list, "total": list.len()})
                })
                .map_err(|e| e.to_string())
        }
        XAction::DmMessages { conversation_id, count } => {
            provider.get_dm_messages(&token, &conversation_id, count).await
                .map(|msgs| {
                    let list: Vec<serde_json::Value> = msgs.into_iter().map(|m| {
                        serde_json::json!({
                            "id": m.id,
                            "sender": m.sender,
                            "content": m.content,
                            "created_at": m.created_at.to_rfc3339(),
                        })
                    }).collect();
                    serde_json::json!({"messages": list, "total": list.len()})
                })
                .map_err(|e| e.to_string())
        }
    };

    emit_result(result)
}
