use crate::api::AppState;
use crate::cli::AnalyticsAction;

pub async fn handle(action: AnalyticsAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        AnalyticsAction::Get { provider, days } => {
            let input = crate::mcp::tools_analytics::AnalyticsGetInput {
                provider, days: Some(days),
            };
            crate::mcp::tools_analytics::handle_analytics_get(state, &input).await.map(|v| v.0)
        }
        AnalyticsAction::Post { post_id } => {
            let input = crate::mcp::tools_analytics::AnalyticsPostInput {
                post_id,
            };
            crate::mcp::tools_analytics::handle_analytics_get_post(state, &input).await.map(|v| v.0)
        }
    };

    super::emit_result(result)
}
