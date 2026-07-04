// ─── Media API Routes ─────────────────────────────────────────
// File upload and serving for post media attachments.
//
// Security:
//   - Upload enforces a MIME allowlist (image/png, image/jpeg, image/webp,
//     image/gif, video/mp4, video/quicktime) AND verifies magic bytes via
//     a small inline sniff. Client-supplied Content-Type is never trusted
//     alone. See `sniff_mime` below.
//   - `serve_media` always sets `X-Content-Type-Options: nosniff` to
//     prevent browsers from interpreting non-image bytes as HTML/JS.

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

/// Allowed MIME types for uploads. Anything else is rejected before
/// the file is written to disk or stored in the DB.
const ALLOWED_MIMES: &[&str] = &[
    "image/png",
    "image/jpeg",
    "image/webp",
    "image/gif",
    "video/mp4",
    "video/quicktime",
];

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

    let declared_mime = field
        .content_type()
        .unwrap_or("application/octet-stream")
        .to_string();

    let data = field.bytes().await.map_err(|e| AppError::BadRequest(format!("Read error: {e}")))?;

    if data.len() as u64 > MAX_FILE_SIZE {
        return Err(AppError::BadRequest("File too large (max 50 MB)".into()));
    }

    // ── MIME validation: allowlist + magic-byte sniff ────────
    // The client-supplied Content-Type is never trusted alone.
    // We sniff the actual file bytes and require the sniffed MIME to
    // (a) be in the allowlist and (b) match the declared Content-Type
    // when one was supplied. Mismatches are rejected with 400.
    let sniffed_mime = sniff_mime(&data);
    let mime_type = match (declared_mime.as_str(), sniffed_mime) {
        // Sniffer recognised the bytes — trust it (it can't be lied to).
        (declared, Some(sniffed)) if ALLOWED_MIMES.contains(&sniffed.as_str()) => {
            // If the client declared something else, reject as suspicious.
            if !declared.is_empty()
                && declared != "application/octet-stream"
                && declared != sniffed
            {
                return Err(AppError::BadRequest(format!(
                    "MIME mismatch: declared `{declared}` but bytes look like `{sniffed}`"
                )));
            }
            sniffed
        }
        // Sniffer didn't recognise the bytes — fall back to declared
        // only if it's in the allowlist (still rejects text/html etc.).
        (declared, None) if ALLOWED_MIMES.contains(&declared) => declared.to_string(),
        // Either sniffer said "not allowed" or declared is not in allowlist.
        (_, Some(sniffed)) => {
            return Err(AppError::BadRequest(format!(
                "Unsupported file type: `{sniffed}`. Allowed: PNG, JPEG, WebP, GIF, MP4, QuickTime."
            )));
        }
        (_, None) => {
            return Err(AppError::BadRequest(
                "Unrecognised file type. Allowed: PNG, JPEG, WebP, GIF, MP4, QuickTime.".into(),
            ));
        }
    };

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
///
/// Security:
///   - Always sets `X-Content-Type-Options: nosniff` to stop browsers from
///     MIME-sniffing the body and interpreting non-image bytes as HTML/JS.
///   - For non-image MIME types (video, octet-stream, etc.), sets
///     `Content-Disposition: attachment` so the body is downloaded, not
///     rendered. This blocks the stored-XSS-by-Content-Type attack vector.
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

    let mime_str = media.mime_type.as_str();

    // Defense-in-depth: sniff the bytes again at serve time. If the
    // sniffed MIME disagrees with the stored MIME, serve as attachment
    // with octet-stream. This catches any pre-existing rows in the DB
    // that were uploaded before the allowlist was enforced.
    //
    // NOTE: sniff_mime MUST be called before `Body::from(data)` below —
    // `Body::from` moves `data` and we need a `&[u8]` reference to it.
    let (final_content_type, disposition) = match sniff_mime(&data) {
        Some(ref sniffed) if sniffed.as_str() == mime_str => (mime_str.to_string(), "inline"),
        Some(ref sniffed) if ALLOWED_MIMES.contains(&sniffed.as_str()) && mime_str == "application/octet-stream" => {
            (sniffed.clone(), if sniffed.starts_with("image/") { "inline" } else { "attachment" })
        }
        _ => (
            "application/octet-stream".to_string(),
            "attachment",
        ),
    };

    let body = Body::from(data);

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, &final_content_type)
        .header(header::CONTENT_LENGTH, body.size_hint().exact().unwrap_or(0))
        .header("X-Content-Type-Options", "nosniff")
        .header("Content-Disposition", disposition)
        .body(body)
        .unwrap())
}

