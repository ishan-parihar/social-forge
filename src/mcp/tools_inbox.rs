// ─── E3: Unified Inbox ───────────────────────────────────────────
// Comments + DMs across every connected provider in ONE list, with ONE
// reply verb that dispatches to reply_to_comment (comments) or send_dm
// (DMs). All reads go through the already-normalized SocialProvider
// trait types (CommentData / DmConversation), so no per-provider
// parsing lives here.

use rmcp::{schemars::JsonSchema, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::social::{CommentData, PostContent};

use super::tools_posts::resolve_first_user;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct InboxListInput {
    /// Max merged items to return (default 50)
    pub limit: Option<usize>,
    /// Restrict to these provider identifiers (e.g. ["x", "reddit"]); all when omitted
    pub providers: Option<Vec<String>>,
    /// Include DM conversations alongside comments (default true)
    pub include_dms: Option<bool>,
    /// How many recent published posts per account to sweep for comments (default 3)
    pub posts_per_account: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct InboxReplyInput {
    /// Provider of the item being answered (from inbox_list: item.provider)
    pub provider: String,
    /// Optional account selector (UUID or provider:name) for multi-account setups
    pub integration: Option<String>,
    /// "comment" or "dm" — from inbox_list: item.kind
    pub kind: String,
    /// For comments: the comment_id from inbox_list
    pub comment_id: Option<String>,
    /// For DMs: the recipient id (conversation participant) from inbox_list
    pub recipient: Option<String>,
    pub text: String,
}

/// Flatten a threaded comment tree into items, depth-first.
fn flatten_comments(
    c: &CommentData,
    provider: &str,
    integration_id: Uuid,
    post_id: &str,
    out: &mut Vec<serde_json::Value>,
) {
    out.push(serde_json::json!({
        "kind": "comment",
        "provider": provider,
        "integration_id": integration_id.to_string(),
        "post_id": post_id,
        "comment_id": c.id,
        "author": c.author_name,
        "text": c.text,
        "created_at": c.created_at.to_rfc3339(),
        "like_count": c.like_count,
    }));
    for r in &c.replies {
        flatten_comments(r, provider, integration_id, post_id, out);
    }
}

pub async fn handle_inbox_list(
    state: &AppState,
    input: &InboxListInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_first_user(state).await?;
    let limit = input.limit.unwrap_or(50).max(1);
    let posts_per_account = input.posts_per_account.unwrap_or(3).clamp(1, 10) as i64;
    let include_dms = input.include_dms.unwrap_or(true);

    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let mut items: Vec<serde_json::Value> = Vec::new();
    let mut fetched = serde_json::Map::new();
    let mut errors: Vec<String> = Vec::new();

    for integration in &integrations {
        if integration.disabled {
            continue;
        }
        if let Some(providers) = &input.providers {
            if !providers.is_empty() && !providers.contains(&integration.provider_identifier) {
                continue;
            }
        }
        let Some(provider) = state.providers.get(&integration.provider_identifier) else {
            continue;
        };
        let token =
            crate::crypto::maybe_decrypt_token(&integration.access_token, state.token_key.as_ref());
        let iid = integration.id;

        let mut comments_n = 0usize;
        let posts = crate::db::queries::recent_platform_post_ids(
            &state.db,
            user_id,
            iid,
            posts_per_account,
        )
        .await
        .unwrap_or_default();
        for (pid,) in &posts {
            match provider.get_post_comments(&token, pid).await {
                Ok(comments) => {
                    for c in &comments {
                        flatten_comments(c, &integration.provider_identifier, iid, pid, &mut items);
                        comments_n += 1 + c.replies.len();
                    }
                }
                Err(e) => errors.push(format!(
                    "{} comments on {}: {e}",
                    integration.provider_identifier, pid
                )),
            }
        }

        let mut dms_n = 0usize;
        if include_dms {
            match provider.get_dm_conversations(&token, 20).await {
                Ok(convs) => {
                    for c in convs {
                        items.push(serde_json::json!({
                            "kind": "dm",
                            "provider": integration.provider_identifier,
                            "integration_id": iid.to_string(),
                            "conversation_id": c.id,
                            "author": c.participant_name.as_ref().unwrap_or(&c.participant),
                            "recipient": c.participant,
                            "text": c.last_message,
                            "created_at": c.last_message_at.map(|t| t.to_rfc3339()),
                            "unread": c.unread_count,
                        }));
                        dms_n += 1;
                    }
                }
                Err(e) => errors.push(format!("{} DMs: {e}", integration.provider_identifier)),
            }
        }

        fetched.insert(
            integration.provider_identifier.clone(),
            serde_json::json!({ "comments": comments_n, "dms": dms_n, "posts_swept": posts.len() }),
        );
    }

    // Newest first; items without a timestamp sink to the bottom.
    items.sort_by(|a, b| {
        let ta = a["created_at"].as_str().unwrap_or("");
        let tb = b["created_at"].as_str().unwrap_or("");
        tb.cmp(ta)
    });
    items.truncate(limit);

    Ok(Json(serde_json::json!({
        "items": items,
        "fetched": serde_json::Value::Object(fetched),
        "errors": errors,
        "note": "inbox_reply answers any item: pass provider + kind + comment_id (comments) or recipient (DMs).",
    })))
}

pub async fn handle_inbox_reply(
    state: &AppState,
    input: &InboxReplyInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = resolve_first_user(state).await?;
    let integration = super::tools_posts::select_integration(
        state,
        user_id,
        &input.provider,
        input.integration.as_deref(),
    )
    .await?;
    let Some(provider) = state.providers.get(&integration.provider_identifier) else {
        return Err(format!(
            "Provider '{}' not found",
            integration.provider_identifier
        ));
    };
    let token =
        crate::crypto::maybe_decrypt_token(&integration.access_token, state.token_key.as_ref());

    let content = PostContent {
        content: input.text.clone(),
        ..Default::default()
    };

    let result = match input.kind.as_str() {
        "comment" => {
            let comment_id = input
                .comment_id
                .as_deref()
                .filter(|s| !s.is_empty())
                .ok_or("comment_id is required to reply to a comment")?;
            provider
                .reply_to_comment(&token, comment_id, &content)
                .await
                .map_err(|e| format!("Reply failed: {e}"))?
        }
        "dm" => {
            let recipient = input
                .recipient
                .as_deref()
                .filter(|s| !s.is_empty())
                .ok_or("recipient is required to reply to a DM")?;
            provider
                .send_dm(&token, recipient, &content)
                .await
                .map_err(|e| format!("DM failed: {e}"))?
        }
        other => return Err(format!("kind must be 'comment' or 'dm', got '{other}'")),
    };

    Ok(Json(serde_json::json!({
        "provider": integration.provider_identifier,
        "kind": input.kind,
        "platform_post_id": result.platform_post_id,
        "platform_post_url": result.platform_post_url,
        "status": result.status,
    })))
}

#[cfg(test)]
mod inbox_tests {
    use super::*;

    fn comment(id: &str, text: &str, replies: Vec<CommentData>) -> CommentData {
        CommentData {
            id: id.into(),
            author_name: Some(format!("author-{id}")),
            author_avatar: None,
            text: text.into(),
            created_at: chrono::Utc::now(),
            like_count: 0,
            replies,
        }
    }

    #[test]
    fn flatten_expands_threaded_replies_depth_first() {
        let tree = comment(
            "root",
            "top",
            vec![
                comment("r1", "first", vec![]),
                comment("r2", "second", vec![comment("r2a", "nested", vec![])]),
            ],
        );
        let mut out = Vec::new();
        flatten_comments(&tree, "x", Uuid::new_v4(), "post-1", &mut out);
        let ids: Vec<&str> = out
            .iter()
            .map(|v| v["comment_id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, vec!["root", "r1", "r2", "r2a"]);
        assert!(out
            .iter()
            .all(|v| v["kind"] == "comment" && v["post_id"] == "post-1"));
    }
}
