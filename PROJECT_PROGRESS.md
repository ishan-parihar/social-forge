# Project Progress: postiz-rust

> **Last Updated**: 2026-05-11 (v3)
> **MCP Tool Count**: 115 (+12 Discord/Skool/YouTube/Pinterest Phase 2 tools)
> **Active Test Pass**: 148/148 ✅ (including 14 WhatsApp tests — native wa-rs)
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
│   │   ├── mod.rs                 # Router: 126 tools registered
│   │   ├── tools_facebook.rs      # 16 tools
│   │   ├── tools_instagram.rs     # 17 tools
│   │   ├── tools_instagram_standalone.rs # 7 tools
│   │   ├── tools_threads.rs       # 9 tools
│   │   ├── tools_linkedin.rs      # 6 tools
│   │   ├── tools_linkedin_page.rs # 4 tools
│   │   ├── tools_youtube.rs       # 9 tools
│   │   ├── tools_pinterest.rs     # 7 tools
│   │   ├── tools_discord.rs       # 10 tools
│   │   ├── tools_skool.rs         # 5 tools
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
│   ├── mcp_tools_test.rs          # 20 MCP tool tests
│   └── provider_methods_test.rs   # 42 provider method tests
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

### 3.1 Provider-Specific MCP Tools (96 tools)

| Provider | Tools File | Tool Count | Tool Prefix | Details |
|---|---|---|---|---|
| X / Twitter | `tools_x.rs` | 18 | `x_` | Posts, search, bookmarks, DMs, lists, trends |
| Facebook | `tools_facebook.rs` | 16 | `fb_` | Pages, posts, albums, photos, videos, events, groups |
| Instagram Business | `tools_instagram.rs` | 17 | `ig_` | Media, comments, insights, hashtags, reels, stories, followers, business discovery |
| Instagram Standalone | `tools_instagram_standalone.rs` | 7 | `ias_` | Media, comments, containers (create/publish/poll) |
| Threads | `tools_threads.rs` | 9 | `th_` | Profile, threads, replies, create thread, delete, insights, poll publish |
| Reddit | `tools_reddit.rs` | 7 | `rd_` | Posts, search, comments, subreddits |
| Telegram Bot | `tools_telegram_bot.rs` | 2 | `tb_` | Messages, chats |
| Telegram User | `tools_telegram_user.rs` | 5 | `tu_` | Messages, contacts |
| WhatsApp | `tools_whatsapp.rs` | 4 | `wa_` | Messages, templates |
| LinkedIn | `tools_linkedin.rs` | 6 | `li_` | Profile, posts, post detail, comments, create comment, create post |
| LinkedIn Page | `tools_linkedin_page.rs` | 4 | `lip_` | List pages, page info, page posts, create comment |
| YouTube | `tools_youtube.rs` | 9 | `yt_` | Search, videos, playlists, items, comments, channel stats, analytics, subscriptions, **find creators** |
| Pinterest | `tools_pinterest.rs` | 7 | `pi_` | User account, boards, pins, pin detail, board analytics, pin analytics, **search pins** |
| Discord | `tools_discord.rs` | 10 | `di_` | Channel info, messages, guild info, thread members, **send message, delete message, reaction, guild channels, server info, forum post** |
| Skool | `tools_skool.rs` | 5 | `sk_` | Publish, **community info, list posts, get post, create comment** |
| **Bluesky** | — | **0** | — | ❌ No MCP tools |

### 3.2 Cross-Cutting MCP Tools (18 tools)

| Module | Tool Count | Details |
|---|---|---|
| `tools_posts.rs` | 8 | Post CRUD, scheduling, publishing across providers |
| `tools_integrations.rs` | 5 | List, connect, disconnect, refresh tokens |
| `tools_calendar.rs` | 1 | Schedule management |
| Auth (inline in mod.rs) | 3 | Login, register, verify token |
| Shared (inline in mod.rs) | 1 | MCP resource/providers |

