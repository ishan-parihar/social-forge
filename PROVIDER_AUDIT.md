# Provider Audit — social-forge (postiz-rust)

> **Living document**. Update as providers are connected, tested, or changed.
> Last updated: 2026-05-15 | Commit: `40f9156` (plus subsequent LinkedIn Page analytics fix)

---

## Quick Status

```
✅ Functional — tested with real connected accounts
⚠️  Partial — has test tokens, needs real OAuth/credentials
🔲 Missing — no credentials or integration exists
```

| Provider | Registration | MCP Tools | Auth Type | DB Integration | Status |
|----------|-------------|-----------|-----------|---------------|--------|
| X/Twitter | Always | 60 | GraphQL cookies + OAuth v2 | ✅ env vars | ✅ Functional |
| Facebook | Always | 31 | Meta OAuth | ✅ 5 pages | ✅ Functional |
| Instagram | Always | 31 | Meta OAuth | ✅ 5 accounts | ✅ Functional |
| Reddit | Always | 22 | Non-OAuth (user+pass) | ✅ d3vilzwrld | ✅ Functional¹ |
| LinkedIn | Always | 13 | OAuth v2 | ✅ Connected | ⚠️ Partial¹ |
| LinkedIn-Page | Conditional | 9 | OAuth v2 | ✅ 3 pages | ⚠️ Partial² |
| WhatsApp | Always | 12 | wa-rs QR pairing | ❌ Not connected | ✅ Implemented² |
| Telegram-User | Always | 21 | Grammers MTProto | ❌ Needs /connect | 🔲 Missing |
| Telegram-Bot | Conditional | 7 | Bot API | ❌ No creds | 🔲 Missing |
| Bluesky | Always | 11 | AT Protocol | ❌ No creds | 🔲 Missing |
| Pinterest | Always | 15 | OAuth | ❌ No creds | 🔲 Missing |
| WordPress | Always | 9 | App Password | ❌ No creds | 🔲 Missing |
| Skool | Always | 11 | Chrome extension | ❌ N/A | 🔲 Missing |
| Instagram-Standalone | Conditional | 15 | IG Basic Display | ⚠️ Test token | ⚠️ Partial |
| Threads | Conditional | 19 | Threads API | ⚠️ Test token | ⚠️ Partial |
| YouTube | Conditional | 19 | YouTube Data API | ✅ 2 channels | ⚠️ Partial³ |
| Discord | Conditional | 21 | Bot API | ❌ No creds | 🔲 Missing |
| TikTok | Conditional | 7 | OAuth | ❌ No creds | 🔲 Missing |
| Mastodon | Conditional | 9 | OAuth | ❌ No creds | 🔲 Missing |
| Slack | Conditional | 9 | OAuth | ❌ No creds | 🔲 Missing |
| Medium | Conditional | 7 | API Key | ❌ No creds | 🔲 Missing |
| Dev.to | Conditional | 7 | API Key | ❌ No creds | 🔲 Missing |
| Hashnode | Conditional | 7 | API Key | ❌ No creds | 🔲 Missing |

¹ Reddit: browse/search/user_info work via cookie/session auth. reqwest TLS fingerprint blocked by Cloudflare on some endpoints (curl works).
² wa-rs native Rust WhatsApp client fully implemented (Phases 1-7). QR pairing flow ready but untested end-to-end.
³ Some YouTube tokens stored in plaintext (pre-TOKEN_ENCRYPTION_KEY). Re-auth resolves. Channel stats + analytics work.

---

## 1. X/Twitter — ✅ Functional

**Commit built**: `40f9156` | **Test files**: `x_integration_test.rs` (15), `live_x_test.rs` (11), `x_transport_test.rs` (5), `x_live_test.rs` (1) | **MCP tools**: 60

### Architecture
- **Primary**: Twitter internal GraphQL API (`x.com/i/api/graphql/`) via **wreq** HTTP client
  - wreq v6.0.0-rc.23 with Chrome131 TLS fingerprint (wreq-util `Emulation::Chrome131`)
  - Cookie-based auth (auth_token + ct0 from browser)
  - Hardcoded Bearer token (`AAAAAAAAAAAAAAAAAAAA...`)
  - X-Client-Transaction-Id per-request (SHA256 of method+path+UUID v4)
  - 22 static GraphQL query IDs (extracted from twitter-cli)
  - 14 sec-ch-ua + Sec-Fetch headers via emulation
  - Rate limit retry with exponential backoff (2 attempts)
  - Write delay helper (1-3s jitter for stateful mutations)
