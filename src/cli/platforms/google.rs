use crate::api::AppState;
use crate::cli::GoogleAction;

pub async fn handle(action: GoogleAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        GoogleAction::YoutubeSearch { channel_id, query, limit } => {
            let input = crate::mcp::tools_google::YtSearchVideosInput {
                channel_id, query, max_results: Some(limit),
            };
            crate::mcp::tools_google::handle_goog_search_videos(state, &input).await.map(|v| v.0)
        }
        GoogleAction::Video { channel_id, video_id } => {
            let input = crate::mcp::tools_google::YtGetVideoInput { channel_id, video_id };
            crate::mcp::tools_google::handle_goog_get_video(state, &input).await.map(|v| v.0)
        }
        GoogleAction::Playlists { channel_id, limit } => {
            let input = crate::mcp::tools_google::YtListPlaylistsInput {
                channel_id, max_results: Some(limit),
            };
            crate::mcp::tools_google::handle_goog_get_playlists(state, &input).await.map(|v| v.0)
        }
        GoogleAction::ChannelStats { channel_id } => {
            let input = crate::mcp::tools_google::YtGetChannelStatsInput { channel_id };
            crate::mcp::tools_google::handle_goog_get_channel_stats(state, &input).await.map(|v| v.0)
        }
    };

    super::emit_result(result)
}