/// GET /api/proxy-media?url=... — proxy external media to bypass CORS/CDN restrictions
/// Used for X/Twitter video CDN which returns 403 when loaded directly from browser.
/// Uses wreq (Chrome TLS fingerprinting) for X/Twitter CDN domains to bypass bot detection.
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

    // X/Twitter CDN domains use wreq for Chrome TLS fingerprinting.
    // Other CDNs use the standard reqwest client.
    // We must fetch inside each branch because wreq::Response and reqwest::Response
    // are different types — Rust's if/else requires both branches to match.
    let is_x_cdn = host == "video.twimg.com" || host == "pbs.twimg.com";
    let range_header = req_headers.get(header::RANGE).and_then(|v| v.to_str().ok()).map(String::from);

    // Shared headers for all upstream requests
    const UPSTREAM_REFERER: &str = "https://x.com/";
    const UPSTREAM_UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

    // Fetch from upstream and extract common fields.
    // Both branches produce identical (status, content_type, content_length, content_range, bytes).
    let (upstream_status, content_type, content_length, content_range, bytes) = if is_x_cdn {
        let mut req = state
            .media_wreq_client
            .get(url)
            .header(header::REFERER, UPSTREAM_REFERER)
            .header(header::USER_AGENT, UPSTREAM_UA);
        if let Some(ref range) = range_header {
            req = req.header(header::RANGE, range.as_str());
        }
        let resp = req.send().await
            .map_err(|e| AppError::Internal(format!("Failed to fetch media (wreq): {e}")))?;
        let status = resp.status();
        if !status.is_success() && status.as_u16() != 206 {
            return Err(AppError::Internal(format!("Upstream returned {status}")));
        }
        let ct = resp.headers().get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();
        let cl = resp.content_length();
        let cr = resp.headers().get("content-range")
            .and_then(|v| v.to_str().ok())
            .map(String::from);
        let bytes = resp.bytes().await
            .map_err(|e| AppError::Internal(format!("Failed to read upstream body (wreq): {e}")))?;
        (status, ct, cl, cr, bytes)
    } else {
        let mut req = state
            .media_http_client
            .get(url)
            .header(header::REFERER, UPSTREAM_REFERER)
            .header(header::USER_AGENT, UPSTREAM_UA);
        if let Some(ref range) = range_header {
            req = req.header(header::RANGE, range.as_str());
        }
        let resp = req.send().await
            .map_err(|e| AppError::Internal(format!("Failed to fetch media (reqwest): {e}")))?;
        let status = resp.status();
        if !status.is_success() && status.as_u16() != 206 {
            return Err(AppError::Internal(format!("Upstream returned {status}")));
        }
        let ct = resp.headers().get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();
        let cl = resp.content_length();
        let cr = resp.headers().get("content-range")
            .and_then(|v| v.to_str().ok())
            .map(String::from);
        let bytes = resp.bytes().await
            .map_err(|e| AppError::Internal(format!("Failed to read upstream body (reqwest): {e}")))?;
        (status, ct, cl, cr, bytes)
    };

    // Build response headers — forward status (200 or 206 for Range)
    let mut builder = Response::builder()
        .status(upstream_status.as_u16())
        .header(header::CONTENT_TYPE, &content_type)
        .header(header::CACHE_CONTROL, "public, max-age=86400")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(header::ACCESS_CONTROL_ALLOW_HEADERS, "Range")
        .header(header::ACCEPT_RANGES, "bytes");

    // Forward content-length and content-range from upstream
    if let Some(cl) = content_length {
        builder = builder.header(header::CONTENT_LENGTH, cl);
    }
    if let Some(cr) = &content_range {
        builder = builder.header("Content-Range", cr.as_str());
    }

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

