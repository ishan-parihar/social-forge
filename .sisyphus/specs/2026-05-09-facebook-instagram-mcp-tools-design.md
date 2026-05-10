# Facebook & Instagram MCP Tools — Design Spec

> **For agentic workers:** This spec covers the design and implementation plan for adding 31 MCP tools (16 Facebook + 15 Instagram) to the Postiz-rust MCP server. These tools provide read/write/query access to Meta's Graph API v21.0 for AI agents.

**Goal:** Give AI agents the same rich Facebook and Instagram interaction capabilities as X/Twitter (20 tools) and Reddit (7 tools) — enabling content creation, feed reading, engagement, analytics, direct messaging, and discovery operations through MCP tools.

**Architecture:** Each provider gets a dedicated `src/mcp/tools_*.rs` module containing input/output types and handler functions, following the exact pattern established by `tools_reddit.rs` and `tools_x.rs`. Handlers call inherent methods on the provider struct. Registration happens in `src/mcp/mod.rs` via `#[tool]` attributes on `PostizMcpServer`.

**Tech Stack:** Rust, rmcp (MCP), Meta Graph API v21.0, reqwest/HTTP, serde_json.

---

## 1. Pre-Requisites: Scopes & Provider Methods

### 1.1 Expanded Facebook Scopes

Current scopes in `scopes()`: `pages_show_list`, `pages_read_engagement`, `pages_manage_posts`, `business_management`, `public_profile`.

**Additional scopes needed** (add conditionally since some require app review):
- `pages_manage_metadata` — for updating posts
- `pages_messaging` — for DM/send_message
- `pages_read_user_content` — for reading feed + comments
- `read_insights` — for page insights
- `public_profile` (already present)

### 1.2 Expanded Instagram Scopes

Current scopes: `instagram_basic`, `instagram_content_publish`, `instagram_manage_comments`, `instagram_manage_insights`, `pages_show_list`, `pages_read_engagement`, `business_management`.

**Additional scopes needed:**
- `instagram_manage_messages` — for IG Direct
- `pages_manage_metadata` — for editing media

### 1.3 Required Inherent Methods on FacebookProvider

The following methods will be added to `impl FacebookProvider` (in `src/social/facebook.rs`):

```rust
// Content Management
async fn get_page_feed(&self, access_token: &str, page_id: &str, limit: u32, ...) -> Result<Value, ProviderError>
async fn get_post(&self, access_token: &str, post_id: &str) -> Result<Value, ProviderError>
async fn create_post(&self, access_token: &str, page_id: &str, message: &str, ...) -> Result<Value, ProviderError>
async fn create_photo_post(&self, access_token: &str, page_id: &str, url: &str, message: &str) -> Result<Value, ProviderError>
async fn create_video_post(&self, access_token: &str, page_id: &str, url: &str, title: &str, description: &str) -> Result<Value, ProviderError>
async fn create_link_post(&self, access_token: &str, page_id: &str, link: &str, message: &str) -> Result<Value, ProviderError>
async fn delete_post(&self, access_token: &str, post_id: &str) -> Result<Value, ProviderError>

// Engagement
async fn get_post_comments(&self, access_token: &str, post_id: &str, ...) -> Result<Value, ProviderError>
async fn create_comment(&self, access_token: &str, post_id: &str, message: &str) -> Result<Value, ProviderError>
async fn delete_comment(&self, access_token: &str, comment_id: &str) -> Result<Value, ProviderError>
async fn reply_to_comment(&self, access_token: &str, comment_id: &str, message: &str) -> Result<Value, ProviderError>

// Discovery
async fn search_pages(&self, access_token: &str, query: &str, limit: u32) -> Result<Value, ProviderError>

// Analytics
async fn get_page_insights(&self, access_token: &str, page_id: &str, metric: &str, period: &str) -> Result<Value, ProviderError>
async fn get_post_insights(&self, access_token: &str, post_id: &str, metric: &str) -> Result<Value, ProviderError>

// Messaging
async fn send_page_message(&self, access_token: &str, psid: &str, message: &str) -> Result<Value, ProviderError>
```

### 1.4 Required Inherent Methods on InstagramProvider