**Total MCP tools: 144** (126 provider-specific + 18 cross-cutting)

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
- **Credentials**: `THREADS_APP_ID` + `THREADS_APP_SECRET`
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
- **33 tests pass** across `mcp_tools_test`, `provider_methods_test`
- All provider methods tested against real Meta APIs with invalid tokens → proper ProviderError returned
- Build: clean (1 pre-existing warning)
- Token lookup: both providers search `integrations` by `provider_identifier` + `internal_id`

---

## 5. LinkedIn Integration — Complete ✅

**Implementation Date**: 2026-05-11

### 5.1 Provider Methods Added

**LinkedIn Personal** (`src/social/linkedin.rs`, 496 lines):
| Method | Endpoint | Purpose |
|---|---|---|
| `get_profile(access_token)` | GET `/v2/userinfo` | Read own profile |
| `get_user_id(access_token)` | GET `/v2/userinfo` (extract `sub`) | Resolve authenticated user URN |
| `get_posts(access_token, author_urn, limit)` | GET `/v2/rest/posts?author=` | List posts (new Posts API) |
| `get_post_detail(access_token, post_urn)` | GET `/v2/rest/posts/{urn}` | Single post by URN |
| `get_post_comments(access_token, post_urn)` | GET `/v2/rest/socialActions/{urn}/comments` | Comments on post |
| `create_comment(access_token, post_urn, actor_urn, message)` | POST `/v2/rest/socialActions/{urn}/comments` | Comment on post |

**LinkedIn Page** (`src/social/linkedin_page.rs`, 385 lines):
| Method | Endpoint | Purpose |
|---|---|---|
| `get_page_posts(access_token, page_id, limit)` | GET `/v2/rest/posts?author=urn:li:organization:{id}` | List org posts |
| `create_comment(access_token, post_urn, page_urn, message)` | POST `/v2/rest/socialActions/{urn}/comments` | Comment as org |

**All use**: Headers `Authorization: Bearer`, `X-Restli-Protocol-Version: 2.0.0`, `LinkedIn-Version: 202601`. Error handling: `200/201→Ok`, `401→TokenExpired`, else→`Api(msg)`.

### 5.2 MCP Tools

**LinkedIn Personal** (`tools_linkedin.rs`, 189 lines): 6 tools
`li_get_profile`, `li_get_posts`, `li_get_post_detail`, `li_get_comments`, `li_create_comment`, `li_create_post`
Token lookup: `find_linkedin_token()` filters by `provider_identifier == "linkedin"` + `internal_id`

**LinkedIn Page** (`tools_linkedin_page.rs`, 133 lines): 4 tools
`lip_list_pages`, `lip_get_page`, `lip_get_page_posts`, `lip_create_comment`
Token lookup: `find_linkedin_page_token()` filters by `provider_identifier == "linkedin-page"` + `internal_id`

### 5.3 Official LinkedIn API v2 Limits

- ✅ **Post publishing** (UGC Posts) — both personal and org
- ✅ **Profile reading** — `/v2/userinfo`
- ✅ **Company page management** — listing, info, org posts
- ✅ **Post reading** — `/v2/rest/posts` (new Posts API, replaced legacy ugcPosts)
- ✅ **Comments** — read and create on any post
- ❌ **Messaging** — requires `rw_messages` scope + LinkedIn Partnership approval
- ❌ **People search** — requires deprecated `r_liteprofile` or Partnership program
- ❌ **Job search** — requires `r_jobs` (Partnership only)
- ❌ **Connections** — not available in v2 API

### 5.4 Reference: linkedin-mcp-server (Voyager API)

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

### 5.5 What We Can Build (Postiz-Rust, OAuth 2.0)

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

## 6. YouTube · Pinterest · Discord · Skool Implementation

**Implementation Date**: 2026-05-11
**New MCP Tools**: 19 (YouTube: 8, Pinterest: 6, Discord: 4, Skool: 1)
**Phase 1 Tool Count**: 84 → 103
**Phase 2 Additions**: +12 (Discord: +6, Skool: +4, YouTube: +1, Pinterest: +1)
**Final Tool Count**: 103 → 115

