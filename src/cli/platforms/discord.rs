use crate::api::AppState;
use crate::cli::DiscordAction;

pub async fn handle(action: DiscordAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        DiscordAction::Channels { guild_id } => {
            let input = crate::mcp::tools_discord::DiGetGuildChannelsInput { guild_id };
            crate::mcp::tools_discord::handle_di_get_guild_channels(state, &input).await.map(|v| v.0)
        }
        DiscordAction::Messages { channel_id, limit } => {
            let input = crate::mcp::tools_discord::DiGetMessagesInput { channel_id, limit: Some(limit) };
            crate::mcp::tools_discord::handle_di_get_messages(state, &input).await.map(|v| v.0)
        }
        DiscordAction::Send { channel_id, text } => {
            let input = crate::mcp::tools_discord::DiSendMessageInput { channel_id, content: text };
            crate::mcp::tools_discord::handle_di_send_message(state, &input).await.map(|v| v.0)
        }
        DiscordAction::Server { guild_id } => {
            let input = crate::mcp::tools_discord::DiGetServerInfoInput { guild_id };
            crate::mcp::tools_discord::handle_di_get_server_info(state, &input).await.map(|v| v.0)
        }
        DiscordAction::Forum { channel_id, title, content } => {
            let input = crate::mcp::tools_discord::DiCreateForumPostInput {
                channel_id, name: title, content, applied_tags: vec![],
            };
            crate::mcp::tools_discord::handle_di_create_forum_post(state, &input).await.map(|v| v.0)
        }
    };
    match result {
        Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
        Err(e) => { eprintln!("{}", serde_json::json!({"error": e})); std::process::exit(1); }
    }
    Ok(())
}
