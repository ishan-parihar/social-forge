use crate::api::AppState;
use crate::cli::DriveAction;

pub async fn handle(action: DriveAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        DriveAction::Files { limit } => {
            let input = crate::mcp::tools_google::DrListFilesInput { max_results: Some(limit), mime_type: None };
            crate::mcp::tools_google::handle_goog_list_files(state, &input).await.map(|v| v.0)
        }
        DriveAction::File { file_id } => {
            let input = crate::mcp::tools_google::DrGetFileInput { file_id };
            crate::mcp::tools_google::handle_goog_get_file(state, &input).await.map(|v| v.0)
        }
        DriveAction::Search { query, limit } => {
            let input = crate::mcp::tools_google::DrSearchFilesInput { query, max_results: Some(limit) };
            crate::mcp::tools_google::handle_goog_search_files(state, &input).await.map(|v| v.0)
        }
        DriveAction::Folders { limit } => {
            let input = crate::mcp::tools_google::DrListFoldersInput { max_results: Some(limit) };
            crate::mcp::tools_google::handle_goog_list_folders(state, &input).await.map(|v| v.0)
        }
        DriveAction::Metadata { file_id } => {
            let input = crate::mcp::tools_google::DrGetFileMetadataInput { file_id };
            crate::mcp::tools_google::handle_goog_get_file_metadata(state, &input).await.map(|v| v.0)
        }
        DriveAction::Export { file_id, mime_type } => {
            let input = crate::mcp::tools_google::DrExportFileInput { file_id, mime_type };
            crate::mcp::tools_google::handle_goog_export_file(state, &input).await.map(|v| v.0)
        }
    };
    match result {
        Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
        Err(e) => { eprintln!("{}", serde_json::json!({"error": e})); std::process::exit(1); }
    }
    Ok(())
}