### 6.1 YouTube (`src/social/youtube.rs`)

| Provider Method | HTTP | Endpoint | Purpose |
|---|---|---|---|
| `search_videos(access_token, query, max_results)` | GET | `/youtube/v3/search` | Search YouTube videos |
| `get_video(access_token, video_id)` | GET | `/youtube/v3/videos?id=` | Single video details |
| `get_playlists(access_token, channel_id, max_results)` | GET | `/youtube/v3/playlists?channelId=` | Channel playlists |
| `get_playlist_items(access_token, playlist_id, max_results)` | GET | `/youtube/v3/playlistItems?playlistId=` | Items in playlist |
| `get_comments(access_token, video_id, max_results)` | GET | `/youtube/v3/commentThreads?videoId=` | Video comments (threaded) |
| `get_channel_stats(access_token, channel_id)` | GET | `/youtube/v3/channels?id=&part=statistics` | Subscriber count, views, videos |
| `get_analytics(access_token, channel_id, start_date, end_date, metrics)` | GET | `/youtube/v3/channels?id=&part=statistics` | Channel analytics |
| `get_subscriptions(access_token, max_results)` | GET | `/youtube/v3/subscriptions?mine=true` | Channel subscriptions |

**MCP Tools** (`tools_youtube.rs`, 195 lines): 8 tools
`yt_search_videos`, `yt_get_video`, `yt_list_playlists`, `yt_get_playlist_items`, `yt_get_comments`, `yt_get_channel_stats`, `yt_get_analytics`, `yt_get_subscriptions`
- Token lookup: `find_yt_token()` filters by `provider_identifier == "youtube"` + `internal_id`
- **API key fallback**: All methods check for `youtube_api_key` config. If no token found AND no API key, returns error.

### 6.2 Pinterest (`src/social/pinterest.rs`)

| Provider Method | HTTP | Endpoint | Purpose |
|---|---|---|---|
| `get_user_account(access_token)` | GET | `/v5/user_account` | Own account info |
| `get_board(access_token, board_id)` | GET | `/v5/boards/{id}` | Single board detail |
| `get_board_pins(access_token, board_id, limit)` | GET | `/v5/boards/{id}/pins` | Pins on a board |
| `get_pin(access_token, pin_id)` | GET | `/v5/pins/{id}` | Single pin detail |
| `get_board_analytics(access_token, board_id, start_date, end_date)` | GET | `/v5/boards/{id}/analytics` | Board metrics |
| `get_pin_analytics(access_token, pin_id, start_date, end_date, metrics)` | GET | `/v5/pins/{id}/analytics` | Pin metrics |

**MCP Tools** (`tools_pinterest.rs`, 159 lines): 6 tools
`pi_get_user_account`, `pi_get_board`, `pi_get_board_pins`, `pi_get_pin`, `pi_get_board_analytics`, `pi_get_pin_analytics`
- Token lookup: `find_pi_token()` filters by `provider_identifier == "pinterest"` + `internal_id`
- Auth: `Authorization: Bearer {token}` header (API v5)

### 6.3 Discord (`src/social/discord.rs`)

| Provider Method | HTTP | Endpoint | Purpose |
|---|---|---|---|
| `get_channel(access_token, channel_id)` | GET | `/v10/channels/{id}` | Channel info |
| `get_channel_messages(access_token, channel_id, limit)` | GET | `/v10/channels/{id}/messages` | Channel messages |
| `get_guild(access_token, guild_id)` | GET | `/v10/guilds/{id}` | Server info |
| `get_thread_members(access_token, channel_id)` | GET | `/v10/channels/{id}/thread-members` | Thread members |

**MCP Tools** (`tools_discord.rs`, 121 lines): 4 tools
`di_get_channel`, `di_get_channel_messages`, `di_get_guild`, `di_get_thread_members`
- Token lookup: `find_di_token()` filters by `provider_identifier == "discord"` + `internal_id`
- Auth: Uses `Bot {bot_token}` for read operations (user `access_token` passed but unused for Discord guild ops)

