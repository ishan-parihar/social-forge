use crate::api::AppState;
use crate::cli::TelegramBotAction;

pub async fn handle(action: TelegramBotAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        TelegramBotAction::Send { bot_index, chat_id, text } => {
            let input = crate::mcp::tools_telegram_bot::TbSendMessageInput {
                token_index: bot_index, chat_id, text,
            };
            crate::mcp::tools_telegram_bot::handle_tb_send_message(state, &input).await.map(|v| v.0.data)
        }
        TelegramBotAction::Photo { bot_index, chat_id, url, caption } => {
            let input = crate::mcp::tools_telegram_bot::TbSendPhotoInput {
                token_index: bot_index, chat_id, photo_url: url, caption,
            };
            crate::mcp::tools_telegram_bot::handle_tb_send_photo(state, &input).await.map(|v| v.0.data)
        }
        TelegramBotAction::Document { bot_index, chat_id, path, caption } => {
            let input = crate::mcp::tools_telegram_bot::TbSendDocumentInput {
                token_index: bot_index, chat_id, document_url: path, caption,
            };
            crate::mcp::tools_telegram_bot::handle_tb_send_document(state, &input).await.map(|v| v.0.data)
        }
        TelegramBotAction::Chat { bot_index, chat_id } => {
            let input = crate::mcp::tools_telegram_bot::TbChatInput { token_index: bot_index, chat_id };
            crate::mcp::tools_telegram_bot::handle_tb_get_chat(state, &input).await.map(|v| v.0.data)
        }
        TelegramBotAction::Updates { bot_index } => {
            let input = crate::mcp::tools_telegram_bot::TbGetUpdatesInput { token_index: bot_index };
            crate::mcp::tools_telegram_bot::handle_tb_get_updates(state, &input).await.map(|v| serde_json::to_value(v.0).unwrap_or_default())
        }
    };
    match result {
        Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
        Err(e) => { eprintln!("{}", serde_json::json!({"error": e})); std::process::exit(1); }
    }
    Ok(())
}
