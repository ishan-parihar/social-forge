# Social-Forge Media Pipeline Audit & Local-File Upload Upgrade Plan

## Executive Summary

**Core Problem:** Social-forge's MCP tools currently require media to be specified as URLs (`/api/media/{id}` or external URLs). An AI agent generating content locally (e.g., carousel-mcp exporting PNGs to disk) **cannot** pass those local files to the posting pipeline without first manually uploading each file via base64 encoding — an unreliable, roundabout process that defeats the purpose of automation.

**Root Cause:** The `upload_from_path` function exists in `tools_media.rs` but is **only exposed via CLI** (`social-forge media upload <path>`), not as an MCP tool. The MCP `posts_media_upload` tool requires base64-encoded data, which is impractical for large images/videos. Additionally, the entire media pipeline assumes URLs are pre-resolved — providers like Instagram's Graph API require publicly accessible URLs, not local file paths.

**Solution Architecture:** Inspired by postiz-app's `readOrFetch` + `UploadFactory` pattern, we implement a three-layer approach:
1. **MCP `media_upload_from_path` tool** — accepts local file paths directly
2. **Auto-URL-resolution in the publish pipeline** — providers can resolve local paths to their own hosted URLs transparently
3. **Batch upload tool** — upload multiple files in a single MCP call for carousel workflows

---

## Part 1: Current State Audit

### 1.1 Media Upload Flow (Current)

```
CLI Path:
  social-forge media upload /path/to/image.png
    → tools_media::upload_from_path()
    → copies file to uploads/ dir, registers in DB
    → returns { id, url: "/api/media/{uuid}" }

MCP Path:
  posts_media_upload({ filename, mime_type, data: "<base64>" })
    → tools_media::upload_media()
    → decodes base64, copies to uploads/ dir, registers in DB
    → returns { id, url: "/api/media/{uuid}" }

HTTP API Path:
  POST /api/media (multipart/form-data)
    → api::media::upload()
    → saves file, registers in DB
    → returns { id, url: "/api/media/{uuid}" }
```

### 1.2 Post Creation Flow (Current)

```
MCP: posts_stage / posts_create
  → media field: Option<serde_json::Value> — expects [{"url": "/api/media/..."}]
  → StagingRequest.media: serde_json::Value
  → Stored in posts table as JSON

Publish: posts_publish
  → PostService::publish()
  → Deserializes media into Vec<MediaAttachment>
  → MediaAttachment.url is passed directly to provider.publish()
  → Provider fetches the URL content internally (e.g., X's fetch_media_bytes)
```

### 1.3 Per-Provider Media Handling

| Provider | How Media is Handled | Supports Local Paths? |
|----------|---------------------|----------------------|
| **X/Twitter** | `fetch_media_bytes()` downloads from URL OR reads local file via `tokio::fs::read()` | ✅ YES (already works) |
| **Instagram** | Passes `media_url` to Graph API — must be a publicly accessible URL | ❌ NO (requires public URL) |
| **Facebook** | Similar to Instagram — uses Graph API with public URLs | ❌ NO |
| **LinkedIn** | Uploads via LinkedIn Media Upload API with binary data | ❌ NO (needs re-upload) |
| **Bluesky** | `upload_and_embed()` downloads from URL, uploads to Bluesky blob store | ❌ NO (needs re-upload) |
| **Mastodon** | Downloads from URL, uploads via multipart to Mastodon instance | ❌ NO (needs re-upload) |
| **TikTok** | Downloads video from URL, re-uploads to TikTok | ❌ NO |
| **Pinterest** | Uses URL-based pin creation | ❌ NO |
| **Threads** | Uses Graph API (similar to Instagram) | ❌ NO |

### 1.4 Critical Gap: The URL Resolution Problem

When an AI agent (e.g., carousel-mcp) exports files to local disk:
```
/home/user/.carousel-mcp/output/slide_1.png
/home/user/.carousel-mcp/output/slide_2.png
...
```