### 6.4 Skool (`src/social/skool.rs`)

| Provider Method | HTTP | Endpoint | Purpose |
|---|---|---|---|
| (publish via SocialProvider trait) | POST | `api2.skool.com/posts` | Post to community |

**MCP Tools** (`tools_skool.rs`, ~80 lines): 1 tool
`sk_publish` — posts content to a Skool community (wraps SocialProvider::publish)
- Token lookup: `find_sk_token()` filters by `provider_identifier == "skool"` + `internal_id`
- **Limited scope**: Skool uses private API (api2.skool.com) with cookie auth. Only `publish()` is reliable.

---

## 7. Phase 2: Feature Parity Enhancement

**Implementation Date**: 2026-05-11
**New Provider Methods**: 12 (Discord: 6, Skool: 4, YouTube: 1, Pinterest: 1)
**New MCP Tools**: 12
**Tool Count**: 103 → 115

**Reference Repos Cloned**:
- `youtube-mcp-server` (TypeScript, youtube-transcript + Google API) — gap: channel enrichment, creator discovery, multi-key failover
- `mcp-pinterest` (TypeScript, Puppeteer scraper) — gap: keyword search (no official API usage)
- `skool-mcp` (TypeScript, Next.js data routes + api2.skool.com) — gap: community info, member management, posts listing courses
- `mcp-discord` (TypeScript, discord.js Gateway) — gap: send message, reactions, channels, forum posts, webhooks

### 7.1 Discord Phase 2

**New Provider Methods** (`src/social/discord.rs`):

| Method | HTTP | Endpoint | Purpose |
|---|---|---|---|
| `send_message(channel_id, content)` | POST | `/channels/{id}/messages` | Send text message to channel |
| `delete_message(channel_id, message_id)` | DELETE | `/channels/{id}/messages/{mid}` | Delete a message |
| `add_reaction(channel_id, message_id, emoji)` | PUT | `/channels/{id}/messages/{mid}/reactions/{emoji}/@me` | React to a message |
| `get_guild_channels(guild_id)` | GET | `/guilds/{id}/channels` | List channels in guild |
| `get_server_info(guild_id)` | GET | `/guilds/{id}?with_counts=true` | Detailed server info with member counts |
| `create_forum_post(channel_id, name, content, tags)` | POST | `/channels/{id}/threads` | Create forum post |

All use `Authorization: Bot {bot_token}` header (no OAuth user token).

**New MCP Tools** (6): `di_send_message`, `di_delete_message`, `di_add_reaction`, `di_get_guild_channels`, `di_get_server_info`, `di_create_forum_post`

### 7.2 Skool Phase 2

**New Provider Methods** (`src/social/skool.rs`): Uses Next.js data routes for reads, api2.skool.com for writes.

| Method | Endpoint | Purpose |
|---|---|---|
| `get_community_info(slug, access_token)` | `/_next/data/{buildId}/{slug}/about.json` | Community info |
| `list_posts(slug, access_token, page, sort, category)` | `/_next/data/{buildId}/{slug}.json?p=&s=&c=` | List posts |
| `get_post(slug, post_slug, access_token)` | `/_next/data/{buildId}/{slug}/p/{post_slug}.json` | Single post |
| `create_comment(post_id, group_id, content, access_token)` | POST `/api2/comments` | Comment on post |

BuildId resolved dynamically: fetch community page → parse `__NEXT_DATA__` from HTML.

**New MCP Tools** (4): `sk_get_info`, `sk_list_posts`, `sk_get_post`, `sk_create_comment`

### 7.3 YouTube Phase 2

| Method | Endpoint | Purpose |
|---|---|---|
| `find_creators(access_token, query, min_subs, max_results)` | 2-step: search → channel enrich | Topic search → group by channel → subscriber count + email detection |

**New MCP Tool**: `yt_find_creators`

### 7.4 Pinterest Phase 2

