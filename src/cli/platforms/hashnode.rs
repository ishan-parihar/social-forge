use crate::api::AppState;
use crate::cli::HashnodeAction;

pub async fn handle(action: HashnodeAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        HashnodeAction::Post { publication_id, title, content } => {
            let input = crate::mcp::tools_hashnode::HnCreatePostInput {
                token: String::new(), publication_id, title, content, tags: None, canonical_url: None,
            };
            crate::mcp::tools_hashnode::handle_hn_create_post(state, &input).await.map(|v| v.0)
        }
        HashnodeAction::List { publication_id, page } => {
            let input = crate::mcp::tools_hashnode::HnListPostsInput {
                token: String::new(), publication_id, page,
            };
            crate::mcp::tools_hashnode::handle_hn_list_posts(state, &input).await.map(|v| v.0)
        }
        HashnodeAction::Get { post_id } => {
            let input = crate::mcp::tools_hashnode::HnGetPostInput { token: String::new(), post_id };
            crate::mcp::tools_hashnode::handle_hn_get_post(state, &input).await.map(|v| v.0)
        }
    };
    match result {
        Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
        Err(e) => { eprintln!("{}", serde_json::json!({"error": e})); std::process::exit(1); }
    }
    Ok(())
}
