# Project Progress: postiz-rust

> **Last Updated**: 2026-05-11
> **Context**: Rust-based social media management platform with MCP (Model Context Protocol) server for AI agent integration, REST API for web UI, and 16 social providers.

---

## 1. Architecture Overview

```
postiz-rust/
├── src/
│   ├── main.rs                    # Single binary entrypoint
│   ├── config.rs                  # Config from env vars
│   ├── social/                    # Social providers (18 modules)
│   │   ├── mod.rs                 # SocialProvider trait definition
│   │   ├── registry.rs            # ProviderRegistry - 16 providers
│   │   ├── common.rs              # Shared HTTP client, helpers
│   │   ├── bluesky.rs             # AT Protocol provider
│   │   ├── discord.rs             # Discord bot provider
│   │   ├── facebook.rs            # Facebook Graph API provider
│   │   ├── instagram.rs           # Instagram Business (FB Graph API)
│   │   ├── instagram_standalone.rs# Instagram Basic Display API
│   │   ├── linkedin.rs            # LinkedIn personal OAuth2
│   │   ├── linkedin_page.rs       # LinkedIn org page OAuth2
│   │   ├── pinterest.rs           # Pinterest provider
│   │   ├── reddit.rs              # Reddit provider
│   │   ├── skool.rs               # Skool (chrome ext cookie)
│   │   ├── telegram_bot.rs        # Telegram bot provider
│   │   ├── telegram_user.rs       # Telegram user provider
│   │   ├── threads.rs             # Threads API provider
│   │   ├── whatsapp.rs            # WhatsApp daemon provider
│   │   ├── x.rs                   # X/Twitter provider
│   │   └── youtube.rs             # YouTube provider
│   ├── mcp/                       # MCP server tools
│   │   ├── mod.rs                 # Router: 104 tools registered
│   │   ├── tools_facebook.rs      # 16 tools
│   │   ├── tools_instagram.rs     # 17 tools
│   │   ├── tools_instagram_standalone.rs # 7 tools (NEW)
│   │   ├── tools_threads.rs       # 9 tools (NEW)
│   │   ├── tools_x.rs             # 18 tools
│   │   ├── tools_reddit.rs        # 7 tools
│   │   ├── tools_telegram_bot.rs  # 2 tools
│   │   ├── tools_telegram_user.rs # 5 tools
│   │   ├── tools_whatsapp.rs      # 4 tools
│   │   ├── tools_posts.rs         # 8 cross-cutting tools
│   │   ├── tools_integrations.rs  # 5 cross-cutting tools
│   │   └── tools_calendar.rs      # 1 cross-cutting tool
│   ├── api/                       # REST API handlers
│   ├── services/                  # Business logic
│   ├── db/                        # Database queries
│   └── models/                    # Data models
├── tests/
│   ├── mcp_tools_test.rs          # 14 MCP tool tests
│   └── provider_methods_test.rs   # 19 provider method tests (NEW)
├── migrations/                    # SQL migrations
└── PROJECT_PROGRESS.md            # This file
```

---

## 2. Provider Registry

**Registry**: `src/social/registry.rs` - `register_providers()`