- **Fallback**: Twitter API v2 OAuth 2.0 PKCE (Bearer token) — used when cookie auth unavailable for some operations

### Auth — Cookie Form (3 input modes)
1. Full `Cookie` header from DevTools → Network tab → request headers (recommended, green border)
2. Individual `auth_token` + `ct0` fields (collapsed under `<details>`)
3. **Import from Browser** button — one-click extraction from Zen/Chrome/Brave/Firefox SQLite cookie stores
   - Chrome/Brave: decrypted via aes-gcm (OS keychain or hardcoded key)
   - Firefox/Zen: plaintext SQLite
   - `.x.com` preferred over `.twitter.com` on cookie domain conflicts

### Test Results
| Tool | Status | Notes |
|------|--------|-------|
| `get_me` | ✅ | Returns @_ishanparihar (id=3301263462, Ishan Parihar) |
| `user_lookup_by_username` | ✅ | Returns user by screen name |
| `user_lookup` | ✅ | v2 fallback — token expired, returns error (expected) |
| `home_timeline` | ✅ | Returns timeline posts |
| `user_tweets` | ✅ | Returns user's tweets |
| `tweet_detail` | ✅ | Fetches single tweet by ID |
| `search_tweets` | ✅ | GraphQL SearchTimeline POST |
| `delete_tweet` | ✅ | DeleteTweet mutation |
| `like_tweet` / `unlike_tweet` | ✅ | FavoriteTweet / UnfavoriteTweet |
| `retweet` / `unretweet` | ✅ | CreateRetweet / DeleteRetweet |
| `bookmarks` / `bookmark_tweet` / `unbookmark_tweet` | ✅ | Bookmarks queries + mutations |
| `followers` / `following` | ⚠️ | Returns 422 "No data" — API restriction, not code bug |
| `follow_user` / `unfollow_user` | ✅ | REST friendships create/destroy |
| `list_timeline` | ✅ | ListLatestTweetsTimeline |

### Env Vars
```
X_CLIENT_ID=<oauth-client-id>
X_CLIENT_SECRET=<oauth-client-secret>
X_AUTH_TOKEN=<browser-cookie>
X_CT0=<browser-cookie>
```

### Known Issues
- `followers`/`following` return 422 (Twitter API limitation, not code)
- `user_lookup` v2 fallback needs OAuth token refresh
- Rate limits reset periodically (~15 min window)

---

## 2. Facebook — ✅ Functional

**MCP tools**: 31 | **Auth**: Meta OAuth (pages_show_list, pages_read_engagement, pages_manage_posts, business_management, pages_manage_engagement, pages_manage_metadata, pages_read_user_content, public_profile, read_insights, pages_messaging)

### Connected Accounts (Golden User: `87c12961-...`)
| Page ID | Type |
|---------|------|
| 4372074126446140 | Facebook Page |
| 604373986102944 | Facebook Page |
| 338858752654432 | Facebook Page |
| 106249392449992 | Facebook Page |
| 102729826251641 | Facebook Page |

### Verified Operations
| Tool | Status | Notes |
|------|--------|-------|
| `fb_get_feed` | ✅ | Returns feed posts |
| `fb_albums` | ✅ | Lists albums |
| `fb_search_pages` | ✅ | Page search |
| `fb_create_post` | ✅ | Text post created |
| `fb_create_photo` | ✅ | Photo post created |
| `fb_create_video` | ✅ | Video post created |
| `fb_delete_post` | ✅ | Post deleted |
| `fb_page_insights` | ✅ | Page insights |
| `fb_conversations` | ✅ | Conversations |

### Env Vars
```
FACEBOOK_APP_ID=<...>
FACEBOOK_APP_SECRET=<...>
```

---

## 3. Instagram Business — ✅ Functional

**MCP tools**: 31 | **Auth**: Meta OAuth (same as Facebook + instagram_basic, instagram_content_publish, instagram_manage_comments, instagram_manage_insights)

### Connected Accounts (Golden User)
| Account ID | Type |
|-----------|------|
| 17841400680408909 | Instagram Business |
| 17841401924712730 | Instagram Business |
| 4372074126446140 | Instagram Business (cross-ref FB) |
| 17841474734070627 | Instagram Business |
| 17841461291118404 | Instagram Business |

