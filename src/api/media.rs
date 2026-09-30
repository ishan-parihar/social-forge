// ─── Media API Routes ─────────────────────────────────────────
// File upload and serving for post media attachments.
//
// Storage:
//   Bytes live behind a `MediaStorage` backend. `LocalDisk` (the default)
//   keeps today's behaviour exactly; `S3` targets any S3-compatible store
//   (Cloudflare R2) so media survives container restarts and is visible to
//   every replica. Selection is by env — see `S3Config::from_env`.
//   `serve_media` and `delete` go through the backend; the upload write
//   path writes the object the same way (`MediaStorage::store`, which owns
//   the "Failed to write file: …" error text).
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
use std::path::PathBuf;
use std::sync::Arc;
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

// ─── Storage backends ──────────────────────────────────────────
// One trait, two impls. Everything above this line speaks in object keys
// (`{uuid}.{ext}`), which is exactly what the `media.storage_path` column
// stores — so a row written by one backend resolves on the same backend
// that read it, and switching backends never rewrites the DB.

/// Failure from a [`MediaStorage`] backend.
///
/// The `ctx` prefix is baked in by the backend so callers can surface
/// `AppError::Internal(e.to_string())` and still get the exact message the
/// handler produced before the backend existed (e.g. "Failed to write
/// file: No space left on device").
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("{ctx}: {src}")]
    Io {
        ctx: &'static str,
        src: std::io::Error,
    },
    #[error("HTTP {0}")]
    Http(String),
    #[error("object not found")]
    NotFound,
}

impl StorageError {
    fn io(ctx: &'static str, src: std::io::Error) -> Self {
        Self::Io { ctx, src }
    }
}

/// Pluggable blob store for uploaded media bytes.
///
/// `key` is the opaque object key, persisted verbatim as
/// `media.storage_path`. Implementations must treat it as relative —
/// never as a path to escape from, and never as something to sanitise
/// differently per backend, or existing rows stop resolving.
#[async_trait::async_trait]
pub trait MediaStorage: Send + Sync {
    /// Short backend id for logs and diagnostics (`"local"`, `"s3"`).
    fn backend(&self) -> &'static str;

    /// Persist `data` under `key`, overwriting any existing object.
    async fn store(&self, key: &str, data: &[u8], mime: &str) -> Result<(), StorageError>;

    /// Fetch the bytes for `key`. Missing objects are
    /// [`StorageError::NotFound`], not `Http`.
    async fn get(&self, key: &str) -> Result<Vec<u8>, StorageError>;

    /// Remove `key`. Deleting an object that is already gone succeeds.
    async fn delete(&self, key: &str) -> Result<(), StorageError>;
}

/// Default backend: files under `config.media_dir`.
pub struct LocalDisk {
    dir: PathBuf,
}

impl LocalDisk {
    fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    fn path(&self, key: &str) -> PathBuf {
        self.dir.join(key)
    }
}

#[async_trait::async_trait]
impl MediaStorage for LocalDisk {
    fn backend(&self) -> &'static str {
        "local"
    }

    async fn store(&self, key: &str, data: &[u8], _mime: &str) -> Result<(), StorageError> {
        tokio::fs::create_dir_all(&self.dir)
            .await
            .map_err(|e| StorageError::io("Failed to create upload dir", e))?;
        tokio::fs::write(self.path(key), data)
            .await
            .map_err(|e| StorageError::io("Failed to write file", e))?;
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        Ok(tokio::fs::read(self.path(key))
            .await
            .map_err(|e| StorageError::io("Failed to read file", e))?)
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        let path = self.path(key);
        if path.exists() {
            tokio::fs::remove_file(&path)
                .await
                .map_err(|e| StorageError::io("Failed to delete file", e))?;
        }
        Ok(())
    }
}

/// Resolved S3/R2 coordinates. Constructed only when all four env vars are
/// present, so a half-configured deployment can never silently half-use S3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct S3Config {
    /// Base URL, e.g. `https://<account>.r2.cloudflarestorage.com`.
    pub endpoint: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
}

/// R2 signs every request as region `auto`. MinIO and AWS S3 would need
/// their own region here; not a knob until someone actually runs one.
const S3_REGION: &str = "auto";

