// ─── Media API Routes ─────────────────────────────────────────
// File upload and serving for post media attachments.

use axum::{
    body::{Body, HttpBody},
    extract::{Path, State},
    http::{header, Response},
    Json,
};
use axum_extra::extract::Multipart;
use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::db::models::MediaPublic;
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

/// GET /api/proxy-media?url=... — proxy external media to bypass CORS/CDN restrictions
/// Used for X/Twitter video CDN which returns 403 when loaded directly from browser.
/// Supports Range requests for video playback (seeking, adaptive streaming).
pub async fn proxy_media(
    State(state): State<AppState>,
    req_headers: axum::http::header::HeaderMap,
    axum::extract::Query(params): axum::extract::Query<ProxyMediaQuery>,
) -> Result<Response<Body>, AppError> {
    use axum::http::header;

    // Validate URL to prevent SSRF — only allow known CDN domains
    let url = &params.url;
    let parsed = url::Url::parse(url).map_err(|_| AppError::BadRequest("Invalid URL".into()))?;
    let host = parsed.host_str().unwrap_or("");
    let allowed = (parsed.scheme() == "https") && (
        host == "video.twimg.com"
        || host == "pbs.twimg.com"
        || host == "media.tenor.com"
        || host == "www.instagram.com"
        || host == "i.ytimg.com"
        || host == "files.catbox.moe"
        || host == "i.imgur.com"
        || (host.starts_with("scontent-") && host.contains(".fbcdn."))
    );

    if !allowed {
        return Err(AppError::BadRequest("Domain not allowed for proxying".into()));
    }

    // Forward Range header from the browser to the upstream CDN
    let mut upstream_req = state
        .media_http_client
        .get(url)
        .header("Referer", "https://x.com/")
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36");

    if let Some(range) = req_headers.get(header::RANGE) {
        if let Ok(range_str) = range.to_str() {
            upstream_req = upstream_req.header(header::RANGE, range_str);
        }
    }

    let resp = upstream_req
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch media: {e}")))?;

    let upstream_status = resp.status();
    if !upstream_status.is_success() && upstream_status.as_u16() != 206 {
        return Err(AppError::Internal(format!("Upstream returned {upstream_status}")));
    }

    let content_type = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    // Build response headers — forward status (200 or 206 for Range)
    let mut builder = Response::builder()
        .status(upstream_status.as_u16())
        .header(header::CONTENT_TYPE, &content_type)
        .header(header::CACHE_CONTROL, "public, max-age=86400")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(header::ACCESS_CONTROL_ALLOW_HEADERS, "Range")
        .header(header::ACCEPT_RANGES, "bytes");

    // Forward content-length and content-range from upstream
    if let Some(cl) = resp.content_length() {
        builder = builder.header(header::CONTENT_LENGTH, cl);
    }
    if let Some(cr) = resp.headers().get("content-range") {
        if let Ok(cr_str) = cr.to_str() {
            builder = builder.header("Content-Range", cr_str);
        }
    }

    // Read the upstream response body
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to read upstream body: {e}")))?;

    let body = Body::from(bytes);

    builder.body(body).map_err(|e| AppError::Internal(e.to_string()))
}

#[derive(Debug, serde::Deserialize)]
pub struct ProxyMediaQuery {
    pub url: String,
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