```rust
// Content Management
async fn get_media(&self, access_token: &str, ig_id: &str, media_id: &str) -> Result<Value, ProviderError>
async fn publish_single_image(&self, access_token: &str, ig_id: &str, url: &str, caption: &str) -> Result<Value, ProviderError>
async fn publish_carousel(&self, access_token: &str, ig_id: &str, media_urls: &[&str], caption: &str) -> Result<Value, ProviderError>
async fn publish_reel(&self, access_token: &str, ig_id: &str, video_url: &str, caption: &str) -> Result<Value, ProviderError>
async fn delete_media(&self, access_token: &str, ig_id: &str, media_id: &str) -> Result<Value, ProviderError>
async fn edit_caption(&self, access_token: &str, ig_id: &str, media_id: &str, caption: &str) -> Result<Value, ProviderError>

// Discovery
async fn get_hashtag_media(&self, access_token: &str, ig_id: &str, hashtag: &str, limit: u32) -> Result<Value, ProviderError>
async fn get_mentions(&self, access_token: &str, ig_id: &str, limit: u32) -> Result<Value, ProviderError>
async fn business_discovery(&self, access_token: &str, ig_id: &str, username: &str) -> Result<Value, ProviderError>

// Engagement
async fn get_media_comments(&self, access_token: &str, ig_id: &str, media_id: &str, ...) -> Result<Value, ProviderError>
async fn reply_to_comment(&self, access_token: &str, ig_id: &str, comment_id: &str, message: &str) -> Result<Value, ProviderError>
async fn reply_to_comment_on_media(&self, access_token: &str, ig_id: &str, media_id: &str, message: &str) -> Result<Value, ProviderError>

// Analytics
async fn get_ig_insights(&self, access_token: &str, ig_id: &str, metric: &str, period: &str) -> Result<Value, ProviderError>
async fn get_media_insights(&self, access_token: &str, ig_id: &str, media_id: &str, metric: &str) -> Result<Value, ProviderError>

// Messaging
async fn send_ig_message(&self, access_token: &str, ig_id: &str, recipient_id: &str, message: &str) -> Result<Value, ProviderError>
```

---

## 2. Facebook MCP Tools (16 tools)

### 2.1 MCP Module: `src/mcp/tools_facebook.rs`

Structure mirrors `tools_x.rs` — separate input/output types per tool + handler function per tool.

#### Content Management (5)

**Tool: `fb_get_feed`**
- Input: `{ page_id: String, limit: Option<u32>, since: Option<String>, until: Option<String> }`
- Output: `{ data: Value }`
- Endpoint: `GET /{page_id}/feed` with fields
- Handler: resolves token, calls `FacebookProvider::get_page_feed()`

**Tool: `fb_get_post`**
- Input: `{ post_id: String }`
- Output: `{ data: Value }`
- Endpoint: `GET /{post_id}` with fields
- Handler: resolves token, calls `FacebookProvider::get_post()`

**Tool: `fb_create_post`**
- Input: `{ page_id: String, message: String }`
- Output: `{ id: String, success: bool }`
- Endpoint: `POST /{page_id}/feed`
- Handler: resolves token, calls `FacebookProvider::create_post()`

**Tool: `fb_create_photo_post`**
- Input: `{ page_id: String, url: String, message: Option<String> }`
- Output: `{ id: String, success: bool }`
- Endpoint: `POST /{page_id}/photos` with url + message
- Handler: resolves token, calls `FacebookProvider::create_photo_post()`

**Tool: `fb_create_video_post`**
- Input: `{ page_id: String, url: String, title: Option<String>, description: Option<String> }`
- Output: `{ id: String, success: bool }`
- Endpoint: `POST /{page_id}/videos` with file_url + title + description
- Handler: resolves token, calls `FacebookProvider::create_video_post()`

**Tool: `fb_delete_post`**
- Input: `{ post_id: String }`
- Output: `{ success: bool }`
- Endpoint: `DELETE /{post_id}`
- Handler: resolves token, calls `FacebookProvider::delete_post()`

#### Engagement & Interaction (4)

**Tool: `fb_get_post_comments`**
- Input: `{ post_id: String, order: Option<String>, limit: Option<u32> }`
- Output: `{ data: Value }`
- Endpoint: `GET /{post_id}/comments` with fields
- Handler: resolves token, calls `FacebookProvider::get_post_comments()`