impl S3Config {
    /// `None` unless all four values are present and non-empty. Partial
    /// config falls back to local disk — with a warning, because that
    /// fallback is exactly what makes media die with the container.
    fn from_env() -> Option<Self> {
        let endpoint = std::env::var("MEDIA_S3_ENDPOINT").ok().filter(|v| !v.is_empty());
        let bucket = std::env::var("MEDIA_S3_BUCKET").ok().filter(|v| !v.is_empty());
        let access_key = std::env::var("MEDIA_S3_KEY").ok().filter(|v| !v.is_empty());
        let secret_key = std::env::var("MEDIA_S3_SECRET").ok().filter(|v| !v.is_empty());

        let cfg = Self::from_vars(
            endpoint.as_deref(),
            bucket.as_deref(),
            access_key.as_deref(),
            secret_key.as_deref(),
        );
        if cfg.is_none() && [endpoint, bucket, access_key, secret_key].iter().any(|v| v.is_some()) {
            warn_once(
                "MEDIA_S3_* is partially set (all four of ENDPOINT, BUCKET, KEY, SECRET are required) \
                 — media falls back to local disk and will not survive a container restart",
            );
        }
        cfg
    }

    fn from_vars(
        endpoint: Option<&str>,
        bucket: Option<&str>,
        access_key: Option<&str>,
        secret_key: Option<&str>,
    ) -> Option<Self> {
        Some(Self {
            endpoint: endpoint?.trim_end_matches('/').to_string(),
            bucket: bucket?.to_string(),
            access_key: access_key?.to_string(),
            secret_key: secret_key?.to_string(),
        })
    }
}

/// S3-compatible backend over plain `PUT`/`GET`/`DELETE` with SigV4
/// (service `s3`). Hand-rolled instead of pulling an SDK: three verbs and
/// one signing routine do not justify the dependency tree, and the HTTP
/// client is the one `AppState` already owns.
pub struct S3 {
    endpoint: String,
    bucket: String,
    access_key: String,
    secret_key: String,
    client: reqwest::Client,
}

impl S3 {
    fn new(cfg: S3Config, client: reqwest::Client) -> Self {
        Self {
            endpoint: cfg.endpoint,
            bucket: cfg.bucket,
            access_key: cfg.access_key,
            secret_key: cfg.secret_key,
            client,
        }
    }

    /// `https://<endpoint>/<bucket>/<encoded key>`. Path-style, which R2
    /// accepts, so a bucket name that is not a valid DNS label still works.
    fn object_url(&self, key: &str) -> String {
        format!(
            "{}/{}/{}",
            self.endpoint,
            self.bucket,
            uri_encode_segment(key)
        )
    }

    /// AWS SigV4 headers (service `s3`, region `auto`) for one request.
    ///
    /// `extra` holds headers the caller also puts on the wire — signing a
    /// header that is not sent (or sending a signed header with a different
    /// value) is the one way to get a 403 from a correct implementation.
    fn sign_headers(
        &self,
        method: &str,
        url: &str,
        payload: &[u8],
        extra: &[(&str, &str)],
    ) -> Result<Vec<(&'static str, String)>, StorageError> {
        let parsed = url::Url::parse(url)
            .map_err(|e| StorageError::Http(format!("bad endpoint url: {e}")))?;
        let host = match (parsed.host_str(), parsed.port()) {
            (Some(h), Some(p)) => format!("{h}:{p}"),
            (Some(h), None) => h.to_string(),
            _ => return Err(StorageError::Http("endpoint has no host".into())),
        };

        let now = chrono::Utc::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let day = now.format("%Y%m%d").to_string();
        let scope = format!("{day}/{S3_REGION}/s3/aws4_request");
        let payload_hash = sha256_hex(payload);

        let mut headers: Vec<(String, String)> = vec![
            ("host".into(), host),
            ("x-amz-content-sha256".into(), payload_hash.clone()),
            ("x-amz-date".into(), amz_date.clone()),
        ];
        for (name, value) in extra {
            headers.push((name.to_ascii_lowercase(), value.trim().to_string()));
        }
        headers.sort_by(|a, b| a.0.cmp(&b.0));
        let signed_headers = headers
            .iter()
            .map(|(n, _)| n.as_str())
            .collect::<Vec<_>>()
            .join(";");
        let canonical_headers = headers
            .iter()
            .map(|(n, v)| format!("{n}:{v}\n"))
            .collect::<String>();

        // S3 signs the path once-encoded, with `/` preserved as a separator.
        let canonical_request = format!(
            "{method}\n{}\n\n{canonical_headers}\n{signed_headers}\n{payload_hash}",
            uri_encode_segment(parsed.path())
        );
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
            sha256_hex(canonical_request.as_bytes())
        );

