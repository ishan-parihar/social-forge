use crate::api::AppState;
use crate::cli::WordpressAction;

pub async fn handle(action: WordpressAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        WordpressAction::Post { title, content, status } => {
            let input = crate::mcp::tools_wordpress::WpCreatePostInput {
                title, content, status, categories: None, tags: None,
            };
            crate::mcp::tools_wordpress::handle_wp_create_post(state, &input).await.map(|v| v.0)
        }
        WordpressAction::List { status, limit } => {
            let input = crate::mcp::tools_wordpress::WpListPostsInput {
                status, per_page: Some(limit as i32),
            };
            crate::mcp::tools_wordpress::handle_wp_list_posts(state, &input).await.map(|v| v.0)
        }
        WordpressAction::Get { id } => {
            let input = crate::mcp::tools_wordpress::WpGetPostInput { post_id: id };
            crate::mcp::tools_wordpress::handle_wp_get_post(state, &input).await.map(|v| v.0)
        }
        WordpressAction::Categories => {
            let input = crate::mcp::tools_wordpress::WpListCategoriesInput {  };
            crate::mcp::tools_wordpress::handle_wp_list_categories(state, &input).await.map(|v| v.0)
        }
    };
    match result {
        Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
        Err(e) => { eprintln!("{}", serde_json::json!({"error": e})); std::process::exit(1); }
    }
    Ok(())
}