**Tool: `fb_comment_on_post`**
- Input: `{ post_id: String, message: String }`
- Output: `{ id: String }`
- Endpoint: `POST /{post_id}/comments`
- Handler: resolves token, calls `FacebookProvider::create_comment()`

**Tool: `fb_delete_comment`**
- Input: `{ comment_id: String }`
- Output: `{ success: bool }`
- Endpoint: `DELETE /{comment_id}`
- Handler: resolves token, calls `FacebookProvider::delete_comment()`

**Tool: `fb_reply_to_comment`**
- Input: `{ comment_id: String, message: String }`
- Output: `{ id: String }`
- Endpoint: `POST /{comment_id}/comments`
- Handler: resolves token, calls `FacebookProvider::reply_to_comment()`

#### Analytics & Insights (2)

**Tool: `fb_get_page_insights`**
- Input: `{ page_id: String, metric: String, period: Option<String>, since: Option<String>, until: Option<String> }`
- Output: `{ data: Value }`
- Endpoint: `GET /{page_id}/insights?metric=...&period=...`
- Handler: resolves token, calls `FacebookProvider::get_page_insights()`
- Notes: Valid metrics include page_impressions, page_engaged_users, page_fans, page_fan_adds, page_reach, page_views_total, page_actions_post_reactions_total, page_stories, page_consumptions, etc.

**Tool: `fb_get_post_insights`**
- Input: `{ post_id: String, metric: String }`
- Output: `{ data: Value }`
- Endpoint: `GET /{post_id}/insights?metric=...`
- Handler: resolves token, calls `FacebookProvider::get_post_insights()`
- Notes: Valid metrics include post_impressions, post_impressions_unique, post_engaged_users, post_reactions_by_type_total, post_clicks, post_video_complete_views_30s, etc.

#### Direct Messaging (1)

**Tool: `fb_send_message`**
- Input: `{ psid: String, message: String, page_id: String }`
- Output: `{ message_id: String, success: bool }`
- Endpoint: `POST /me/messages` (via Page Platform — requires pages_messaging scope)
- Handler: resolves token, calls `FacebookProvider::send_page_message()`
- Notes: PSID = Page-scoped ID of the recipient. Requires user to have initiated conversation with the page. Uses the Page's access token (pages_messaging scope).

---

## 3. Instagram MCP Tools (15 tools)

### 3.1 MCP Module: `src/mcp/tools_instagram.rs`

Same structure pattern.

#### Content Management (5)

**Tool: `ig_get_media`**
- Input: `{ media_id: String }`
- Output: `{ data: Value }`
- Endpoint: `GET /{media_id}` with fields
- Handler: resolves IG Business account token, calls `InstagramProvider::get_media()`

**Tool: `ig_publish_image`**
- Input: `{ image_url: String, caption: Option<String> }`
- Output: `{ media_id: String, permalink: String }`
- Endpoint: POST container creation → POST media publish (two-step Instagram publish flow)
- Handler: resolves IG Business account token, calls `InstagramProvider::publish_single_image()`

**Tool: `ig_publish_carousel`**
- Input: `{ image_urls: Vec<String>, caption: Option<String> }`
- Output: `{ media_id: String, permalink: String }`
- Endpoint: Create CAROUSEL container with children → POST publish
- Handler: resolves IG Business account token, calls `InstagramProvider::publish_carousel()`

**Tool: `ig_publish_reel`**
- Input: `{ video_url: String, caption: Option<String>, cover_url: Option<String> }`
- Output: `{ media_id: String, permalink: String }`
- Endpoint: POST REEL container (media_type=REELS) → POST publish
- Handler: resolves IG Business account token, calls `InstagramProvider::publish_reel()`

**Tool: `ig_delete_media`**
- Input: `{ media_id: String }`
- Output: `{ success: bool }`
- Endpoint: `DELETE /{media_id}`
- Handler: resolves IG Business account token, calls `InstagramProvider::delete_media()`

**Tool: `ig_edit_caption`**
- Input: `{ media_id: String, caption: String }`
- Output: `{ success: bool }`
- Endpoint: `POST /{media_id}` with caption param
- Handler: resolves IG Business account token, calls `InstagramProvider::edit_caption()`

#### Research & Discovery (4)