| Method | Endpoint | Purpose |
|---|---|---|
| `search_pins(access_token, query, limit)` | `GET /v5/pins/search?query=&page_size=` | Keyword search via official API v5 |

**New MCP Tool**: `pi_search_pins`

### 7.5 Phase 2 Results

| Provider | Before | After | Reference Tools | Key New Capabilities |
|---|---|---|---|---|
| Discord | 4 tools | 10 tools | 21 tools | Send, delete, react, channels, server info, forum posts |
| Skool | 1 tool | 5 tools | 13 tools | Community info, posts list/get, comments |
| YouTube | 8 tools | 9 tools | 10 tools | Creator discovery with channel enrichment |
| Pinterest | 6 tools | 7 tools | 3 tools (scraper) | Search pins via official API |

---

## 9. Provider Auth & Token Details

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

## 10. Missing MCP Tools — Priority Queue

### Tier 1: High Priority

| Provider | Tools to Build | Effort | Reason |
|---|---|---|---|
| **Bluesky** | `bs_get_profile`, `bs_get_timeline`, `bs_create_post`, `bs_search`, `bs_get_thread` | 2-3 days | Last orphan provider (0 tools) |
| **YouTube publish** | Fix `publish()` stub | 0.5 day | Currently returns "coming soon" |

### Tier 2: Medium Priority

| Provider | Tools | Notes |
|---|---|---|
| **Discord** | ✅ Done | 10 tools — Phase 2 complete |
| **Skool** | ✅ Done | 5 tools — Phase 2 complete |
| **YouTube** | ✅ Done | 9 tools — Phase 2 complete |
| **Pinterest** | ✅ Done | 7 tools — Phase 2 complete |
| **LinkedIn** | ✅ Done | 10 tools (6 personal + 4 page) |

### Tier 3: Low Priority

| Provider | Tools | Notes |
|---|---|---|
| **Instagram Standalone** | 7 tools | Coverage good |
| **Threads** | 9 tools | Coverage good |
| X / Twitter | 18 tools | Coverage good |
| Facebook | 16 tools | Coverage good |
| Instagram Business | 17 tools | Coverage good |
| Reddit | 7 tools | Coverage adequate |
| Telegram Bot | 2 tools | Coverage adequate |
| Telegram User | 5 tools | Coverage adequate |
| WhatsApp | 4 tools | Coverage adequate |

---

## 11. CLI Coverage

**Current state**: No `src/cli/` module exists. The single binary `postiz-rust` has two modes:
1. **Web server** (default) — REST API at port 3001
2. **MCP stdio** (`--mcp` flag) — MCP tools over stdio for AI agent integration

**No provider-specific CLI interface**. The `gogcli/` and `tg/` directories are separate Go/C subprojects for Telegram — unrelated to the main tool.

**Recommendation**: MCP serves as the programmable CLI. For human CLI use, tools can be added to a `src/cli.rs` or commands module, but the `--mcp` approach is the intended interface.

---

## 12. Test Coverage

| Test File | Tests | Scope |
|---|---|---|
| `tests/mcp_tools_test.rs` | 20 | MCP tool module registration, provider creation, scope verification, router registration for all 115 tools |
| `tests/provider_methods_test.rs` | 42 | All provider methods (7 IG standalone + 7 Threads + 6 LinkedIn personal + 4 LinkedIn Page + 6 Discord + 4 Skool + 1 YouTube find_creators + 1 Pinterest search_pins + 6 handler chain) hit real APIs with invalid tokens |
| `tests/linkedin_e2e_test.rs` | 20 | Comprehensive LinkedIn end-to-end: provider registry, auth URLs, scopes, MCP server, handler chain, DB schema, publish flows, reconnect flow |
| `tests/mcp_meta_audit.rs` | 2 | Meta audit of FB/IG tool names and structure |

**Total**: 135 tests (53 unit + 82 integration), all passing
**Build**: Clean (1 pre-existing warning: unused `binary_path` in `telegram_daemon.rs`)