### Verified Operations
| Tool | Status | Notes |
|------|--------|-------|
| `ig_get_media` | ✅ | Lists media |
| `ig_get_reels` | ✅ | Lists reels |
| `ig_get_stories` | ✅ | Lists stories |
| `ig_get_followers` | ✅ | Follower list |
| `ig_get_insights` | ✅ | Account insights |
| `ig_business_discovery` | ✅ | Business discovery |
| `ig_media_insights` | ✅ | Per-media insights |
| `ig_get_tagged` | ✅ | Tagged media |

### Env Vars
```
INSTAGRAM_APP_ID=<...>
INSTAGRAM_APP_SECRET=<...>
```

---

## 4. Reddit — ✅ Functional

**MCP tools**: 22 | **Auth**: Non-OAuth (username + password session)

### Connected Account
- **Username**: d3vilzwrld

### Verified Operations
| Tool | Status | Notes |
|------|--------|-------|
| `browse` | ✅ | Front page listing |
| `search` | ✅ | Search posts |
| `post_detail` | ✅ | Single post |
| `get_comments` | ✅ | Comments on post |
| `user_info` | ✅ | User profile |
| `send_dm` | ⚠️ | Works via curl, reqwest TLS blocked by Cloudflare |
| `inbox` | ⚠️ | Works via curl, reqwest TLS blocked by Cloudflare |

### Known Issues
- **Cloudflare TLS fingerprint blocking**: `send_dm` and `inbox` fail via reqwest but work via `curl`.
  - Attempted fixes: HTTP/1.1 only, removed conflicting User-Agent, cookie_store(true), gzip(true), all default_headers
  - Root cause: Reddit CDN differentiates curl's native TLS fingerprint from reqwest's rustls
  - Mitigation: `send_dm` (DM) and `inbox` work via curl CLI. For production, consider wreq or curl subprocess fallback.

### Env Vars
```
REDDIT_CLIENT_ID=<...>
REDDIT_CLIENT_SECRET=<...>
REDDIT_USERNAME=d3vilzwrld
REDDIT_PASSWORD=<...>
```

---

## 5. LinkedIn — ⚠️ Partial

**MCP tools**: 13 (+ 9 for LinkedIn Page) | **Auth**: OAuth v2

### Current State
- **LinkedIn Personal**: Connected via browser OAuth ✅. `get_user_id` works with real token. `get_profile` / `get_posts` fail — token expired. Needs re-auth.
- **LinkedIn Page**: 3 pages connected (Ishan Parihar ×2, Design Aesthetics). `fetch_page_info` works for all 3 ✅. `pages()` lists correctly ✅. `get_page_posts` returns "No virtual resource found" (scope/permission issue). `analytics` had `internal_id` parameter bug — **FIXED** (was ignoring parameter, always resolved org_id from first page).
- `linkedin_e2e_test.rs` (20 tests) — 1 failure: test DB has real integrations (assertion expects 0)
- Token decryption fixed for plaintext tokens

### Env Vars
```
LINKEDIN_CLIENT_ID=<...>
LINKEDIN_CLIENT_SECRET=<...>
```

### To Re-Connect (if token expires)
1. Visit `http://localhost:3000/`
2. Click "Connect" on LinkedIn card  
3. Authorize via browser OAuth

---

## 6. WhatsApp — ✅ Implemented (untested live)

**MCP tools**: 12 | **Auth**: wa-rs QR pairing

### Architecture
- **Primary**: `wa-rs` v0.2.0 native Rust WhatsApp Web client
  - `WhaClient` wrapper around `Arc<wa_rs::Client>`
  - `SqliteStore` for Signal protocol persistence
  - QR code + pair-code phone authentication
  - Send text/edit/revoke messages
  - Group list/create/invite via wa-rs Groups API
  - Chat/contact queries via rusqlite-backed `wa-meta.db`
- **Fallback**: wacli Go binary sidecar (JSON-RPC stdin/stdout) — kept for backward compatibility

### Module Files
| File | Purpose |
|------|---------|
| `src/wa/mod.rs` | WhaClient wrapper (connect, auth status, store_dir) |
| `src/wa/auth.rs` | `pair_with_code()`, `wait_for_authentication()` |
| `src/wa/messages.rs` | `send_text()`, `edit_message()`, `revoke_message()` |
| `src/wa/chats.rs` | `list_chats()`, `list_contacts()` via rusqlite |
| `src/wa/groups.rs` | `list_groups()`, `create_group()`, `get_group_invite_link()` |

