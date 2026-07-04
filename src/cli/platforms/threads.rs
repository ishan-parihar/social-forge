use crate::api::AppState;
use crate::cli::ThreadsAction;

pub async fn handle(action: ThreadsAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        ThreadsAction::Profile { threads_id } => {
            let input = crate::mcp::tools_threads::ThreadsGetProfileInput { threads_id };
            crate::mcp::tools_threads::handle_th_get_profile(state, &input).await.map(|v| v.0)
        }
        ThreadsAction::List { threads_id, limit } => {
            let input = crate::mcp::tools_threads::ThreadsGetThreadsInput { threads_id, limit: Some(limit) };
            crate::mcp::tools_threads::handle_th_get_threads(state, &input).await.map(|v| v.0)
        }
        ThreadsAction::Post { threads_id, text, media_url } => {
            let input = crate::mcp::tools_threads::ThreadsCreateThreadInput {
                threads_id, text, media_url, media_type: None,
            };
            crate::mcp::tools_threads::handle_th_create_thread(state, &input).await.map(|v| v.0)
        }
        ThreadsAction::Reply { threads_id, media_id, text } => {
            let input = crate::mcp::tools_threads::ThreadsReplyToThreadInput {
                threads_id, media_id, message: text,
            };
            crate::mcp::tools_threads::handle_th_reply_to_thread(state, &input).await.map(|v| v.0)
        }
        ThreadsAction::Delete { threads_id, media_id } => {
            let input = crate::mcp::tools_threads::ThreadsDeleteThreadInput { threads_id, media_id };
            crate::mcp::tools_threads::handle_th_delete_thread(state, &input).await.map(|v| v.0)
        }
        ThreadsAction::Insights { threads_id, metric, period } => {
            let input = crate::mcp::tools_threads::ThreadsGetInsightsInput { threads_id, metric, period };
            crate::mcp::tools_threads::handle_th_get_insights(state, &input).await.map(|v| v.0)
        }
    };

    super::emit_result(result)
}