There is **no way** to get these files into a social-forge post via MCP because:
1. `posts_media_upload` requires base64 (impractical for 10+ images)
2. `upload_from_path` exists but is CLI-only, not exposed as MCP tool
3. Even after upload, the returned URL (`/api/media/{id}`) is a **local server URL** — not publicly accessible by Instagram/Facebook APIs
4. Providers that download from URLs (X, Bluesky, Mastodon) can work with local URLs IF the social-forge server is running, but platform APIs that need public URLs will fail

### 1.5 What postiz-app Does Differently

Postiz solves this with a layered architecture:

```
1. Storage Abstraction (IUploadProvider interface):
   - local-storage.service.ts  →  saves to local filesystem
   - s3-storage.service.ts     →  uploads to Cloudflare R2 / S3
   - UploadFactory.createStorage()  →  picks backend by STORAGE_PROVIDER env var

2. URL Resolution (readOrFetch utility):
   - If input is a URL → fetches the bytes
   - If input is a local path → reads the file directly
   - Returns unified { buffer, mimeType, originalName }

3. Upload-From-URL Tool (AI Agent specific):
   - ssrfSafeDispatcher() → safe URL fetching
   - file-type detection from buffer
   - Uploads via UploadFactory → returns { id, path }
   - Registers in MediaService DB

4. Provider Media Processing (updateMedia in orchestrator):
   - Before calling provider.post(), media is processed:
     - Fetched/resolved to bytes
     - Format-converted if needed (e.g., JPEG conversion)
     - Re-uploaded to platform-specific storage if required
```

**Key Insight:** Postiz decouples "media storage" from "media publishing." Media is stored once (in R2/local), then each provider re-uploads from that storage to the platform's own media endpoint. This is the pattern social-forge needs.

---

## Part 2: Proposed Architecture

### 2.1 Core Principle: Storage vs. Publishing Separation

```
┌─────────────────────────────────────────────────────────────┐
│                    MEDIA STORAGE LAYER                       │
│                                                              │
│  Local files ──→ uploads/ (local) ──→ DB record ──→ URL     │
│  External URLs ──→ downloads ──→ uploads/ ──→ DB record     │
│  Base64 data ──→ decodes ──→ uploads/ ──→ DB record         │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                  MEDIA RESOLUTION LAYER                      │
│                                                              │
│  For each provider at publish time:                          │
│  - If MediaAttachment.url is local path → serve via API     │
│  - If MediaAttachment.url is external URL → proxy or pass   │
│  - Provider fetches bytes as needed (already works for X)   │
│  - For providers needing public URLs → use proxy endpoint   │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                 PROVIDER PUBLISH LAYER                       │
│                                                              │
│  Each provider's publish() receives Vec<MediaAttachment>     │
│  and handles media according to platform requirements:      │
│  - X: fetch_media_bytes() → upload to Twitter media API     │
│  - IG: needs public URL → use proxy or self-hosted URL      │
│  - Bluesky: download → upload to blob store                  │
│  - LinkedIn: download → upload via LinkedIn Media API        │
│  - Mastodon: download → upload via multipart                 │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 New MCP Tools

#### Tool 1: `media_upload_from_path` (NEW)
```rust
pub struct MediaUploadFromPathInput {
    pub path: String,           // Local file path
    pub alt: Option<String>,    // Alt text
}

// Accepts a local file path, copies to uploads/, registers in DB
// Reuses existing upload_from_path() logic
// Returns { id, url, mime_type, filename, size }
```

#### Tool 2: `media_upload_batch` (NEW)
```rust
pub struct MediaUploadBatchInput {
    pub paths: Vec<String>,     // Multiple local file paths
    pub alt: Option<String>,    // Shared alt text (optional)
}

