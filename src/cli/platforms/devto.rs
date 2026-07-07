use crate::api::AppState;
use crate::cli::DevtoAction;

pub async fn handle(action: DevtoAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        DevtoAction::Post { title, content, tags, publish } => {
            let input = crate::mcp::tools_devto::DvCreatePostInput {
                title, content, tags, published: Some(publish),
            };
            crate::mcp::tools_devto::handle_dv_create_post(state, &input).await.map(|v| v.0)
        }
        DevtoAction::List => {
            let input = crate::mcp::tools_devto::DvListPostsInput {  };
            crate::mcp::tools_devto::handle_dv_list_posts(state, &input).await.map(|v| v.0)
        }
        DevtoAction::Get { id } => {
            let input = crate::mcp::tools_devto::DvGetPostInput { article_id: id };
            crate::mcp::tools_devto::handle_dv_get_post(state, &input).await.map(|v| v.0)
        }
    };

    super::emit_result(result)
}