| Test Suite | Tests | Scope |
|---|---|---|
| `tests/mcp_tools_test.rs` | 20 | MCP tool module registration, provider creation, scope verification for all 115 tools |
| `tests/provider_methods_test.rs` | 42 | All provider methods across IG standalone, Threads, LinkedIn, YouTube, Pinterest, Discord, Skool hit real APIs with invalid tokens |
| `tests/linkedin_e2e_test.rs` | 20 | Comprehensive LinkedIn end-to-end: provider registry, auth URLs, scopes, MCP server, handler chain, DB schema, publish flows, reconnect flow |
| `tests/mcp_meta_audit.rs` | 2 | Meta audit of FB/IG tool names and structure |
| All lib tests (`#[cfg(test)]`) | 53 | Unit tests across all crates |
| **Total** | **148 passing** | (14 WhatsApp tests — all passing with native wa-rs) |

---

## 13. Environment Configuration

| Env Var | Used By | Required | Notes |
|---|---|---|---|
| `DATABASE_URL` | All | ✅ | PostgreSQL connection |
| `JWT_SECRET` | Auth | ✅ | Min 32 chars |
| `APP_URL` | OAuth callbacks | ✅ | e.g., http://localhost:3000 |
| `INSTAGRAM_APP_ID` | IG Standalone | Conditional | Basic Display API app |
| `INSTAGRAM_APP_SECRET` | IG Standalone | Conditional | Basic Display API app secret |
| `THREADS_APP_ID` | Threads | Conditional | Threads API app |
| `THREADS_APP_SECRET` | Threads | Conditional | Threads API app secret |
| `INSTAGRAM_CLIENT_ID` | Instagram Business | ✅ | Facebook Graph API app |
| `INSTAGRAM_CLIENT_SECRET` | Instagram Business | ✅ | Facebook Graph API app secret |
| `LINKEDIN_CLIENT_ID` | LinkedIn | Conditional | LinkedIn OAuth 2.0 app |
| `LINKEDIN_CLIENT_SECRET` | LinkedIn | Conditional | LinkedIn OAuth 2.0 app secret |
| `TOKEN_ENCRYPTION_KEY` | Token Storage | Optional | 32 bytes, encrypts tokens at rest |

---

## 14. Next Steps

### Completed ✅
1. ~~LinkedIn MCP tools~~ → 10 tools implemented (6 personal + 4 page), **2026-05-11**
2. ~~YouTube MCP tools~~ → 8 tools implemented (search, videos, playlists, comments, analytics, subscriptions), **2026-05-11**
3. ~~Pinterest MCP tools~~ → 6 tools implemented (user account, boards, pins, analytics), **2026-05-11**
4. ~~Discord MCP tools~~ → 4 tools implemented (channel, messages, guild, thread members), **2026-05-11**
5. ~~Skool MCP tool~~ → 1 tool implemented (publish wrapper), **2026-05-11**
6. ~~Phase 2: Discord/Skool/YouTube/Pinterest feature parity~~ → +12 tools (send message, delete, react, channels, server info, forum, skool info/posts/comments, creator discovery, pin search), **2026-05-11**

### Immediate (1-2 weeks)
7. **Bluesky MCP tools**: Profile, timeline, create post, search, thread (5 tools) — last orphan provider
8. **Fix linkedin_debug.rs**: Pre-existing compilation error (missing `SocialProvider` import)
9. **YouTube publish**: Fix `publish()` stub (currently returns "coming soon" error)

### Short-term (2-4 weeks)
10. End-to-end integration tests with real connected accounts
11. Provider-specific CLI commands via MCP client

### Medium-term (1-2 months)
12. Implement Voyager-style LinkedIn integration (separate sub-project, cookie auth) — unlocks messaging, jobs, people search, connections
13. Pinterest `publish()`: Support video and carousel pins (currently image-only)
14. Discord `publish()`: Support embeds for bot messages

### Infrastructure
15. SQLx offline cache for CI builds
16. API documentation (REST + MCP)
17. Integration test harness with Docker