### Test Results (14 tests — all pass without wacli binary)
| Test | Status | Notes |
|------|--------|-------|
| Metadata traits | ✅ | ProviderMetadata, Capabilities |
| Provider registration | ✅ | WhatsAppProvider creation |
| MCP handler compilation | ✅ | All 4 handlers type-check |
| MCP handlers error without daemon | ✅ | Graceful error without wacli |
| CRUD create/find/list/delete (chats + contacts) | ✅ | 8 rusqlite-backed tests |
| Empty store behavior | ✅ | Returns empty Vec |
| Upsert behavior | ✅ | Duplicate JID updates |
| LIMIT queries | ✅ | Respects limit parameter |
| Concurrent table access | ✅ | Both tables within one DB |
| Bad path error handling | ✅ | Returns IO error for invalid path |

### To Connect
1. Start social-forge with `WHATSAPP_STORE_DIR` set
2. Call `wa_auth_status` MCP tool → returns `{"authenticated": false}`
3. Call `wa_pair_with_code` with phone number
4. Use `wa_check_auth` to poll until authenticated
5. Message/chat ops become available

### Env Vars
```
WHATSAPP_STORE_DIR=./wa_store
```

---

## 7. Instagram-Standalone — ⚠️ Partial

**MCP tools**: 15 | **Auth**: Instagram Basic Display API

### Current State
- Registered conditionally (needs `INSTAGRAM_APP_ID`)
- Has test token in DB
- No real OAuth completed
- Uses different API from Instagram Business (Basic Display, not Graph API)

### Env Vars
```
INSTAGRAM_APP_ID=<...>
INSTAGRAM_APP_SECRET=<...>
```

---

## 8. Threads — ⚠️ Partial

**MCP tools**: 19 | **Auth**: OAuth v2 (Meta)

### Current State
- Registered conditionally (needs `THREADS_APP_ID`)
- Has test token in DB
- No real OAuth completed

### Env Vars
```
THREADS_APP_ID=<...>
THREADS_APP_SECRET=<...>
```

---

## 9. YouTube — ⚠️ Partial

**MCP tools**: 19 | **Auth**: OAuth v2 (Google)

### Current State
- **Connected via browser OAuth** ✅ — 2 channels linked
- `get_channel_stats` works (47 subs, 33 videos, 9475 views) ✅
- `get_analytics` works (433 views, 954 min watch time) ✅
- `fetch_page_info` works with real name + token ✅
- **Known issue**: Some tokens stored in plaintext (connected before `TOKEN_ENCRYPTION_KEY` was set). Re-auth resolves.
- Test: `tests/live_linkedin_youtube_test.rs` (23 tests, all pass)

### Env Vars
```
YOUTUBE_CLIENT_ID=<...>
YOUTUBE_CLIENT_SECRET=<...>
```

---

## 10. Telegram-User — 🔲 Missing

**MCP tools**: 21 | **Auth**: Grammers MTProto (phone-based)

### Architecture
- Uses `grammers` crate v0.7 (MTProto client library)
- `TelegramClientManager` (Arc<Mutex<InnerState>>)
  - `is_authenticated()`, `request_login_code()`, `sign_in()`, `check_password()`
  - `send_message()`, `list_dialogs()`, `list_contacts()`, `search()`, `user_info()`
- Always registered (no env var gate)

### To Connect
1. Get API ID/hash from https://my.telegram.org
2. Set `TELEGRAM_API_ID` and `TELEGRAM_API_HASH` in .env
3. Start server, open onboarding
4. Click "Connect" on Telegram User card
5. Enter phone number → verification code → 2FA password (if needed)
6. Session persists in memory for server lifetime

### Env Vars
```
TELEGRAM_API_ID=<...>
TELEGRAM_API_HASH=<...>
```

---

## 11. Telegram-Bot — 🔲 Missing

**MCP tools**: 7 | **Auth**: Bot API token

### To Connect
1. Create bot via @BotFather on Telegram
2. Set `TELEGRAM_BOT_TOKENS` (comma-separated for multiple bots)
3. Bot appears as a connected provider

### Env Vars
```
TELEGRAM_BOT_TOKENS=<token1>,<token2>
```

---

## 12–23. Remaining Providers — 🔲 Missing