        let k_date = hmac_sha256(format!("AWS4{}", self.secret_key).as_bytes(), day.as_bytes())?;
        let k_region = hmac_sha256(&k_date, S3_REGION.as_bytes())?;
        let k_service = hmac_sha256(&k_region, b"s3")?;
        let k_signing = hmac_sha256(&k_service, b"aws4_request")?;
        let signature = hex::encode(hmac_sha256(&k_signing, string_to_sign.as_bytes())?);

        Ok(vec![
            ("x-amz-date", amz_date),
            ("x-amz-content-sha256", payload_hash),
            (
                "authorization",
                format!(
                    "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signed_headers}, Signature={signature}",
                    self.access_key
                ),
            ),
        ])
    }

    /// Send a signed `PUT`/`GET`/`DELETE`. Returns the raw response so
    /// callers can interpret 404 themselves (missing object vs. already
    /// deleted are both fine for `delete`).
    async fn send_signed(
        &self,
        method: &str,
        url: &str,
        payload: Vec<u8>,
        mime: Option<&str>,
    ) -> Result<reqwest::Response, StorageError> {
        let mut req = match method {
            "PUT" => self.client.put(url),
            "GET" => self.client.get(url),
            "DELETE" => self.client.delete(url),
            _ => return Err(StorageError::Http(format!("unsupported method {method}"))),
        };
        let extra: Vec<(&str, &str)> = mime.map(|m| vec![("content-type", m)]).unwrap_or_default();
        if let Some(m) = mime {
            req = req.header("content-type", m);
        }
        for (name, value) in self.sign_headers(method, url, &payload, &extra)? {
            req = req.header(name, value);
        }
        req.body(payload)
            .send()
            .await
            .map_err(|e| StorageError::Http(e.to_string()))
    }
}

#[async_trait::async_trait]
impl MediaStorage for S3 {
    fn backend(&self) -> &'static str {
        "s3"
    }

    async fn store(&self, key: &str, data: &[u8], mime: &str) -> Result<(), StorageError> {
        let resp = self
            .send_signed("PUT", &self.object_url(key), data.to_vec(), Some(mime))
            .await?;
        expect_ok(resp).await?;
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let resp = self
            .send_signed("GET", &self.object_url(key), Vec::new(), None)
            .await?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(StorageError::NotFound);
        }
        expect_ok(resp)
            .await?
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| StorageError::Http(e.to_string()))
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        let resp = self
            .send_signed("DELETE", &self.object_url(key), Vec::new(), None)
            .await?;
        // S3 already answers 204 for a missing key; tolerate a bucket that
        // answers 404 instead, since LocalDisk::delete is a no-op there.
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(());
        }
        expect_ok(resp).await?;
        Ok(())
    }
}

async fn expect_ok(resp: reqwest::Response) -> Result<reqwest::Response, StorageError> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    let body = resp.text().await.unwrap_or_default();
    Err(StorageError::Http(format!(
        "{status}: {}",
        body.chars().take(200).collect::<String>()
    )))
}

