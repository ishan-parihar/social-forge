// ── Facebook CLI Handler ──────────────────────────────────────
// Relocated from cli/run.rs to match the platforms/*.rs shim pattern.

use crate::api::AppState;
use crate::cli::FacebookAction;
use crate::cli::platforms::emit_result;
use crate::cli::run::{find_facebook_page_token, resolve_user};

pub async fn handle(action: FacebookAction, state: &AppState) -> anyhow::Result<()> {
    let user_id = resolve_user(state).await?;

    let result: Result<serde_json::Value, String> = match action {
        FacebookAction::Posts { page_id } => {
            let input = crate::mcp::tools_facebook::FbGetFeedInput { page_id, limit: Some(20), since: None, until: None };
            crate::mcp::tools_facebook::handle_fb_get_feed(state, &input).await
                .map(|v| v.0).map_err(|e| e.to_string())
        }
        FacebookAction::Insights { page_id, metric } => {
            let input = crate::mcp::tools_facebook::FbPageInsightsInput {
                page_id, metric, period: Some("day".to_string()), since: None, until: None,
            };
            crate::mcp::tools_facebook::handle_fb_page_insights(state, &input).await
                .map(|v| v.0).map_err(|e| e.to_string())
        }
        FacebookAction::Comment { post_id, text } => {
            let page_id = post_id.split('_').next().unwrap_or(&post_id).to_string();
            let input = crate::mcp::tools_facebook::FbCommentInput {
                page_id, post_id, message: text,
            };
            crate::mcp::tools_facebook::handle_fb_comment(state, &input).await
                .map(|v| v.0).map_err(|e| e.to_string())
        }
        FacebookAction::Create { page_id, message } => {
            let input = crate::mcp::tools_facebook::FbCreatePostInput {
                page_id, message, link: None,
            };
            crate::mcp::tools_facebook::handle_fb_create_post(state, &input).await
                .map(|v| v.0).map_err(|e| e.to_string())
        }
        FacebookAction::Photo { page_id, url, caption } => {
            let input = crate::mcp::tools_facebook::FbCreatePhotoInput {
                page_id, url, caption,
            };
            crate::mcp::tools_facebook::handle_fb_create_photo(state, &input).await
                .map(|v| v.0).map_err(|e| e.to_string())
        }
        FacebookAction::Video { page_id, url, title } => {
            let input = crate::mcp::tools_facebook::FbCreateVideoInput {
                page_id, file_url: url, title, description: None,
            };
            crate::mcp::tools_facebook::handle_fb_create_video(state, &input).await
                .map(|v| v.0).map_err(|e| e.to_string())
        }
        FacebookAction::Delete { post_id } => {
            let page_id = post_id.split('_').next().unwrap_or(&post_id);
            match find_facebook_page_token(state, user_id, page_id).await {
                Err(e) => Err(e.to_string()),
                Ok(_) => {
                    let input = crate::mcp::tools_facebook::FbDeletePostInput {
                        page_id: page_id.to_string(), post_id,
                    };
                    crate::mcp::tools_facebook::handle_fb_delete_post(state, &input).await
                        .map(|v| v.0).map_err(|e| e.to_string())
                }
            }
        }
        FacebookAction::React { post_id, reaction_type } => {
            let page_id = post_id.split('_').next().unwrap_or(&post_id);
            match find_facebook_page_token(state, user_id, page_id).await {
                Err(e) => Err(e.to_string()),
                Ok(_) => {
                    let input = crate::mcp::tools_facebook::FbReactInput {
                        page_id: page_id.to_string(), post_id, reaction_type,
                    };
                    crate::mcp::tools_facebook::handle_fb_react(state, &input).await
                        .map(|v| v.0).map_err(|e| e.to_string())
                }
            }
        }
        FacebookAction::Conversations { page_id } => {
            let input = crate::mcp::tools_facebook::FbConversationsInput { page_id };
            crate::mcp::tools_facebook::handle_fb_conversations(state, &input).await
                .map(|v| v.0).map_err(|e| e.to_string())
        }
        FacebookAction::Send { page_id, conversation_id, text } => {
            let input = crate::mcp::tools_facebook::FbSendMessageInput {
                page_id, conversation_id, message: text,
            };
            crate::mcp::tools_facebook::handle_fb_send_message(state, &input).await
                .map(|v| v.0).map_err(|e| e.to_string())
        }
        FacebookAction::Pages { query } => {
            let input = crate::mcp::tools_facebook::FbSearchPagesInput { query };
            crate::mcp::tools_facebook::handle_fb_search_pages(state, &input).await
                .map(|v| v.0).map_err(|e| e.to_string())
        }
        FacebookAction::PageInsights { page_id, metric } => {
            let input = crate::mcp::tools_facebook::FbPageInsightsInput {
                page_id, metric, period: None, since: None, until: None,
            };
            crate::mcp::tools_facebook::handle_fb_page_insights(state, &input).await
                .map(|v| v.0).map_err(|e| e.to_string())
        }
        FacebookAction::Albums { page_id } => {
            let input = crate::mcp::tools_facebook::FbAlbumsInput { page_id };
            crate::mcp::tools_facebook::handle_fb_albums(state, &input).await
                .map(|v| v.0).map_err(|e| e.to_string())
        }
    };

    emit_result(result)
}