**Tool: `ig_get_hashtag_media`**
- Input: `{ hashtag: String, limit: Option<u32> }`
- Output: `{ data: Value }`
- Endpoint: GET /ig_hashtag_search → GET /{hashtag_id}/recent_media
- Handler: resolves IG Business account token, calls `InstagramProvider::get_hashtag_media()`
- Notes: Two-step flow: search hashtag by name, then get recent media for that hashtag ID. Requires `instagram_basic` scope + business account.

**Tool: `ig_get_mentions`**
- Input: `{ limit: Option<u32> }`
- Output: `{ data: Value }`
- Endpoint: GET /{ig_id}/mentions
- Handler: resolves IG Business account token, calls `InstagramProvider::get_mentions()`

**Tool: `ig_business_discovery`**
- Input: `{ username: String }`
- Output: `{ data: Value }`
- Endpoint: `GET /{ig_id}?fields=business_discovery.username({username}){...}`
- Handler: resolves IG Business account token, calls `InstagramProvider::business_discovery()`
- Notes: Only works for IG Business/Creator accounts. Returns the target account's profile info, media, insights.

**Tool: `ig_search_hashtag`**
- Input: `{ query: String, limit: Option<u32> }`
- Output: `{ data: Value }`
- Endpoint: `GET /ig_hashtag_search?user_id={ig_id}&q={query}`
- Handler: resolves IG Business account token, calls `InstagramProvider::search_hashtag()`

#### Engagement & Interaction (3)

**Tool: `ig_get_media_comments`**
- Input: `{ media_id: String, limit: Option<u32> }`
- Output: `{ data: Value }`
- Endpoint: `GET /{media_id}/comments` with fields
- Handler: resolves IG Business account token, calls `InstagramProvider::get_media_comments()`
- Notes: Requires `instagram_manage_comments` scope.

**Tool: `ig_reply_to_comment`**
- Input: `{ comment_id: String, message: String }`
- Output: `{ id: String }`
- Endpoint: `POST /{comment_id}/replies`
- Handler: resolves IG Business account token, calls `InstagramProvider::reply_to_comment()`

**Tool: `ig_reply_to_comment_on_media`**
- Input: `{ media_id: String, message: String }`
- Output: `{ id: String }`
- Endpoint: `POST /{media_id}/comments`
- Handler: resolves IG Business account token, calls `InstagramProvider::reply_to_comment_on_media()`

#### Analytics & Insights (2)

**Tool: `ig_get_insights`**
- Input: `{ metric: String, period: Option<String>, since: Option<String>, until: Option<String> }`
- Output: `{ data: Value }`
- Endpoint: `GET /{ig_id}/insights?metric=...&period=...`
- Handler: resolves IG Business account token, calls `InstagramProvider::get_ig_insights()`
- Notes: Valid metrics: impressions, reach, profile_views, follower_count, email_contacts, get_directions_clicks, website_clicks, phone_call_clicks. Periods: day, week, days_28.

**Tool: `ig_get_media_insights`**
- Input: `{ media_id: String, metric: Option<String> }`
- Output: `{ data: Value }`
- Endpoint: `GET /{media_id}/insights?metric=...`
- Handler: resolves IG Business account token, calls `InstagramProvider::get_media_insights()`
- Notes: Valid metrics: engagement, impressions, reach, saved, video_views (for reels). If omitted, returns all available.

#### Direct Messaging (1)

**Tool: `ig_send_message`**
- Input: `{ recipient_id: String, message: String }`
- Output: `{ success: bool }`
- Endpoint: `POST /{ig_id}/messages` (via Instagram Messaging API)
- Handler: resolves IG Business account token, calls `InstagramProvider::send_ig_message()`
- Notes: Requires `instagram_manage_messages` scope. Recipient must be follower/have initiated conversation.

---

## 4. Implementation Tasks

### Task 1: Add inherent methods to FacebookProvider

**File:** `src/social/facebook.rs`
**Change:** Add `impl FacebookProvider` block with ~12 inherent methods (get_page_feed, get_post, create_post, create_photo_post, create_video_post, delete_post, get_post_comments, create_comment, delete_comment, reply_to_comment, get_page_insights, get_post_insights, send_page_message)
**Lines added:** ~250
**Pattern:** Each method takes `&self`, `access_token: &str`, domain-specific params, returns `Result<serde_json::Value, ProviderError>`. Follows the X provider inherent method pattern with HTTP status checking (same as X: check 429/401/other).