| Provider | Provider ID | Registration | Auth Method | Scopes | Publish |
|---|---|---|---|---|---|
| X / Twitter | `x` | Always | OAuth 2.0 | tweet.read, tweet.write, users.read, offline.access | ✅ |
| Facebook | `facebook` | Always | OAuth 2.0 (multi-step) | pages_manage_posts, pages_read_engagement, pages_show_list, business_management | ✅ |
| Instagram Business | `instagram` | Always | OAuth 2.0 (multi-step) | instagram_basic, instagram_content_publish, pages_show_list | ✅ |
| LinkedIn | `linkedin` | Always | OAuth 2.0 PKCE | openid, profile, email, w_member_social | ✅ |
| LinkedIn Page | `linkedin-page` | Conditional (linkedin_client_id) | OAuth 2.0 PKCE (multi-step) | + rw_organization_admin, w_organization_social, r_organization_social | ✅ |
| Reddit | `reddit` | Always | Password Grant | read, submit, edit | ✅ |
| Bluesky | `bluesky` | Always | App Password | AT Protocol | ✅ |
| Pinterest | `pinterest` | Always | OAuth 2.0 | boards:read, pins:read, pins:write | ✅ |
| Telegram Bot | `telegram-bot` | Conditional (telegram_bot_tokens) | Bot Token | chat messages | ✅ |
| Telegram User | `telegram-user` | Always | Code-based | user messages | ✅ |
| WhatsApp | `whatsapp` | Always | Daemon | messages | ✅ |
| Skool | `skool` | Always | Chrome Ext Cookie | community access | ✅ |
| **Instagram Standalone** | `instagram-standalone` | Conditional (instagram_app_id) | OAuth 2.0 | instagram_business_basic, instagram_business_content_publish, instagram_business_manage_comments, instagram_business_manage_messages | ✅ |
| **Threads** | `threads` | Conditional (threads_client_id) | OAuth 2.0 | threads_basic, threads_content_publish | ✅ |
| YouTube | `youtube` | Conditional (youtube_client_id) | OAuth 2.0 (multi-step) | youtube.upload | ⚠️ Stub (returns "coming soon") |
| Discord | `discord` | Conditional (discord_client_id) | OAuth 2.0 | identify, guilds | ❌ |

**Total**: 16 providers (10 always-registered, 6 conditional on credentials)

---

## 3. MCP Tool Coverage

### 3.1 Provider-Specific MCP Tools (86 tools)

| Provider | Tools File | Tool Count | Tool Prefix | Details |
|---|---|---|---|---|
| X / Twitter | `tools_x.rs` | 18 | `x_` | Posts, search, bookmarks, DMs, lists, trends |
| Facebook | `tools_facebook.rs` | 16 | `fb_` | Pages, posts, albums, photos, videos, events, groups |
| Instagram Business | `tools_instagram.rs` | 17 | `ig_` | Media, comments, insights, hashtags, reels, stories, followers, business discovery |
| **Instagram Standalone** | `tools_instagram_standalone.rs` | **7** *(NEW)* | `ias_` | Media, comments, containers (create/publish/poll) |
| **Threads** | `tools_threads.rs` | **9** *(NEW)* | `th_` | Profile, threads, replies, create thread, delete, insights, poll publish |
| Reddit | `tools_reddit.rs` | 7 | `rd_` | Posts, search, comments, subreddits |
| Telegram Bot | `tools_telegram_bot.rs` | 2 | `tb_` | Messages, chats |
| Telegram User | `tools_telegram_user.rs` | 5 | `tu_` | Messages, contacts |
| WhatsApp | `tools_whatsapp.rs` | 4 | `wa_` | Messages, templates |
| **LinkedIn** | — | **0** | — | ❌ No MCP tools |
| **LinkedIn Page** | — | **0** | — | ❌ No MCP tools |
| **Bluesky** | — | **0** | — | ❌ No MCP tools |
| **YouTube** | — | **0** | — | ❌ No MCP tools |
| **Pinterest** | — | **0** | — | ❌ No MCP tools |
| **Skool** | — | **0** | — | ❌ No MCP tools |
| **Discord** | — | **0** | — | ❌ No MCP tools |

### 3.2 Cross-Cutting MCP Tools (18 tools)

| Module | Tool Count | Details |
|---|---|---|
| `tools_posts.rs` | 8 | Post CRUD, scheduling, publishing across providers |
| `tools_integrations.rs` | 5 | List, connect, disconnect, refresh tokens |
| `tools_calendar.rs` | 1 | Schedule management |
| Auth (inline in mod.rs) | 3 | Login, register, verify token |
| Shared (inline in mod.rs) | 1 | MCP resource/providers |

**Total MCP tools: 104** (86 provider-specific + 18 cross-cutting)

---

## 4. Instagram Standalone + Threads Implementation (Latest)

