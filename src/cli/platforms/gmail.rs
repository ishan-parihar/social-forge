use crate::api::AppState;
use crate::cli::GmailAction;

pub async fn handle(action: GmailAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        GmailAction::Profile => {
            crate::mcp::tools_google::handle_goog_get_profile(state, &()).await.map(|v| v.0)
        }
        GmailAction::Messages { limit, query } => {
            let input = crate::mcp::tools_google::GmListMessagesInput { max_results: Some(limit), query };
            crate::mcp::tools_google::handle_goog_list_messages(state, &input).await.map(|v| v.0)
        }
        GmailAction::Message { id } => {
            let input = crate::mcp::tools_google::GmGetMessageInput { message_id: id };
            crate::mcp::tools_google::handle_goog_get_message(state, &input).await.map(|v| v.0)
        }
        GmailAction::Send { to, subject, body } => {
            let input = crate::mcp::tools_google::GmSendMessageInput { to, subject, body };
            crate::mcp::tools_google::handle_goog_send_message(state, &input).await.map(|v| v.0)
        }
        GmailAction::Labels => {
            crate::mcp::tools_google::handle_goog_list_labels(state, &()).await.map(|v| v.0)
        }
        GmailAction::Thread { id } => {
            let input = crate::mcp::tools_google::GmGetThreadInput { thread_id: id };
            crate::mcp::tools_google::handle_goog_get_thread(state, &input).await.map(|v| v.0)
        }
        GmailAction::Search { query, limit } => {
            let input = crate::mcp::tools_google::GmSearchMessagesInput { query, max_results: Some(limit) };
            crate::mcp::tools_google::handle_goog_search_messages(state, &input).await.map(|v| v.0)
        }
    };

    super::emit_result(result)
}