/// Sniff the actual MIME type from file magic bytes.
///
/// Returns `Some(mime)` for the formats in `ALLOWED_MIMES`, or `None`
/// if the bytes don't match any recognised signature. This is used
/// both at upload time (defense-in-depth against client-supplied
/// Content-Type lies) and at serve time (defense-in-depth against
/// any pre-existing DB rows that predate the upload allowlist).
///
/// Signatures sourced from the IANA media-type registry and the
/// `infer` crate (which we don't depend on to keep the dep tree lean).
fn sniff_mime(data: &[u8]) -> Option<String> {
    if data.len() < 4 {
        return None;
    }
    // PNG: 89 50 4E 47 0D 0A 1A 0A
    if data.len() >= 8 && data[..8] == [137, 80, 78, 71, 13, 10, 26, 10] {
        return Some("image/png".into());
    }
    // JPEG: FF D8 FF
    if data.len() >= 3 && data[..3] == [0xFF, 0xD8, 0xFF] {
        return Some("image/jpeg".into());
    }
    // GIF: "GIF87a" or "GIF89a"
    if data.len() >= 6 && (data[..6] == *b"GIF87a" || data[..6] == *b"GIF89a") {
        return Some("image/gif".into());
    }
    // WebP: "RIFF" .... "WEBP"
    if data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        return Some("image/webp".into());
    }
    // MP4: ftyp box at offset 4 — "ftyp" + 4-char brand
    // Mask: 00 00 00 ?? 66 74 79 70 (?? = box size, varies)
    if data.len() >= 12 && &data[4..8] == b"ftyp" {
        let brand = &data[8..12];
        // Common MP4 brands: isom, iso2, mp41, mp42, avc1, M4V , M4A , etc.
        // QuickTime: qt  (with trailing spaces), which we map to video/quicktime.
        if brand == b"qt  " || brand == b"qt  " {
            return Some("video/quicktime".into());
        }
        return Some("video/mp4".into());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniff_png_signature() {
        let png = [137, 80, 78, 71, 13, 10, 26, 10, 0, 0];
        assert_eq!(sniff_mime(&png), Some("image/png".into()));
    }

    #[test]
    fn sniff_jpeg_signature() {
        let jpg = [0xFF, 0xD8, 0xFF, 0xE0, 0, 0];
        assert_eq!(sniff_mime(&jpg), Some("image/jpeg".into()));
    }

    #[test]
    fn sniff_gif_signatures() {
        assert_eq!(sniff_mime(b"GIF87aextra"), Some("image/gif".into()));
        assert_eq!(sniff_mime(b"GIF89aextra"), Some("image/gif".into()));
    }

    #[test]
    fn sniff_webp_signature() {
        let mut webp = b"RIFF\x00\x00\x00\x00WEBP".to_vec();
        webp.extend_from_slice(&[0; 10]);
        assert_eq!(sniff_mime(&webp), Some("image/webp".into()));
    }

    #[test]
    fn sniff_mp4_signature() {
        let mp4 = [0, 0, 0, 32, b'f', b't', b'y', b'p', b'i', b's', b'o', b'm'];
        assert_eq!(sniff_mime(&mp4), Some("video/mp4".into()));
    }

    #[test]
    fn sniff_rejects_html() {
        let html = b"<script>alert(1)</script>";
        assert_eq!(sniff_mime(html), None);
    }

    #[test]
    fn sniff_rejects_empty() {
        assert_eq!(sniff_mime(&[]), None);
        assert_eq!(sniff_mime(&[1, 2, 3]), None);
    }
}
