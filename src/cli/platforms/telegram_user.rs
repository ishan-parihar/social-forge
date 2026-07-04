use crate::api::AppState;
use crate::cli::TelegramUserAction;

pub async fn handle(action: TelegramUserAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        TelegramUserAction::Send { peer, text } => {
            let input = crate::mcp::tools_telegram_user::TuSendMessageInput { peer, text };
            crate::mcp::tools_telegram_user::handle_tu_send_message(state, &input).await.map(|v| v.0.data)
        }
        TelegramUserAction::Dialogs { limit } => {
            let input = crate::mcp::tools_telegram_user::TuListDialogsInput { limit: Some(limit) };
            crate::mcp::tools_telegram_user::handle_tu_list_dialogs(state, &input).await.map(|v| v.0.data)
        }
        TelegramUserAction::Contacts => {
            let input = crate::mcp::tools_telegram_user::TuListContactsInput { query: None };
            crate::mcp::tools_telegram_user::handle_tu_list_contacts(state, &input).await.map(|v| v.0.data)
        }
        TelegramUserAction::Search { query } => {
            let input = crate::mcp::tools_telegram_user::TuSearchInput { query };
            crate::mcp::tools_telegram_user::handle_tu_search(state, &input).await.map(|v| v.0.data)
        }
    };

    super::emit_result(result)
}
