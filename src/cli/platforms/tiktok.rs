use crate::api::AppState;
use crate::cli::TiktokAction;

pub async fn handle(action: TiktokAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        TiktokAction::Profile => {
            let input = crate::mcp::tools_tiktok::TtProfileInput {  };
            crate::mcp::tools_tiktok::handle_tt_profile(state, &input)
                .await
                .map(|v| v.0)
        }
        TiktokAction::Post { text, video_url } => {
            let input = crate::mcp::tools_tiktok::TtCreatePostInput {
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
                max_count: Some(limit),
            };
            crate::mcp::tools_tiktok::handle_tt_list_videos(state, &input)
                .await
                .map(|v| v.0)
        }
    };

    super::emit_result(result)
}
