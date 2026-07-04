use crate::api::AppState;
use crate::cli::SlackAction;

pub async fn handle(action: SlackAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        SlackAction::Channels => {
            let input = crate::mcp::tools_slack::SlListChannelsInput {  };
            crate::mcp::tools_slack::handle_sl_list_channels(state, &input).await.map(|v| v.0)
        }
        SlackAction::History { channel_id, limit } => {
            let input = crate::mcp::tools_slack::SlChannelHistoryInput {
                channel: channel_id, limit: Some(limit),
            };
            crate::mcp::tools_slack::handle_sl_channel_history(state, &input).await.map(|v| v.0)
        }
        SlackAction::Send { channel_id, text } => {
            let input = crate::mcp::tools_slack::SlSendMessageInput {
                channel: channel_id, content: text,
            };
            crate::mcp::tools_slack::handle_sl_send_message(state, &input).await.map(|v| v.0)
        }
        SlackAction::Users => {
            let input = crate::mcp::tools_slack::SlListUsersInput {  };
            crate::mcp::tools_slack::handle_sl_list_users(state, &input).await.map(|v| v.0)
        }
    };

    super::emit_result(result)
}