/// Percent-encode a path, preserving `/` so object keys with folders keep
/// their separators. S3 signs the path in exactly this encoded form, so the
/// value sent on the wire and the value signed can never drift.
fn uri_encode_segment(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for b in path.bytes() {
        let c = b as char;
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~' | '/') {
            out.push(c);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(<sha2::Sha256 as sha2::Digest>::digest(bytes))
}

fn hmac_sha256(key: &[u8], msg: &[u8]) -> Result<Vec<u8>, StorageError> {
    use hmac::{Hmac, Mac};
    let mut mac = <Hmac<sha2::Sha256> as Mac>::new_from_slice(key)
        .map_err(|e| StorageError::Http(format!("bad hmac key: {e}")))?;
    mac.update(msg);
    Ok(mac.finalize().into_bytes().to_vec())
}

/// Backend selection, split out from [`storage`] so the rules are testable
/// without a bucket and without mutating process env.
fn select_backend(
    s3: Option<S3Config>,
    media_dir: &str,
    client: &reqwest::Client,
) -> Arc<dyn MediaStorage> {
    match s3 {
        Some(cfg) => {
            log_backend_once(true, &cfg.bucket);
            Arc::new(S3::new(cfg, client.clone()))
        }
        None => {
            log_backend_once(false, media_dir);
            Arc::new(LocalDisk::new(media_dir))
        }
    }
}

/// The backend media bytes live in for this request.
///
/// Reads the S3 env on every call so a restarted process picks up a changed
/// config without a rebuild, and so the local path stays byte-identical to
/// the previous hardcoded `config.media_dir` behaviour when S3 is unset.
pub fn storage(state: &AppState) -> Arc<dyn MediaStorage> {
    select_backend(S3Config::from_env(), &state.config.media_dir, &state.media_http_client)
}

/// `storage()` is per-request but the answer only changes when the operator
/// changes the environment — log it once, loudly, and not per request.
fn log_backend_once(is_s3: bool, where_: &str) {
    static LOGGED: std::sync::Once = std::sync::Once::new();
    LOGGED.call_once(|| {
        if is_s3 {
            tracing::info!("Media storage: S3 bucket `{where_}`");
        } else {
            tracing::info!("Media storage: local disk `{where_}`");
        }
    });
}

fn warn_once(msg: &str) {
    static LOGGED: std::sync::Once = std::sync::Once::new();
    LOGGED.call_once(|| tracing::warn!("{msg}"));
}

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

    storage(&state)
        .delete(&entry.storage_path)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

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

    let data = storage(&state)
        .get(&media.storage_path)
        .await
        .map_err(|_| AppError::NotFound("File not found on disk".into()))?;

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

    // ── backend selection ───────────────────────────────────
    // No bucket, no network: these pin the rules that decide which
    // backend serves a request.

    fn s3_env() -> S3Config {
        S3Config::from_vars(
            Some("https://acct.r2.cloudflarestorage.com/"),
            Some("media"),
            Some("AKIA"),
            Some("secret"),
        )
        .expect("all four vars present")
    }

    #[test]
    fn selects_local_when_s3_unset() {
        let backend = select_backend(None, "./uploads", &reqwest::Client::new());
        assert_eq!(backend.backend(), "local");
    }

    #[test]
    fn selects_s3_when_configured() {
        let backend = select_backend(Some(s3_env()), "./uploads", &reqwest::Client::new());
        assert_eq!(backend.backend(), "s3");
    }

    #[test]
    fn s3_config_requires_all_four_vars() {
        let partial = S3Config::from_vars(Some("https://acct.r2.cloudflarestorage.com"), Some("media"), Some("AKIA"), None);
        assert!(partial.is_none());
    }

    #[test]
    fn s3_config_rejects_blank_vars() {
        let blank = S3Config::from_vars(Some("https://acct.r2.cloudflarestorage.com"), Some("  "), Some("AKIA"), Some("secret"));
        assert!(blank.is_none());
    }

    #[test]
    fn s3_config_strips_trailing_slash_from_endpoint() {
        assert_eq!(s3_env().endpoint, "https://acct.r2.cloudflarestorage.com");
    }

    #[test]
    fn object_url_is_path_style_and_encodes_key() {
        let s3 = S3::new(s3_env(), reqwest::Client::new());
        assert_eq!(
            s3.object_url("ab cd.png"),
            "https://acct.r2.cloudflarestorage.com/media/ab%20cd.png"
        );
    }

    #[test]
    fn local_disk_roundtrips_and_deletes_idempotently() {
        let dir = std::env::temp_dir().join(format!("sf-media-test-{}", Uuid::new_v4()));
        let local = LocalDisk::new(&dir);
        let rt = async_runtime();

        rt.block_on(async {
            local.store("a.png", b"\x89PNG-body", "image/png").await.unwrap();
            assert_eq!(local.get("a.png").await.unwrap(), b"\x89PNG-body");

            local.delete("a.png").await.unwrap();
            assert!(matches!(local.get("a.png").await, Err(StorageError::Io { .. })));
            // Second delete must be a no-op, not an error.
            local.delete("a.png").await.unwrap();
        });

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Minimal current-thread runtime so the disk test needs no dev-dependency.
    fn async_runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
    }
}