// Uploads multiple files in one call
// Returns { media: Vec<{ id, url, mime_type, filename, size }> }
// Critical for carousel workflows (5-10 images)
```

#### Tool 3: `media_upload_from_url` (NEW)
```rust
pub struct MediaUploadFromUrlInput {
    pub url: String,            // External URL to download
    pub filename: Option<String>, // Override filename
    pub alt: Option<String>,
}

// Downloads from URL, saves to uploads/, registers in DB
// Useful when agent has image URLs but wants them hosted locally
// Returns { id, url, mime_type, filename, size }
```

### 2.3 CLI Enhancement: `media upload-batch`

```bash
social-forge media upload-batch /path/to/slide_1.png /path/to/slide_2.png ...
# or
social-forge media upload-batch /path/to/carousel-*.png
# Returns JSON array of { id, url } objects
```

### 2.4 Provider-Level Media Resolution

For providers that require publicly accessible URLs (Instagram, Facebook, Threads), add automatic URL resolution:

```rust
// In SocialProvider trait, add optional method:
async fn resolve_media_url(&self, attachment: &MediaAttachment, config: &Config) -> Result<String, ProviderError> {
    // Default: return attachment.url as-is
    // Override for providers that need local→public URL resolution
    Ok(attachment.url.clone())
}

// Instagram/Facebook override:
// If URL starts with "/api/media/" or is localhost → use config.app_url to build public URL
// e.g., "/api/media/abc" → "https://your-server.com/api/media/abc"
```

This works because social-forge already has a `serve_media` endpoint at `GET /api/media/:id` that serves the file with correct MIME type.

---

## Part 3: Implementation Plan

### Phase 1: Expose Local File Upload via MCP (1-2 days)

**Goal:** AI agents can upload local files to social-forge via MCP.

#### Step 1.1: Add `media_upload_from_path` MCP Tool
- **File:** `src/mcp/tools_media.rs`
- Add `MediaUploadFromPathInput` and `MediaUploadFromPathOutput` structs
- Add `pub async fn upload_from_path_mcp()` that wraps existing `upload_from_path()`
- **File:** `src/mcp/mod.rs`
- Register the new tool in `SocialForgeMcpServer` with `#[tool]` attribute
- Description: "Upload media from a local file path (more efficient than base64 for large files)"

#### Step 1.2: Add `media_upload_batch` MCP Tool
- **File:** `src/mcp/tools_media.rs`
- Add `MediaUploadBatchInput` / `MediaUploadBatchOutput` structs
- Implement batch upload: iterate paths, call `upload_from_path()` for each, collect results
- Add parallel upload with `tokio::join!` or `futures::future::join_all` for performance

