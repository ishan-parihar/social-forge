// ── Instagram CLI Handler ─────────────────────────────────────
// Relocated from cli/run.rs to match the platforms/*.rs shim pattern.

use crate::api::AppState;
use crate::cli::InstagramAction;
use crate::cli::platforms::emit_result;
use crate::cli::run::resolve_user;

pub async fn handle(action: InstagramAction, state: &AppState) -> anyhow::Result<()> {
    let user_id = resolve_user(state).await?;

    let result: Result<serde_json::Value, String> = match action {
        InstagramAction::Posts { account_id } => {
            let input = crate::mcp::tools_instagram::IgGetMediaInput {
                ig_id: account_id, limit: Some(20),
            };
            crate::mcp::tools_instagram::handle_ig_get_media(state, &input).await
                .map(|v| v.0).map_err(|e| e.to_string())
        }
        InstagramAction::Insights { account_id, metric } => {
            let input = crate::mcp::tools_instagram::IgGetInsightsInput {
                ig_id: account_id, metric, period: Some("day".to_string()),
            };
            crate::mcp::tools_instagram::handle_ig_get_insights(state, &input).await
                .map(|v| v.0).map_err(|e| e.to_string())
        }
        InstagramAction::Comment { media_id, text } => {
            match crate::db::queries::list_integrations(&state.db, user_id).await {
                Err(e) => Err(format!("DB error: {e}")),
                Ok(integrations) => {
                    let ig = integrations.iter()
                        .find(|i| i.provider_identifier == "instagram" || i.provider_identifier == "instagram-standalone");
                    match ig {
                        None => Err("No Instagram account connected".to_string()),
                        Some(ig) => {
                            let token = ig.access_token.clone();
                            let token = crate::crypto::maybe_decrypt_token(&token, state.token_key.as_ref());
                            let provider = crate::social::instagram::InstagramProvider::new(&state.config);
                            provider.reply_to_ig_comment(&token, &media_id, &text).await
                                .map_err(|e| e.to_string())
                        }
                    }
                }
            }
        }
        InstagramAction::Dm { account_id, recipient, content } => {
            let input = crate::mcp::tools_instagram::IgSendDmInput {
                ig_id: account_id, recipient_id: recipient, content,
            };
            crate::mcp::tools_instagram::handle_ig_send_dm(state, &input).await
                .map(|v| v.0).map_err(|e| e.to_string())
        }
        InstagramAction::DmList { account_id, count } => {
            let input = crate::mcp::tools_instagram::IgListConversationsInput {
                ig_id: account_id, limit: Some(count),
            };
            crate::mcp::tools_instagram::handle_ig_list_conversations(state, &input).await
                .map(|v| v.0).map_err(|e| e.to_string())
        }
        InstagramAction::DmMessages { account_id, conversation_id, count } => {
            let input = crate::mcp::tools_instagram::IgGetMessagesInput {
                ig_id: account_id, conversation_id, limit: Some(count),
            };
            crate::mcp::tools_instagram::handle_ig_get_messages(state, &input).await
                .map(|v| v.0).map_err(|e| e.to_string())
        }
        InstagramAction::MediaDetail { account_id, media_id } => {
            let input = crate::mcp::tools_instagram::IgGetMediaDetailInput {
                ig_id: account_id, media_id,
            };
            crate::mcp::tools_instagram::handle_ig_get_media_detail(state, &input).await
                .map(|v| v.0).map_err(|e| e.to_string())
        }
        InstagramAction::Hashtag { account_id, query } => {
            let input = crate::mcp::tools_instagram::IgSearchHashtagInput {
                ig_id: account_id, query,
            };
            crate::mcp::tools_instagram::handle_ig_search_hashtag(state, &input).await
                .map(|v| v.0).map_err(|e| e.to_string())
        }
        InstagramAction::Reels { account_id } => {
            let input = crate::mcp::tools_instagram::IgGetReelsInput {
                ig_id: account_id,
            };
            crate::mcp::tools_instagram::handle_ig_get_reels(state, &input).await
                .map(|v| v.0).map_err(|e| e.to_string())
        }
        InstagramAction::Stories { account_id } => {
            let input = crate::mcp::tools_instagram::IgGetStoriesInput {
                ig_id: account_id,
            };
            crate::mcp::tools_instagram::handle_ig_get_stories(state, &input).await
                .map(|v| v.0).map_err(|e| e.to_string())
        }
        InstagramAction::Followers { account_id } => {
            let input = crate::mcp::tools_instagram::IgGetFollowersInput {
                ig_id: account_id,
            };
            crate::mcp::tools_instagram::handle_ig_get_followers(state, &input).await
                .map(|v| v.0).map_err(|e| e.to_string())
        }
        InstagramAction::InsightsAudience { account_id } => {
            let input = crate::mcp::tools_instagram::IgGetInsightsAudienceInput {
                ig_id: account_id,
            };
            crate::mcp::tools_instagram::handle_ig_get_insights_audience(state, &input).await
                .map(|v| v.0).map_err(|e| e.to_string())
        }
        InstagramAction::Mentions { account_id } => {
            let input = crate::mcp::tools_instagram::IgGetMentionsInput {
                ig_id: account_id,
            };
            crate::mcp::tools_instagram::handle_ig_get_mentions(state, &input).await
                .map(|v| v.0).map_err(|e| e.to_string())
        }
        InstagramAction::PollContainer { account_id, creation_id } => {
            let input = crate::mcp::tools_instagram::IgPollContainerInput {
                ig_id: account_id, creation_id,
            };
            crate::mcp::tools_instagram::handle_ig_poll_container(state, &input).await
                .map(|v| v.0).map_err(|e| e.to_string())
        }
    };

    emit_result(result)
}