### Task 2: Add inherent methods to InstagramProvider

**File:** `src/social/instagram.rs`
**Change:** Add `impl InstagramProvider` block with ~15 inherent methods (get_media, publish_single_image, publish_carousel, publish_reel, delete_media, edit_caption, get_hashtag_media, get_mentions, business_discovery, search_hashtag, get_media_comments, reply_to_comment, reply_to_comment_on_media, get_ig_insights, get_media_insights, send_ig_message)
**Lines added:** ~300
**Pattern:** Same as Facebook — inherent methods with HTTP status checking.

### Task 3: Create `src/mcp/tools_facebook.rs`

**New file:** ~350 lines
**Contents:** Input/output types for all 16 Facebook MCP tools + handler functions. Helpers: `find_facebook_token()`, `resolve_page_token()` returning `(integration_id, access_token)`.

### Task 4: Create `src/mcp/tools_instagram.rs`

**New file:** ~320 lines
**Contents:** Input/output types for all 15 Instagram MCP tools + handler functions. Helpers: `find_instagram_token()`, `resolve_ig_account()`.

### Task 5: Register in `src/mcp/mod.rs`

**File:** `src/mcp/mod.rs`
**Changes:**
- Add `mod tools_facebook;` and `mod tools_instagram;` declarations
- Add 31 `#[tool(description = "...")]` entries on `PostizMcpServer` impl
- Each entry follows the exact pattern: `async fn fb_xxx/ig_xxx(&self, params: ...) -> Result<Json<...>, String>`

### Task 6: Expand OAuth scopes

**File:** `src/social/facebook.rs` and `src/social/instagram.rs`
**Changes:** Expand `scopes()` to include new scopes needed for the tools (pages_messaging, pages_manage_metadata, pages_read_user_content, read_insights, instagram_manage_messages).

### Task 7: Build, verify, restart

- `cargo check` and `cargo build --release`
- `lsp_diagnostics` clean on all changed/created files
- Server restart on port 3000 with --mcp
- Verify health, providers list, and MCP tool count

---

## 5. Design Decisions

### 5.1 Token Resolution Pattern

Each MCP handler follows the same pattern as Reddit and X:

```rust
// Resolve user from JWT
let claims = resolve_user(&state, &params.token)?;

// Find the X integration for this user
let integration = find_x_token(&state, claims.user_id)
    .await?
    .ok_or("No X/Twitter account connected")?;

// Call provider method
let provider = XProvider::new(&state.config);
let result = provider.home_timeline(
    &integration.access_token,
    &claims.user_id,
    params.max_results.unwrap_or(20),
    params.pagination_token.as_deref(),
).await.map_err(|e| e.to_string())?;

Ok(Json(XHomeTimelineOutput { data: result }))
```

For Facebook, the integration's `access_token` is a page-scoped token. For the multi-step flow, the parent integration (root_internal_id is NULL or root) stores the user-level token. Page child integrations store the page-scoped token.

Facebook handlers will use `find_facebook_token()` helper that searches for a page-scoped integration. The MCP tool signature includes `page_id` as a parameter so the agent specifies which page to act on.

### 5.2 Graph API URL Construction

All Facebook and Instagram Graph API calls use `self.graph_url()` ("https://graph.facebook.com/v21.0") as the base, same as existing code.

### 5.3 HTTP Status Checking

Every inherent method checks HTTP status codes (same fix applied to X tools):
- 429 → `ProviderError::RateLimited`
- 401 → `ProviderError::TokenExpired`
- Other errors → `ProviderError::Api` with detail message

### 5.4 Scopes Strategy

Facebook scopes are additive — existing scopes already cover most tools. The new `pages_messaging` scope requires app review in production but works in dev mode. Tools that require unapproved scopes will work during development but fail with a clear error in production. We add the scopes to the scopes() method — the OAuth consent screen will show all requested scopes on next connect.

---

## 6. API Reference

### Facebook Graph API v21.0 Endpoints