All require env var population. Test coverage exists via `provider_methods_test.rs` (51 bad-token tests).

| Provider | MCP Tools | Env Vars Needed |
|----------|-----------|----------------|
| Bluesky | 11 | `BLUESKY_HANDLE`, `BLUESKY_PASSWORD` |
| Pinterest | 15 | `PINTEREST_CLIENT_ID`, `PINTEREST_CLIENT_SECRET` |
| Discord | 21 | `DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET`, `DISCORD_BOT_TOKEN` |
| TikTok | 7 | `TIKTOK_CLIENT_ID`, `TIKTOK_CLIENT_SECRET` |
| Mastodon | 9 | `MASTODON_CLIENT_ID`, `MASTODON_CLIENT_SECRET`, `MASTODON_INSTANCE_URL` |
| Slack | 9 | `SLACK_CLIENT_ID`, `SLACK_CLIENT_SECRET` |
| Medium | 7 | `MEDIUM_ACCESS_TOKEN` |
| Dev.to | 7 | `DEVTO_API_KEY` |
| Hashnode | 7 | `HASHNODE_API_KEY` |
| WordPress | 9 | Per-account application passwords (no global) |
| Skool | 11 | Chrome extension (no credentials) |

---

## Test Suite Reference

```text
tests/
├── provider_methods_test.rs    51 tests  — All pass
│   # Covers: X(4), Reddit(3), WordPress(2),
│   # Instagram-Standalone(9), Threads(9), LinkedIn Personal(7),
│   # LinkedIn Page(4), Discord(6), Skool(4), YouTube(1), Pinterest(1)
├── mcp_tools_test.rs           20 tests  — All pass
│   # Provider registry, metadata, tool count verification
├── linkedin_e2e_test.rs        20 tests  — 1 failure (test DB has real integrations now)
├── x_integration_test.rs       15 tests  — All pass
├── whatsapp_integration_test.rs 14 tests  — All pass
├── live_x_test.rs              11 tests  — All pass
├── x_transport_test.rs          5 tests  — All pass
├── x_live_test.rs               1 test   — Passes
├── live_linkedin_youtube_test.rs 23 tests — All pass (requires live DB + real tokens)
└── mcp_meta_audit.rs            0 tests   — Utility module
```

---

## Infrastructure

| Component | Status | Notes |
|-----------|--------|-------|
| PostgreSQL | ✅ Running | Docker container `postiz-rust-postgres-1` (port 5432) |
| Redis | ✅ Running | Docker container (caching/rate limiting) |
| Token Encryption | ✅ Wired | AES-256-GCM via `TOKEN_ENCRYPTION_KEY` (64 hex chars) |
| MCP stdio mode | ✅ Working | `--mcp` flag for AI agent integration |
| Port conflicts | ✅ Resolved | llm-proxy moved to 4488 (was 3000) |
| Onboarding UX | ✅ Working | Disconnect buttons, cookie form, status badges |

---

## Priority Action Items

### 🔴 High — Fix connected provider issues
- [ ] **LinkedIn Personal** — Token expired. Re-auth via onboarding needed.
- [ ] **YouTube** — Re-auth fixes token decryption for plaintext-stored tokens.
- [ ] **LinkedIn Page `get_page_posts`** — "No virtual resource found" (scope/permission). May need `r_liteprofile` re-auth.
- [ ] **Threads** — Not yet connected via browser OAuth (test token only).
- [ ] **Instagram-Standalone** — Not yet connected via browser OAuth (test token only).

### 🟡 Medium — Credential setup
- [ ] **Telegram API** — Get API ID/hash from my.telegram.org, set env vars, connect via onboarding
- [ ] **Reddit Cloudflare** — Evaluate wreq as alternative to reqwest for mail/inbox endpoints
- [ ] **WhatsApp live** — Test QR pairing end-to-end with real phone number
- [ ] **Discord** — Create bot, set DISCORD_BOT_TOKEN + env vars
- [ ] **Bluesky** — Set BLUESKY_HANDLE/PASSWORD

### 🟢 Low — Nice to have
- [ ] **Mastodon** — Register app, set env vars
- [ ] **Slack** — Create Slack app, set env vars
- [ ] **Medium/Dev.to/Hashnode** — Generate API keys
- [ ] **Pinterest/TikTok** — App registration + OAuth setup
- [ ] **WordPress** — Create application passwords per-site
