use crate::api::AppState;
use crate::cli::HashnodeAction;

pub async fn handle(action: HashnodeAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        HashnodeAction::Post { publication_id, title, content } => {
            let input = crate::mcp::tools_hashnode::HnCreatePostInput {
                publication_id, title, content, tags: None, canonical_url: None,
            };
            crate::mcp::tools_hashnode::handle_hn_create_post(state, &input).await.map(|v| v.0)
        }
        HashnodeAction::List { publication_id, page } => {
            let input = crate::mcp::tools_hashnode::HnListPostsInput {
                publication_id, page,
            };
            crate::mcp::tools_hashnode::handle_hn_list_posts(state, &input).await.map(|v| v.0)
        }
        HashnodeAction::Get { post_id } => {
            let input = crate::mcp::tools_hashnode::HnGetPostInput { post_id };
            crate::mcp::tools_hashnode::handle_hn_get_post(state, &input).await.map(|v| v.0)
        }
    };

    super::emit_result(result)
}