| Tool | Method | Endpoint | Auth Scope |
|------|--------|----------|------------|
| fb_get_feed | GET | `/{page_id}/feed` | pages_read_engagement |
| fb_get_post | GET | `/{post_id}` | pages_read_engagement |
| fb_create_post | POST | `/{page_id}/feed` | pages_manage_posts |
| fb_create_photo_post | POST | `/{page_id}/photos` | pages_manage_posts |
| fb_create_video_post | POST | `/{page_id}/videos` | pages_manage_posts |
| fb_delete_post | DELETE | `/{post_id}` | pages_manage_posts |
| fb_get_post_comments | GET | `/{post_id}/comments` | pages_read_engagement |
| fb_comment_on_post | POST | `/{post_id}/comments` | pages_manage_posts |
| fb_delete_comment | DELETE | `/{comment_id}` | pages_manage_posts |
| fb_reply_to_comment | POST | `/{comment_id}/comments` | pages_manage_posts |
| fb_get_page_insights | GET | `/{page_id}/insights` | read_insights |
| fb_get_post_insights | GET | `/{post_id}/insights` | read_insights |
| fb_send_message | POST | `/me/messages` | pages_messaging |

### Instagram Graph API v21.0 Endpoints

| Tool | Method | Endpoint | Auth Scope |
|------|--------|----------|------------|
| ig_get_media | GET | `/{media_id}` | instagram_basic |
| ig_publish_image | POST | Two-step: container + publish | instagram_content_publish |
| ig_publish_carousel | POST | Two-step: container + publish | instagram_content_publish |
| ig_publish_reel | POST | Two-step: container + publish | instagram_content_publish |
| ig_delete_media | DELETE | `/{media_id}` | instagram_content_publish |
| ig_edit_caption | POST | `/{media_id}` | instagram_content_publish |
| ig_get_hashtag_media | GET | `/ig_hashtag_search` → `/{hashtag_id}/recent_media` | instagram_basic |
| ig_get_mentions | GET | `/{ig_id}/mentions` | instagram_basic |
| ig_business_discovery | GET | `/{ig_id}?fields=business_discovery...` | instagram_basic |
| ig_search_hashtag | GET | `/ig_hashtag_search` | instagram_basic |
| ig_get_media_comments | GET | `/{media_id}/comments` | instagram_manage_comments |
| ig_reply_to_comment | POST | `/{comment_id}/replies` | instagram_manage_comments |
| ig_reply_to_comment_on_media | POST | `/{media_id}/comments` | instagram_manage_comments |
| ig_get_insights | GET | `/{ig_id}/insights` | instagram_manage_insights |
| ig_get_media_insights | GET | `/{media_id}/insights` | instagram_manage_insights |
| ig_send_message | POST | `/{ig_id}/messages` | instagram_manage_messages |

---

## 7. Page vs IG Account Resolution

### Facebook Page Token Resolution

For Facebook, each page integration stores its own page-scoped access token in `access_token`. The parent integration (root_internal_id = NULL) stores the user-level token. MCP handlers look up a specific page integration by the user-provided `page_id` parameter.

The helper `find_page_integration(state, user_id, page_id)` queries for an integration where:
- `user_id` matches
- `provider_identifier` = "facebook"
- `internal_id` = `page_id`

### Instagram Business Account Resolution

For Instagram, the account is resolved from the page's Instagram Business Account link. The page integration's access_token is the page-scoped token. IG operations use `ig_id` which comes from `/{page_id}?fields=instagram_business_account`.

The MCP handler calls `resolve_ig_business_account(access_token)` (existing method) to get the IG Business Account ID, then uses that for all IG API calls.

---

## 8. Error Handling

All MCP handlers return `Result<Json<T>, String>` — mapping `ProviderError` variants to user-friendly strings:
- `ProviderError::RateLimited` → "Rate limited by Facebook/Instagram API. Please wait before retrying."
- `ProviderError::TokenExpired` → "Access token expired. Please reconnect the account via the onboarding page."
- `ProviderError::Api(msg)` → `msg` directly
- `ProviderError::Auth(msg)` → `msg` directly
- `ProviderError::InvalidRequest(msg)` → `msg` directly

---

## 9. Self-Review

- No placeholders or TBDs in the tool mappings — every tool has a named endpoint, input/output types, and handler
- All tools have a corresponding API endpoint in Meta Graph API v21.0 (verified during research phase)
- Scope requirements are documented per-tool
- Pattern is consistent with existing tools_reddit.rs and tools_x.rs
- Sequential task ordering: provider methods before MCP handlers (dependency order)