### 4.1 Instagram Standalone Provider
- **File**: `src/social/instagram_standalone.rs`
- **API**: `graph.instagram.com/v21.0` (Basic Display API)
- **Auth**: OAuth 2.0 via `instagram.com/oauth/authorize` → `api.instagram.com/oauth/access_token` → long-lived token upgrade
- **Credentials**: `INSTAGRAM_APP_ID` + `INSTAGRAM_APP_SECRET`
- **Provider Methods (7)**:
  - `get_media(access_token, ig_user_id, limit)` → List media with caption, media_type, media_url, permalink, timestamp
  - `get_media_detail(access_token, media_id)` → Single media detail
  - `get_media_comments(access_token, media_id)` → Comments on media
  - `reply_to_comment(access_token, comment_id, message)` → Reply to comment
  - `create_container(access_token, ig_user_id, media_url, caption, media_type)` → Create container for publishing
  - `publish_container(access_token, ig_user_id, creation_id)` → Publish a container
  - `poll_container_status(access_token, creation_id)` → Poll container publish status
- **MCP Tools (7)**:
  - `ias_get_media` / `ias_get_media_detail` / `ias_get_comments` / `ias_reply_to_comment`
  - `ias_create_container` / `ias_publish_container` / `ias_poll_container`

### 4.2 Threads Provider
- **File**: `src/social/threads.rs`
- **API**: `graph.threads.net/v1.0`
- **Auth**: OAuth 2.0 via `threads.net/oauth/authorize` → `graph.threads.net/oauth/access_token` → long-lived token upgrade
- **Credentials**: `THREADS_CLIENT_ID` + `THREADS_CLIENT_SECRET`
- **Provider Methods (7)**:
  - `get_profile(access_token)` → Threads profile info
  - `get_threads(access_token, user_id, limit)` → List threads
  - `get_thread_detail(access_token, media_id)` → Single thread detail
  - `get_thread_replies(access_token, media_id)` → Replies on thread
  - `reply_to_thread(access_token, media_id, message)` → Reply to thread
  - `get_insights(access_token, user_id, metric, period)` → Thread insights
  - `delete_thread(access_token, media_id)` → Delete thread
- **MCP Tools (9)**:
  - `th_get_profile` / `th_get_threads` / `th_get_thread_detail` / `th_get_replies` / `th_reply_to_thread`
  - `th_create_thread` / `th_delete_thread` / `th_get_insights` / `th_poll_publish_status`

### 4.3 Test Results
- **33/33 tests pass** (14 mcp_tools_test + 19 provider_methods_test)
- All provider methods tested against real Meta APIs with invalid tokens → proper ProviderError returned
- Build: clean (1 pre-existing warning)
- Token lookup: both providers search `integrations` by `provider_identifier` + `internal_id`

---

## 5. LinkedIn Integration Analysis

### 5.1 What We Have (OAuth 2.0)

| Aspect | LinkedIn Personal | LinkedIn Page |
|---|---|---|
| **File** | `src/social/linkedin.rs` (326 lines) | `src/social/linkedin_page.rs` (326 lines) |
| **Auth** | OAuth 2.0 PKCE | OAuth 2.0 PKCE (multi-step) |
| **Scopes** | openid, profile, email, w_member_social | + rw_organization_admin, w_organization_social, r_organization_social |
| **API Base** | `api.linkedin.com/v2` | `api.linkedin.com/v2` |
| **Capabilities** | `publish()` (UGC Posts), `exchange_code()` | `publish()` (UGC Posts), `pages()` (list orgs), `fetch_page_info()`, `reconnect()` |
| **MCP Tools** | ❌ **None** | ❌ **None** |
| **Config** | `LINKEDIN_CLIENT_ID` + `LINKEDIN_CLIENT_SECRET` | Same + LINKEDIN_CLIENT_ID |
| **Registration** | Always registered | Conditional (linkedin_client_id exists) |

