// ─── Media API Routes ─────────────────────────────────────────
// File upload and serving for post media attachments.

use axum::{
    body::{Body, HttpBody},
    extract::{Path, Query, State},
    http::{header, Response},
    Json,
};
use axum_extra::extract::Multipart;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::models::{MediaEntry, MediaPublic};
use crate::db::queries;
use crate::error::AppError;

use super::AppState;

#[derive(Debug, serde::Deserialize)]
pub struct ListMediaQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub search: Option<String>,
}

const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024; // 50 MB

/// POST /api/media — upload a file
pub async fn upload(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    mut multipart: Multipart,
) -> Result<Json<MediaPublic>, AppError> {
    let field = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Invalid multipart: {e}")))?
        .ok_or_else(|| AppError::BadRequest("No file uploaded".into()))?;

    let original_name = field
        .file_name()
        .unwrap_or("unnamed")
        .to_string();

    let mime_type = field
        .content_type()
        .unwrap_or("application/octet-stream")
        .to_string();

    let data = field.bytes().await.map_err(|e| AppError::BadRequest(format!("Read error: {e}")))?;

    if data.len() as u64 > MAX_FILE_SIZE {
        return Err(AppError::BadRequest("File too large (max 50 MB)".into()));
    }

    // Save to local filesystem
    let file_id = Uuid::new_v4();
    let ext = std::path::Path::new(&original_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("bin");
    let filename = format!("{file_id}.{ext}");
    let upload_dir = std::path::Path::new(&state.config.media_dir);
    tokio::fs::create_dir_all(upload_dir).await.map_err(|e| {
        AppError::Internal(format!("Failed to create upload dir: {e}"))
    })?;

    let filepath = upload_dir.join(&filename);
    tokio::fs::write(&filepath, &data).await.map_err(|e| {
        AppError::Internal(format!("Failed to write file: {e}"))
    })?;

    // Get dimensions for images
    let (width, height) = if mime_type.starts_with("image/") {
        detect_image_dimensions(&data)
    } else {
        (None, None)
    };

    let entry = queries::create_media(
        &state.db,
        auth.user_id,
        &original_name,
        &filename,
        &mime_type,
        data.len() as i64,
        width,
        height,
    )
    .await?;

    Ok(Json(MediaPublic::from(entry)))
}

/// GET /api/media — list user's media uploads
pub async fn list(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    axum::extract::Query(query): axum::extract::Query<ListMediaQuery>,
) -> Result<Json<Vec<MediaPublic>>, AppError> {
    let limit = query.limit.unwrap_or(50).min(200);
    let offset = query.offset.unwrap_or(0).max(0);
    let search = query.search.as_deref();
    let entries = queries::list_media(&state.db, auth.user_id, limit, offset, search).await?;
    Ok(Json(entries.into_iter().map(MediaPublic::from).collect()))
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let entry = queries::delete_media(&state.db, id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Media not found".into()))?;

    let upload_dir = std::path::Path::new(&state.config.media_dir);
    let filepath = upload_dir.join(&entry.storage_path);
    if filepath.exists() {
        tokio::fs::remove_file(&filepath).await.map_err(|e| {
            AppError::Internal(format!("Failed to delete file: {e}"))
        })?;
    }

    Ok(Json(serde_json::json!({"deleted": true})))
}

/// GET /api/media/:id — serve a media file (single-user mode, no auth)
pub async fn serve_media(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Response<Body>, AppError> {
    let media = queries::get_media(&state.db, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Media not found".into()))?;

    let upload_dir = std::path::Path::new(&state.config.media_dir);
    let filepath = upload_dir.join(&media.storage_path);
    let data = tokio::fs::read(&filepath).await.map_err(|_| {
        AppError::NotFound("File not found on disk".into())
    })?;

    let mime: mime::Mime = media.mime_type.parse().unwrap_or(mime::APPLICATION_OCTET_STREAM);
    let body = Body::from(data);

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, mime.as_ref())
        .header(header::CONTENT_LENGTH, body.size_hint().exact().unwrap_or(0))
        .body(body)
        .unwrap())
}

fn detect_image_dimensions(data: &[u8]) -> (Option<i32>, Option<i32>) {
    // Simple PNG dimensions check
    if data.len() > 24 && data[..8] == [137, 80, 78, 71, 13, 10, 26, 10] {
        let w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
        return (Some(w as i32), Some(h as i32));
    }
    // JPEG dimensions check
    if data.len() > 4 && data[..2] == [0xFF, 0xD8] {
        let mut pos = 2;
        while pos + 9 < data.len() {
            if data[pos] == 0xFF && data[pos + 1] >= 0xC0 && data[pos + 1] <= 0xCF {
                let h = u16::from_be_bytes([data[pos + 5], data[pos + 6]]);
                let w = u16::from_be_bytes([data[pos + 7], data[pos + 8]]);
                return (Some(w as i32), Some(h as i32));
            }
            pos += 2 + u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
            if pos == 2 { break; } // safety: avoid getting stuck
        }
    }
    (None, None)
}
