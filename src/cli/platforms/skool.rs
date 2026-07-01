use crate::api::AppState;
use crate::cli::SkoolAction;

pub async fn handle(action: SkoolAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        SkoolAction::Post { group_id, title, content, label } => {
            let input = crate::mcp::tools_skool::SkPublishInput { group_id, title, content, label };
            crate::mcp::tools_skool::handle_sk_publish(state, &input).await.map(|v| v.0)
        }
        SkoolAction::Info { community_slug } => {
            let input = crate::mcp::tools_skool::SkGetInfoInput { community_slug };
            crate::mcp::tools_skool::handle_sk_get_info(state, &input).await.map(|v| v.0)
        }
        SkoolAction::Posts { community_slug, page, sort } => {
            let input = crate::mcp::tools_skool::SkListPostsInput { community_slug, page, sort, category: None };
            crate::mcp::tools_skool::handle_sk_list_posts(state, &input).await.map(|v| v.0)
        }
        SkoolAction::Comment { post_id, group_id, content } => {
            let input = crate::mcp::tools_skool::SkCreateCommentInput { post_id, group_id, content };
            crate::mcp::tools_skool::handle_sk_create_comment(state, &input).await.map(|v| v.0)
        }
    };
    match result {
        Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
        Err(e) => { eprintln!("{}", serde_json::json!({"error": e})); std::process::exit(1); }
    }
    Ok(())
}
