// ─── MCP Media Tools ───────────────────────────────────────────
// Tool handlers for media upload via MCP protocol.
// Allows AI agents to upload images/videos for post attachments.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MediaUploadInput {
    pub filename: String,
    pub mime_type: String,
    pub data: String, // base64-encoded file data
    pub alt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MediaUploadOutput {
    pub id: String,
    pub url: String,
    pub mime_type: String,
    pub filename: String,
    pub size: i64,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MediaListInput {
    pub limit: Option<i64>,
    pub search: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MediaItem {
    pub id: String,
    pub url: String,
    pub filename: String,
    pub mime_type: String,
    pub size: i64,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MediaListOutput {
    pub media: Vec<MediaItem>,
    pub total: i64,
}

pub async fn upload_media(
    state: &AppState,
    input: &MediaUploadInput,
) -> Result<Json<MediaUploadOutput>, String> {
    let user_id = resolve_first_user(state).await?;

    let data = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &input.data,
    )
    .map_err(|e| format!("Invalid base64 data: {e}"))?;

    if data.len() > 50 * 1024 * 1024 {
        return Err("File too large (max 50 MB)".into());
    }

    let file_id = Uuid::new_v4();
    let ext = std::path::Path::new(&input.filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("bin");
    let filename = format!("{file_id}.{ext}");

    let upload_dir = std::path::Path::new(&state.config.media_dir);
    tokio::fs::create_dir_all(upload_dir)
        .await
        .map_err(|e| format!("Failed to create upload dir: {e}"))?;

    let filepath = upload_dir.join(&filename);
    tokio::fs::write(&filepath, &data)
        .await
        .map_err(|e| format!("Failed to write file: {e}"))?;

    let entry = crate::db::queries::create_media(
        &state.db,
        user_id,
        &input.filename,
        &filename,
        &input.mime_type,
        data.len() as i64,
        None,
        None,
    )
    .await
    .map_err(|e| format!("DB error: {e}"))?;

    Ok(Json(MediaUploadOutput {
        id: entry.id.to_string(),
        url: format!("/api/media/{}", entry.id),
        mime_type: input.mime_type.clone(),
        filename: input.filename.clone(),
        size: data.len() as i64,
    }))
}

pub async fn list_media(
    state: &AppState,
    input: &MediaListInput,
) -> Result<Json<MediaListOutput>, String> {
    let user_id = resolve_first_user(state).await?;
    let limit = input.limit.unwrap_or(50).min(200);

    let entries = crate::db::queries::list_media(
        &state.db,
        user_id,
        limit,
        0,
        input.search.as_deref(),
    )
    .await
    .map_err(|e| format!("DB error: {e}"))?;

    let total = entries.len() as i64;
    let media = entries.into_iter().map(|e| MediaItem {
        id: e.id.to_string(),
        url: format!("/api/media/{}", e.id),
        filename: e.original_name,
        mime_type: e.mime_type,
        size: e.file_size,
        created_at: e.created_at.to_rfc3339(),
    }).collect();

    Ok(Json(MediaListOutput { media, total }))
}

async fn resolve_first_user(state: &AppState) -> Result<Uuid, String> {
    let user = sqlx::query_scalar::<_, Uuid>(
        "SELECT u.id FROM users u \
         WHERE EXISTS (SELECT 1 FROM integrations i WHERE i.user_id = u.id) \
         LIMIT 1"
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    if let Some(id) = user {
        return Ok(id);
    }

    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users LIMIT 1")
        .fetch_optional(&state.db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No user registered".to_string())
}
