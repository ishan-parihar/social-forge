use crate::api::AppState;
use crate::cli::MediumBlogAction;

pub async fn handle(action: MediumBlogAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        MediumBlogAction::Post { title, content, tags } => {
            let input = crate::mcp::tools_medium::MdCreatePostInput {
                title, content, tags, publish_status: None,
            };
            crate::mcp::tools_medium::handle_md_create_post(state, &input).await.map(|v| v.0)
        }
        MediumBlogAction::List => {
            let input = crate::mcp::tools_medium::MdListPostsInput {  };
            crate::mcp::tools_medium::handle_md_list_posts(state, &input).await.map(|v| v.0)
        }
        MediumBlogAction::Get { id } => {
            let input = crate::mcp::tools_medium::MdGetPostInput { post_id: id };
            crate::mcp::tools_medium::handle_md_get_post(state, &input).await.map(|v| v.0)
        }
    };
    match result {
        Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
        Err(e) => { eprintln!("{}", serde_json::json!({"error": e})); std::process::exit(1); }
    }
    Ok(())
}
