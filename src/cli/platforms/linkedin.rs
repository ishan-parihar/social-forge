// ── LinkedIn Personal CLI Handler ─────────────────────────────
// Relocated from cli/run.rs to match the platforms/*.rs shim pattern.

use crate::api::AppState;
use crate::cli::LinkedinAction;
use crate::cli::platforms::emit_result;
use crate::cli::run::{find_linkedin_token, resolve_user};
use crate::social::SocialProvider;

pub async fn handle(action: LinkedinAction, state: &AppState) -> anyhow::Result<()> {
    let user_id = resolve_user(state).await?;
    let (token, li_id) = find_linkedin_token(state, user_id).await?;

    let provider = crate::social::linkedin::LinkedInProvider::new(&state.config);

    let result: Result<serde_json::Value, String> = match action {
        LinkedinAction::Profile => {
            provider.get_profile(&token).await.map_err(|e| e.to_string())
        }
        LinkedinAction::Posts { limit } => {
            let author_urn = format!("urn:li:person:{li_id}");
            provider.get_posts(&token, &author_urn, limit).await.map_err(|e| e.to_string())
        }
        LinkedinAction::Post { text } => {
            let post = crate::social::PostContent {
                content: text,
                media: vec![],
                settings: serde_json::Value::Object(serde_json::Map::new()),
            in_reply_to: None,
            };
            provider.publish(&token, &post).await
                .map(|r| serde_json::json!({"id": r.platform_post_id, "url": r.platform_post_url, "status": r.status}))
                .map_err(|e| e.to_string())
        }
        LinkedinAction::Delete { post_urn } => {
            let url = format!(
                "https://api.linkedin.com/v2/rest/posts/{}",
                urlencoding::encode(&post_urn)
            );
            let resp = reqwest::Client::new()
                .delete(&url)
                .header("Authorization", format!("Bearer {token}"))
                .header("X-Restli-Protocol-Version", "2.0.0")
                .header("LinkedIn-Version", "202401")
                .send().await
                .map_err(|e| format!("LinkedIn delete failed: {e}"));
            match resp {
                Ok(r) if r.status().is_success() => Ok(serde_json::json!({"deleted": true})),
                Ok(r) => Err(format!("LinkedIn delete failed ({})", r.status())),
                Err(e) => Err(e),
            }
        }
        LinkedinAction::Reactions { post_urn } => {
            let url = format!(
                "https://api.linkedin.com/v2/rest/reactions/(entity:{})",
                urlencoding::encode(&post_urn)
            );
            let resp = reqwest::Client::new()
                .get(&url)
                .header("Authorization", format!("Bearer {token}"))
                .header("X-Restli-Protocol-Version", "2.0.0")
                .header("LinkedIn-Version", "202401")
                .send().await
                .map_err(|e| format!("LinkedIn reactions failed: {e}"));
            match resp {
                Ok(r) => r.json::<serde_json::Value>().await.map_err(|e| format!("Parse error: {e}")),
                Err(e) => Err(e),
            }
        }
        LinkedinAction::Analytics => {
            let author_urn = format!("urn:li:person:{li_id}");
            provider.get_posts(&token, &author_urn, 5).await
                .map(|posts| serde_json::json!({"analytics_summary": posts}))
                .map_err(|e| e.to_string())
        }
        LinkedinAction::Reply { comment_id, content } => {
            let post = crate::social::PostContent {
                content,
                media: vec![],
                settings: serde_json::Value::Object(serde_json::Map::new()),
            in_reply_to: None,
            };
            provider.reply_to_comment(&token, &comment_id, &post).await
                .map(|r| serde_json::json!({"id": r.platform_post_id, "status": r.status}))
                .map_err(|e| e.to_string())
        }
        LinkedinAction::Dm { recipient, content } => {
            let post = crate::social::PostContent {
                content,
                media: vec![],
                settings: serde_json::Value::Object(serde_json::Map::new()),
            in_reply_to: None,
            };
            provider.send_dm(&token, &recipient, &post).await
                .map(|r| serde_json::json!({"id": r.platform_post_id, "status": r.status}))
                .map_err(|e| e.to_string())
        }
        LinkedinAction::DmList { count } => {
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
        LinkedinAction::DmMessages { conversation_id, count } => {
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