**Official LinkedIn API v2 limits** (what's possible with our OAuth2 tokens):
- ✅ **Post publishing** (UGC Posts) — both personal and org — already implemented
- ✅ **Profile reading** — `/v2/userinfo` (OpenID Connect, limited fields: sub, name, given_name, family_name, picture)
- ✅ **Company pages** — `/v2/organizationalEntityAcls?q=roleAssignee&role=ADMINISTRATOR` — already implemented
- ⚠️ **Post reading** — `/v2/ugcPosts?q=authors&author=urn:li:person:{id}` — possible but need author URN from profile
- ⚠️ **Comment reading/writing** — `/v2/socialActions/{urn}/comments` — possible but needs URN resolution
- ❌ **Messaging** — Requires `rw_messages` scope + additional approval from LinkedIn
- ❌ **People search** — Requires `r_liteprofile` (deprecated v1 API) or Partnership program
- ❌ **Job search** — Requires `r_jobs` scope (limited Partnership program)
- ❌ **Connections** — Not available in v2 API

### 5.2 Reference: linkedin-mcp-server (Voyager API)

**Repository**: `/home/ishanp/Documents/GitHub/CLONED-REPOS/linkedin-mcp-server`
**Language**: Python
**Auth model**: Session cookies (`li_at`, `JSESSIONID`) from browser
**API protocol**: LinkedIn Voyager API (internal REST + GraphQL at `www.linkedin.com`)

| Module | Tools | What It Does | Feasibility in postiz-rust |
|---|---|---|---|
| `person.py` | `get_person_profile` | Full profile: experience, education, skills, about, posts, contact info | Low — requires Voyager API + cookie auth |
| `person.py` | `search_people` | Search LinkedIn people directory | Low — requires Voyager API |
| `person.py` | `get_contact_info` | Email, phone from profile | Medium — via Voyager profile endpoint |
| `person.py` | `get_sidebar_profiles` | People also viewed | Low — Voyager-specific |
| `person.py` | `connect_with_person` | Send connection request | Low — Voyager-specific |
| `company.py` | `get_company` | Company about + open jobs | Low — Voyager-specific data |
| `company.py` | `get_company_posts` | Recent company posts | Medium — official API has feed endpoint |
| `job.py` | `get_job` | Job details | Low — Voyager-specific |
| `job.py` | `search_jobs` | Job search | Low — Voyager-specific |
| `messaging.py` | `get_inbox` | Message inbox | Low — requires Voyager |
| `messaging.py` | `get_conversation` | Conversation history | Low — requires Voyager |
| `messaging.py` | `search_conversations` | Search messages | Low — requires Voyager |
| `messaging.py` | `send_message` | Send DM | Low — requires Voyager |
| `search.py` | `search_linkedin` | Wraps search_people | Low — Voyager-specific |

**Key dependencies**: `curl-cffi>=0.15.0` (Chrome 146 TLS fingerprint impersonation), `fastmcp`, `structlog`
**Cookie refresh**: Lightpanda headless browser, auto-import from Firefox/Chrome/Brave

### 5.3 What We Can Build (Postiz-Rust, OAuth 2.0)

**Viability assessment**: The Voyager API (internal LinkedIn.com API) and the official LinkedIn API v2 are fundamentally incompatible — different auth models (cookies vs OAuth2), different endpoints, different rate limits. We cannot integrate Voyager into postiz-rust without adding a completely separate auth system.

**What's feasible with OAuth 2.0 tokens**:

| Tool | Priority | Effort | Notes |
|---|---|---|---|
| `li_get_profile` | High | 1 day | `/v2/userinfo` already works; add `/v2/me` for full profile (urn, headline, etc.) |
| `li_get_posts` | High | 1 day | `/v2/ugcPosts?q=authors&author=urn:li:person:{urn}` after resolving profile URN |
| `li_create_post` | High | 0.5 day | Already have `publish()` — just wrap in MCP tool |
| `li_list_pages` | Medium | 0.5 day | Already have `pages()` — wrap in MCP tool |
| `li_get_page_posts` | Medium | 1 day | Query UGC Posts for org author |
| `li_get_page_info` | Medium | 0.5 day | Already have `fetch_page_info()` |
| `li_create_page_post` | Medium | 0.5 day | Already have page `publish()` |
| `li_get_comments` | Low | 1-2 days | `/v2/socialActions/{shareUrn}/comments` — needs URN resolution |
| `li_search_people` | ❌ | N/A | Requires Partnership or deprecated API |
| `li_send_message` | ❌ | N/A | Requires `rw_messages` scope approval |

**Recommendation**: Build 3 high-priority LinkedIn MCP tools (get_profile, get_posts, create_post) first, then 3 medium-priority tools (list_pages, get_page_posts, create_page_post). Skip Voyager-only features.

---

## 6. Provider Auth & Token Details

| Provider | Auth Type | Refresh | Cron Refresh | Token Encryption | DB Columns |
|---|---|---|---|---|---|
| X | OAuth2 | ✅ | ❌ | ✅ | user_id, provider_identifier, access_token, refresh_token, token_expires_at, internal_id, profile_name, profile_picture, disabled |
| Facebook | OAuth2 (multi-step) | ✅ | ❌ | ✅ | Same + profile_url |
| Instagram | OAuth2 (multi-step) | ✅ | ❌ | ✅ | Same |
| LinkedIn | OAuth2 PKCE | ✅ | ❌ | ✅ | Same |
| LinkedIn Page | OAuth2 PKCE (multi-step) | ✅ | ❌ | ✅ | Same |
| Reddit | Password Grant | ❌ | ❌ | ✅ | access_token only |
| Bluesky | App Password | ❌ | ❌ | ✅ | Non-expiring |
| Pinterest | OAuth2 | ✅ | ❌ | ✅ | Same |
| IG Standalone | OAuth2 | ✅ (long-lived upgrade) | ✅ | ✅ | Same, internal_id = IG user ID |
| Threads | OAuth2 | ✅ (long-lived upgrade) | ✅ | ✅ | Same, internal_id = Threads user ID |
| YouTube | OAuth2 (multi-step) | ✅ | ❌ | ✅ | Same |
| Discord | OAuth2 | ✅ | ❌ | ✅ | Same |
| Telegram Bot | Bot Token | ❌ | ❌ | ✅ | bot token from env |
| Telegram User | Code-based | ❌ | ❌ | ✅ | session |
| WhatsApp | Daemon | ❌ | ❌ | ✅ | daemon-managed |
| Skool | Chrome Ext Cookie | ❌ | ❌ | ✅ | cookie |

**Integrations table schema**: `id (UUID PK)`, `user_id (UUID FK → users)`, `provider_identifier (TEXT)`, `provider_name (TEXT)`, `internal_id (TEXT)`, `access_token (TEXT)`, `refresh_token (TEXT)`, `token_expires_at (TIMESTAMPTZ)`, `profile_name (TEXT)`, `profile_picture (TEXT)`, `profile_url (TEXT)`, `disabled (BOOLEAN)`, `refresh_needed (BOOLEAN)`, `posting_times (JSONB)`, `created_at`, `updated_at`
**Unique**: `(user_id, provider_identifier, internal_id)`

---

## 7. Missing MCP Tools — Priority Queue

### Tier 1: High Priority (Active Users)

| Provider | Tools to Build | Effort | Reason |
|---|---|---|---|
| **LinkedIn** | `li_get_profile`, `li_get_posts`, `li_create_post`, `li_list_pages` | 2-3 days | Largest gap; widely used for publishing |
| **Bluesky** | `bs_get_profile`, `bs_get_timeline`, `bs_create_post`, `bs_search`, `bs_get_thread` | 2-3 days | Growing platform, full AT Protocol available |
| **YouTube** | `yt_get_channel`, `yt_get_videos`, `yt_upload_video`, `yt_get_analytics` | 2-3 days | Conditional provider, stub publish |

### Tier 2: Medium Priority

| Provider | Tools to Build | Effort | Reason |
|---|---|---|---|
| **Pinterest** | `pin_get_boards`, `pin_get_pins`, `pin_create_pin`, `pin_search` | 1-2 days | Niche, some active users |
| **LinkedIn Page** | `li_page_get_info`, `li_page_get_posts`, `li_page_create_post` | 1 day | Companion to LinkedIn personal |
| **Skool** | `sk_get_groups`, `sk_get_posts` | 1 day | Community management |
| **Discord** | `dc_get_guilds`, `dc_send_message` | 1 day | Conditional provider |

### Tier 3: Low Priority

| Provider | Tools | Notes |
|---|---|---|
| **Instagram Standalone** | ✅ Done | 7 tools |
| **Threads** | ✅ Done | 9 tools |
| X / Twitter | 18 tools | Coverage good |
| Facebook | 16 tools | Coverage good |
| Instagram Business | 17 tools | Coverage good |
| Reddit | 7 tools | Coverage adequate |
| Telegram Bot | 2 tools | Coverage adequate |
| Telegram User | 5 tools | Coverage adequate |
| WhatsApp | 4 tools | Coverage adequate |

---

## 8. CLI Coverage

**Current state**: No `src/cli/` module exists. The single binary `postiz-rust` has two modes:
1. **Web server** (default) — REST API at port 3001
2. **MCP stdio** (`--mcp` flag) — MCP tools over stdio for AI agent integration

**No provider-specific CLI interface**. The `gogcli/` and `tg/` directories are separate Go/C subprojects for Telegram — unrelated to the main tool.

**Recommendation**: MCP serves as the programmable CLI. For human CLI use, tools can be added to a `src/cli.rs` or commands module, but the `--mcp` approach is the intended interface.

---

## 9. Test Coverage

| Test File | Tests | Scope |
|---|---|---|
| `tests/mcp_tools_test.rs` | 14 | MCP tool module registration, provider creation, scope verification, router registration for all 74+ tools |
| `tests/provider_methods_test.rs` | 19 | All 14 provider methods (7 IG standalone + 7 Threads) hit real Meta APIs with invalid tokens — verify error handling. Auth URL generation. MCP handler chain verification. Threads publish method test. |

**Total**: 33 tests, all passing
**Build**: Clean (1 pre-existing warning: unused `publish_tool` field in `telegram_daemon.rs`)

---

## 10. Environment Configuration

| Env Var | Used By | Required | Notes |
|---|---|---|---|
| `DATABASE_URL` | All | ✅ | PostgreSQL connection |
| `JWT_SECRET` | Auth | ✅ | Min 32 chars |
| `APP_URL` | OAuth callbacks | ✅ | e.g., http://localhost:3000 |
| `INSTAGRAM_APP_ID` | IG Standalone | Conditional | Basic Display API app |
| `INSTAGRAM_APP_SECRET` | IG Standalone | Conditional | Basic Display API app secret |
| `THREADS_CLIENT_ID` | Threads | Conditional | Threads API app |
| `THREADS_CLIENT_SECRET` | Threads | Conditional | Threads API app secret |
| `INSTAGRAM_CLIENT_ID` | Instagram Business | ✅ | Facebook Graph API app |
| `INSTAGRAM_CLIENT_SECRET` | Instagram Business | ✅ | Facebook Graph API app secret |
| `LINKEDIN_CLIENT_ID` | LinkedIn | Conditional | LinkedIn OAuth 2.0 app |
| `LINKEDIN_CLIENT_SECRET` | LinkedIn | Conditional | LinkedIn OAuth 2.0 app secret |
| `TOKEN_ENCRYPTION_KEY` | Token Storage | Optional | 32 bytes, encrypts tokens at rest |

---

## 11. Next Steps

### Immediate (1-2 weeks)
1. **LinkedIn MCP tools**: `li_get_profile`, `li_get_posts`, `li_create_post` (3 high-priority tools)
2. **Bluesky MCP tools**: Profile, timeline, create post, search, thread (5 tools)

### Short-term (2-4 weeks)
3. **YouTube MCP tools**: Channel info, video listing, upload, analytics (4 tools)
4. **Pinterest MCP tools**: Boards, pins, create pin, search (4 tools)
5. **LinkedIn Page MCP tools**: Page info, page posts, create page post (3 tools)

### Medium-term (1-2 months)
6. Implement Voyager-style LinkedIn integration (separate sub-project, cookie auth)
7. Provider-specific CLI commands via MCP client
8. End-to-end integration tests with real connected accounts

### Infrastructure
9. SQLx offline cache for CI builds
10. API documentation (REST + MCP)
11. Integration test harness with Docker