#### Step 1.3: Add `media_upload_from_url` MCP Tool
- **File:** `src/mcp/tools_media.rs`
- Add `MediaUploadFromUrlInput` / `MediaUploadFromUrlOutput` structs
- Download file from URL using `reqwest` or `wreq`
- Validate MIME type (image/*, video/*)
- Save to uploads/, register in DB

#### Step 1.4: Add `media upload-batch` CLI Command
- **File:** `src/cli/mod.rs`
- Extend `MediaAction` enum with `UploadBatch { paths: Vec<String> }`
- **File:** `src/cli/run.rs`
- Handle the new command, call batch upload function

#### Step 1.5: Bridge for MCP CLI
- **File:** `src/cli/mcp_bridge.rs`
- Register `posts_media_upload_from_path` and `posts_media_upload_batch` in the bridge

---

### Phase 2: Provider Media Resolution (2-3 days)

**Goal:** Providers that need public URLs can automatically resolve local media URLs.

#### Step 2.1: Add Media URL Resolution to SocialProvider Trait
- **File:** `src/social/mod.rs`
- Add `resolve_media_url()` default method to `SocialProvider` trait
- Default implementation returns the URL unchanged

#### Step 2.2: Implement Resolution for URL-Dependent Providers
- **File:** `src/social/instagram.rs`
- Override `resolve_media_url()`:
  - If URL is relative (`/api/media/...`), prepend `config.app_url`
  - If URL is localhost, replace with `config.app_url`
  - If URL is already public, return as-is
- **File:** `src/social/facebook.rs` — same pattern
- **File:** `src/social/threads.rs` — same pattern
- **File:** `src/social/instagram_standalone.rs` — same pattern

#### Step 2.3: Wire Resolution into Publish Pipeline
- **File:** `src/services/posts.rs`
- In `PostService::publish()`, before calling `provider.publish()`:
  ```rust
  let resolved_media: Vec<MediaAttachment> = media.iter()
      .map(|m| async {
          let url = provider.resolve_media_url(m, &config).await.unwrap_or_else(|_| m.url.clone());
          MediaAttachment { url, ..m.clone() }
      })
      .collect();
  ```

#### Step 2.4: Ensure `serve_media` Endpoint is Publicly Accessible
- Verify `GET /api/media/:id` works without auth (already does in single-user mode)
- Add note to deployment docs: social-forge must be reachable from platform APIs
- For self-hosted: ensure firewall/port forwarding exposes the media endpoint

---

### Phase 3: Carousel Workflow Integration (2-3 days)

**Goal:** End-to-end carousel posting from local files.

#### Step 3.1: Add `create_carousel_post` MCP Tool
- **File:** `src/mcp/tools_posts.rs`
- New tool that orchestrates:
  1. Accept `local_paths: Vec<String>` + `content` + `integration_ids`
  2. Batch-upload all paths via `media_upload_batch`
  3. Stage post with the returned media URLs
  4. Return staged post IDs

```rust
pub struct CreateCarouselPostInput {
    pub local_paths: Vec<String>,     // Local file paths for slides
    pub content: String,               // Caption/text
    pub integration_ids: Vec<String>,  // Target platforms
    pub settings: Option<serde_json::Value>,
    pub scheduled_at: Option<String>,
    pub first_comment: Option<String>,
}

pub struct CreateCarouselPostOutput {
    pub staged: Vec<StagedPostInfo>,
    pub media_uploaded: Vec<MediaUploadOutput>,
    pub total_posts: usize,
}
```

#### Step 3.2: Add `social-forge carousel` CLI Command
```bash
social-forge carousel post \
  --slides /path/to/slide_1.png,/path/to/slide_2.png \
  --content "10 Tips for Productivity" \
  --platforms instagram \
  --first-comment "Swipe for more tips!"
```

#### Step 3.3: Platform Validation for Carousels
- **File:** `src/services/staging.rs`
- Add carousel-specific validation:
  - Instagram: max 10 items, images or videos (not mixed)
  - Facebook: max 10 items for carousel posts
  - LinkedIn: document share for PDFs, image share for images
  - X/Twitter: max 4 images

#### Step 3.4: Update StagingRequest for Explicit Media Types
- **File:** `src/services/staging.rs`
- Extend `StagingRequest` to support:
  ```rust
  pub struct MediaItem {
      pub url: String,
      pub mime_type: Option<String>,
      pub alt: Option<String>,
  }
  ```
- Backward compatible: still accepts `[{"url": "..."}]` format

---

### Phase 4: Polish & Error Handling (1-2 days)

#### Step 4.1: File Size & Format Validation
- **File:** `src/mcp/tools_media.rs`
- Add validation in all upload tools:
  - Max file size: 50MB (already exists)
  - Supported formats: JPEG, PNG, GIF, WebP, MP4, MOV
  - MIME type detection from file extension and magic bytes

#### Step 4.2: Error Recovery for Batch Uploads
- If one file in a batch fails, continue with others
- Return partial results with error details per file

#### Step 4.3: Update AGENTS.md Documentation
- Document the new MCP tools
- Add carousel workflow examples
- Add troubleshooting section for media upload issues

#### Step 4.4: Add Unit Tests
- Test `upload_from_path_mcp` with mock filesystem
- Test `media_upload_batch` with multiple files
- Test `media_upload_from_url` with mock HTTP server
- Test provider media resolution (local → public URL)

---

## Part 4: Postiz-App Patterns Worth Adopting

### 4.1 Storage Abstraction (Future Enhancement)
Postiz's `IUploadProvider` interface is elegant:
```typescript
interface IUploadProvider {
  uploadSimple(path: string): Promise<string>;
  uploadFile(file: Express.Multer.File): Promise<string>;
  removeFile(filePath: string): Promise<void>;
}
```

**Recommendation for social-forge:** Not needed immediately since social-forge uses local filesystem + DB. But when S3/R2 support is needed, adopt this pattern:
- Create `src/storage/mod.rs` with `StorageProvider` trait
- Implement `LocalStorageProvider` and `S3StorageProvider`
- `UploadFactory::create()` selects backend by config

### 4.2 `readOrFetch` Pattern
Postiz's `readOrFetch` utility is a great pattern for provider-level media handling:
```typescript
async function readOrFetch(urlOrPath: string): Promise<{ buffer: Buffer, mimeType: string }> {
  if (isUrl(urlOrPath)) {
    return fetchUrl(urlOrPath);
  } else {
    return readLocalFile(urlOrPath);
  }
}
```

**Recommendation:** Implement this as a utility in `src/social/common.rs`:
```rust
pub async fn read_or_fetch(url_or_path: &str) -> Result<(Vec<u8>, String), ProviderError> {
    if url_or_path.starts_with("http://") || url_or_path.starts_with("https://") {
        // Download from URL
        let resp = reqwest::get(url_or_path).await?;
        let bytes = resp.bytes().await?.to_vec();
        let mime = mime::Guess::from_path(url_or_path).first_or_octet_stream();
        Ok((bytes, mime.to_string()))
    } else {
        // Read from local path
        let bytes = tokio::fs::read(url_or_path).await?;
        let mime = mime::Guess::from_path(url_or_path).first_or_octet_stream();
        Ok((bytes, mime.to_string()))
    }
}
```

This is already partially implemented in X's `fetch_media_bytes()` — extend it as a shared utility.

### 4.3 Media Validation Layer
Postiz validates media per-provider BEFORE publishing:
- `checkValidity()` runs format checks, dimension checks, count limits
- Returns specific error codes for each validation failure

**Recommendation:** Add `validate_media()` to `SocialProvider` trait:
```rust
fn validate_media(&self, media: &[MediaAttachment]) -> Result<(), String> {
    Ok(()) // Default: no validation
}

// Instagram override:
fn validate_media(&self, media: &[MediaAttachment]) -> Result<(), String> {
    if media.is_empty() {
        return Err("Instagram requires at least one media attachment".into());
    }
    if media.len() > 10 {
        return Err("Instagram carousel supports max 10 items".into());
    }
    // Check for mixed media types
    let has_video = media.iter().any(|m| m.mime_type.starts_with("video/"));
    let has_image = media.iter().any(|m| m.mime_type.starts_with("image/"));
    if has_video && has_image {
        return Err("Instagram carousels cannot mix images and videos".into());
    }
    Ok(())
}
```

---

## Part 5: Testing Strategy

### 5.1 Unit Tests

```rust
// src/mcp/tools_media.rs - add tests
#[tokio::test]
async fn test_upload_from_path_mcp() {
    // Create temp file, upload via tool, verify DB entry
}

#[tokio::test]
async fn test_upload_batch() {
    // Create multiple temp files, batch upload, verify all entries
}

#[tokio::test]
async fn test_upload_from_url() {
    // Mock HTTP server, download URL, verify storage
}

#[tokio::test]
async fn test_provider_media_resolution() {
    // Test Instagram's resolve_media_url with local paths
}
```

### 5.2 Integration Test: Carousel End-to-End

```bash
# 1. Upload local files via MCP
social-forge mcp-call posts_media_upload_batch \
  --json '{"paths": ["/tmp/slide_1.png", "/tmp/slide_2.png", "/tmp/slide_3.png"]}'

# 2. Stage carousel post
social-forge mcp-call posts_stage \
  --json '{"content": "Check out these slides!", "media": [{"url": "/api/media/id1"}, {"url": "/api/media/id2"}, {"url": "/api/media/id3"}], "integration_ids": ["ig-account-uuid"]}'

# 3. Publish
social-forge mcp-call posts_publish --json '{"id": "staged-post-uuid"}'
```

### 5.3 E2E Test with Mock Provider

```rust
#[tokio::test]
async fn test_carousel_publish_mock_instagram() {
    // 1. Create temp images
    // 2. Upload via MCP tools
    // 3. Stage with mock Instagram integration
    // 4. Publish (mock Graph API)
    // 5. Verify carousel containers were created correctly
}
```

---

## Part 6: Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Local server not reachable from platform APIs | High | Document requirement; add health check; consider ngrok/tunnel for dev |
| Large file uploads via MCP timeout | Medium | Add progress reporting; chunked upload for large files |
| Provider rate limits during batch upload | Medium | Add delays between uploads; respect 429 responses |
| File format incompatibility | Low | Validate before upload; auto-convert JPEG if needed |
| Disk space exhaustion | Low | Add cleanup job for old media; configurable retention |

---

## Part 7: File Reference

### social-forge Files to Modify
| File | Changes |
|------|---------|
| `src/mcp/tools_media.rs` | Add `upload_from_path_mcp`, `upload_batch`, `upload_from_url` |
| `src/mcp/mod.rs` | Register new MCP tools |
| `src/cli/mod.rs` | Add `MediaAction::UploadBatch` |
| `src/cli/run.rs` | Handle batch upload CLI command |
| `src/cli/mcp_bridge.rs` | Bridge new tools for CLI→MCP |
| `src/social/mod.rs` | Add `resolve_media_url()` to trait |
| `src/social/instagram.rs` | Implement `resolve_media_url()` |
| `src/social/facebook.rs` | Implement `resolve_media_url()` |
| `src/social/threads.rs` | Implement `resolve_media_url()` |
| `src/social/common.rs` | Add `read_or_fetch()` utility |
| `src/services/posts.rs` | Wire media resolution into publish |
| `src/services/staging.rs` | Add carousel validation |
| `src/mcp/tools_posts.rs` | Add `create_carousel_post` tool |

### postiz-app Reference Files
| File | Pattern to Adopt |
|------|-----------------|
| `libraries/helpers/src/utils/read.or.fetch.ts` | URL-or-path unified fetch |
| `libraries/nestjs-libraries/src/upload/upload.interface.ts` | Storage abstraction interface |
| `libraries/nestjs-libraries/src/upload/upload.factory.ts` | Storage backend selection |
| `libraries/nestjs-libraries/src/integrations/social.abstract.ts` | Media validation pattern |
| `libraries/nestjs-libraries/src/chat/tools/upload.from.url.tool.ts` | URL-to-media conversion |
| `libraries/nestjs-libraries/src/integrations/social/instagram.provider.ts` | Provider media validation |

---

## Implementation Priority

| Priority | Task | Effort | Impact |
|----------|------|--------|--------|
| 🔴 P0 | MCP `media_upload_from_path` tool | 2h | Unblocks local file uploads |
| 🔴 P0 | MCP `media_upload_batch` tool | 3h | Enables carousel workflows |
| 🟠 P1 | Provider `resolve_media_url()` for IG/FB | 4h | Enables publishing to URL-dependent platforms |
| 🟠 P1 | `media upload-batch` CLI command | 2h | CLI parity |
| 🟡 P2 | `create_carousel_post` composite tool | 4h | One-shot carousel creation |
| 🟡 P2 | `read_or_fetch()` shared utility | 2h | Cleaner provider media handling |
| 🟢 P3 | Media validation per-provider | 3h | Better error messages |
| 🟢 P3 | Storage abstraction (S3/R2) | 8h | Production-ready storage |
