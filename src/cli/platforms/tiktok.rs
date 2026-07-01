use crate::api::AppState;
use crate::cli::TiktokAction;

pub async fn handle(action: TiktokAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        TiktokAction::Profile => {
            let input = crate::mcp::tools_tiktok::TtProfileInput { token: String::new() };
            crate::mcp::tools_tiktok::handle_tt_profile(state, &input)
                .await
                .map(|v| v.0)
        }
        TiktokAction::Post { text, video_url } => {
            let input = crate::mcp::tools_tiktok::TtCreatePostInput {
                token: String::new(),
                text,
                video_data: None,
                video_url,
            };
            crate::mcp::tools_tiktok::handle_tt_create_post(state, &input)
                .await
                .map(|v| v.0)
        }
        TiktokAction::Videos { limit } => {
            let input = crate::mcp::tools_tiktok::TtListVideosInput {
                token: String::new(),
                max_count: Some(limit),
            };
            crate::mcp::tools_tiktok::handle_tt_list_videos(state, &input)
                .await
                .map(|v| v.0)
        }
    };
    match result {
        Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
        Err(e) => { eprintln!("{}", serde_json::json!({"error": e})); std::process::exit(1); }
    }
    Ok(())
}
