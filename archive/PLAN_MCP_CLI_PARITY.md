# MCP ↔ CLI Parity Implementation Plan

**Goal**: Every MCP tool must have a corresponding CLI command. The CLI must be fully functional for all social-forge operations — no feature gap, no MCP-only features.

**Strategy**: Extend CLI with native subcommands for all MCP tools. Each platform gets `<platform> <action>` CLI commands. The final result: a single `social-forge-agent` skill covers both CLI and MCP workflows.

---

## Phase 1: Foundation — Shared Service Layer (P0)

**Why first**: Many MCP tools bypass `PostService` with direct provider calls. CLI needs a unified entry point.

### 1.1 Create `PlatformService` (new file: `src/services/platform.rs`)

A thin service layer that wraps provider-specific operations and is callable from both CLI and MCP:

```rust
pub struct PlatformService<'a> {
    state: &'a AppState,
    user_id: Uuid,
}

impl<'a> PlatformService<'a> {
    pub async fn get_profile(&self, provider: &str) -> Result<serde_json::Value>;
    pub async fn get_feed(&self, provider: &str, limit: u32) -> Result<serde_json::Value>;
    pub async fn get_post(&self, provider: &str, post_id: &str) -> Result<serde_json::Value>;
    pub async fn create_post(&self, provider: &str, content: &str, opts: PostOptions) -> Result<serde_json::Value>;
    pub async fn delete_post(&self, provider: &str, post_id: &str) -> Result<serde_json::Value>;
    pub async fn get_comments(&self, provider: &str, post_id: &str) -> Result<serde_json::Value>;
    pub async fn create_comment(&self, provider: &str, post_id: &str, text: &str) -> Result<serde_json::Value>;
    pub async fn get_analytics(&self, provider: &str, days: u32) -> Result<serde_json::Value>;
    pub async fn send_dm(&self, provider: &str, recipient: &str, text: &str) -> Result<serde_json::Value>;
    // ... etc
}
```

Each method loads the integration, decrypts the token, and delegates to the provider. This eliminates the ~260 individual MCP handler functions doing the same boilerplate.

### 1.2 Refactor existing MCP handlers

Migrate `tools_*.rs` handlers to delegate to `PlatformService` instead of reimplementing provider access. This:
- Reduces MCP tool code by ~60%
- Ensures CLI and MCP share identical logic
- Makes adding new CLI commands trivial (just call `PlatformService`)

**Files modified**: All `src/mcp/tools_*.rs` files
**New file**: `src/services/platform.rs`

---

## Phase 2: Platform CLI Commands (P1)

Each platform gets a CLI subcommand with read/write actions. Pattern:

```bash
social-forge <platform> <action> [args] [--json|--pretty]
```

### 2.1 Read-Only Platforms (add write capability)

| Platform | CLI Current | MCP Has | CLI Add |
|----------|------------|---------|---------|
| **TikTok** | status only | profile, create, list_videos | `tiktok profile`, `tiktok post "text"`, `tiktok videos` |
| **Threads** | status only | profile, threads, thread_detail, replies, reply, create, delete, insights, poll | `threads profile`, `threads list`, `threads post "text"`, `threads reply <id> "text"`, `threads delete <id>`, `threads insights <id>` |
| **Bluesky** | status only | profile, timeline, create, search, feed, reply | `bluesky profile`, `bluesky timeline`, `bluesky post "text"`, `bluesky search "query"`, `bluesky feed`, `bluesky reply <id> "text"` |
| **Mastodon** | status only | create, timeline, get, search, reply | `mastodon post "text"`, `mastodon timeline`, `mastodon get <id>`, `mastodon search "query"`, `mastodon reply <id> "text"` |
| **Facebook** | status only | feed, post, comments, create, photo, video, delete, comment, react, page_insights, conversations, msgs, send_message, search_pages, albums | `facebook feed`, `facebook post "text"`, `facebook photo <url>`, `facebook comments <id>`, `facebook reply <id> "text"`, `facebook pages`, `facebook page-insights`, `facebook conversations`, `facebook send <recipient> "text"` |

### 2.2 Platform-Specific Features (new CLI commands)

| Platform | MCP Tools | CLI Add |
|----------|-----------|---------|
| **Instagram** | get_media, get_media_detail, get_comments, search_hashtag, get_hashtag_media, get_insights, get_tagged, create_container, publish_container, reply_to_comment, get_reels, get_stories, get_followers, business_discovery, get_insights_audience, get_mentions, poll_container, send_dm, list_conversations, get_messages | `instagram media`, `instagram post "caption" --media-url <url>`, `instagram reels`, `instagram stories`, `instagram followers`, `instagram insights`, `instagram hashtag <query>`, `instagram dm <recipient> "text"`, `instagram conversations` |
| **LinkedIn Page** | list_pages, get_page, get_page_posts, create_comment, create_post, get_analytics, get_post_analytics, get_followers, delete_post, get_reactions, get_shares | `linkedin-page list`, `linkedin-page post "text" --page <id>`, `linkedin-page feed <page_id>`, `linkedin-page analytics <page_id>`, `linkedin-page followers <page_id>` |
| **YouTube** | search_videos, get_video, list_playlists, get_playlist_items, get_comments, get_channel_stats, get_analytics, get_subscriptions, find_creators, reply_comment | `youtube search "query"`, `youtube video <id>`, `youtube playlists`, `youtube stats`, `youtube analytics`, `youtube subscriptions`, `youtube reply <comment_id> "text"` |
| **Discord** | get_channel, get_messages, get_guild, get_thread_members, send_message, delete_message, add_reaction, get_guild_channels, get_server_info, create_forum_post | `discord channels`, `discord messages <channel_id>`, `discord send <channel_id> "text"`, `discord server`, `discord forum <channel_id> "title" "content"` |
| **Slack** | send_message, list_channels, channel_history, list_users | `slack channels`, `slack history <channel_id>`, `slack send <channel_id> "text"`, `slack users` |
| **Telegram Bot** | send_message, get_updates, get_me, get_chat, get_chat_member_count, send_photo, send_document, forward_message, pin_message, unpin_message | `telegram-bot send <chat_id> "text"`, `telegram-bot photo <chat_id> <url>`, `telegram-bot document <chat_id> <path>`, `telegram-bot chat <chat_id>`, `telegram-bot updates` |
| **WhatsApp** | auth_status, send_text, chats, contacts, edit_message, revoke_message, list_groups, create_group, group_invite_link | `whatsapp send <phone> "text"`, `whatsapp chats`, `whatsapp contacts`, `whatsapp groups`, `whatsapp group create "name"`, `whatsapp invite-link <group_id>` |
| **Telegram User** | auth_status, send_message, list_dialogs, list_contacts, search, request_code, sign_in | `telegram send <dialog_id> "text"`, `telegram dialogs`, `telegram contacts`, `telegram search "query"` |
| **Pinterest** | get_user_account, get_board, get_board_pins, get_pin, get_board_analytics, get_pin_analytics, search_pins | `pinterest profile`, `pinterest board <id>`, `pinterest pins <board_id>`, `pinterest pin <id>`, `pinterest search "query"`, `pinterest board-analytics <id>`, `pinterest pin-analytics <id>` |
| **GitHub** | get_authenticated_user, get_user, list_repos, get_repo, list_issues, get_issue, create_issue, list_pull_requests, get_pull_request, list_commits, list_branches, list_releases, search_repos, search_code, list_contributors, get_repo_content, close_issue, list_my_repos | `github repos`, `github issues <repo>`, `github prs <repo>`, `github create-issue <repo> "title"`, `github close-issue <repo> <number>`, `github commits <repo>`, `github branches <repo>`, `github search "query"`, `github user [username]`, `github releases <repo>` |

### 2.3 Blogging Platforms (new CLI commands)

| Platform | MCP Tools | CLI Add |
|----------|-----------|---------|
| **WordPress** | create_post, list_posts, get_post, list_categories | `wordpress post "title" "content"`, `wordpress list`, `wordpress get <id>`, `wordpress categories` |
| **Hashnode** | create_post, list_posts, get_post | `hashnode post "title" "content"`, `hashnode list`, `hashnode get <id>` |
| **Medium** | create_post, list_posts, get_post | `medium post "title" "content"`, `medium list`, `medium get <id>` |
| **Dev.to** | create_post, list_posts, get_post | `devto post "title" "content"`, `devto list`, `devto get <id>` |
| **Skool** | publish, get_info, list_posts, get_post, create_comment | `skool post "title" "content"`, `skool info`, `skool posts`, `skool comment <post_id> "text"` |

### 2.4 Analytics & Insights (new CLI commands)

| Feature | MCP Tools | CLI Add |
|---------|-----------|---------|
| **Analytics** | analytics_get, analytics_get_post | `analytics get <provider>`, `analytics post <post_id>` |
| **Instagram Insights** | ig_get_insights, ig_get_insights_audience | `instagram insights <ig_id>`, `instagram audience <ig_id>` |
| **LinkedIn Analytics** | li_get_analytics, li_get_post_analytics | `linkedin analytics`, `linkedin post-analytics <post_id>` |
| **LinkedIn Page Analytics** | lip_get_analytics, lip_get_post_analytics | `linkedin-page analytics <page_id>`, `linkedin-page post-analytics <post_id>` |
| **Facebook Page Insights** | fb_page_insights | `facebook page-insights <page_id>` |
| **YouTube Analytics** | yt_get_analytics, yt_get_channel_stats | `youtube analytics`, `youtube stats` |
| **Pinterest Analytics** | pi_get_board_analytics, pi_get_pin_analytics | `pinterest board-analytics <id>`, `pinterest pin-analytics <id>` |
| **Threads Insights** | th_get_insights | `threads insights <id>` |

### 2.5 System Features (new CLI commands)

| Feature | MCP Tools | CLI Add |
|---------|-----------|---------|
| **Webhooks** | wh_create, wh_list, wh_get, wh_update, wh_delete, wh_test | `webhooks list`, `webhooks create <url>`, `webhooks get <id>`, `webhooks update <id>`, `webhooks delete <id>`, `webhooks test <id>` |
| **Notifications** | notif_list, notif_mark_read, notif_mark_all_read, notif_create | `notifications list`, `notifications read <id>`, `notifications read-all`, `notifications create "title" "body"` |
| **Tags** | tag_create, tag_list, tag_get, tag_update, tag_delete | `tags list`, `tags create "name"`, `tags get <id>`, `tags update <id> "name"`, `tags delete <id>` |
| **Feed** | feed_list, feed_import | `feed list`, `feed import <url>` |
| **DM** (unified) | dm_send, dm_list, dm_messages | `dm send <integration_id> <recipient> "text"`, `dm list <integration_id>`, `dm messages <integration_id> <conversation_id>` |
| **Comments** (unified) | comments_get, comments_reply, comments_delete | `comments get <post_id>`, `comments reply <comment_id> "text"`, `comments delete <comment_id>` |
| **Google** | goog_search_videos, goog_get_video, goog_get_playlists, goog_get_playlist_items, goog_get_comments, goog_get_channel_stats, goog_get_analytics, goog_get_subscriptions, goog_find_creators, goog_get_profile, goog_list_messages, goog_get_message, goog_send_message, goog_list_labels, goog_get_thread, goog_search_messages, goog_list_calendars, goog_list_events, goog_get_event, goog_create_event, goog_update_event, goog_delete_event | `google youtube-search "query"`, `google video <id>`, `google playlists`, `google gmail`, `google send <to> "subject" "body"`, `google calendars`, `google events`, `google create-event "title" "start" "end"` |
| **Drive** | dr_list_files, dr_get_file, dr_search_files, dr_list_folders, dr_get_file_metadata, dr_export_file | `drive files`, `drive file <id>`, `drive search "query"`, `drive folders`, `drive metadata <id>`, `drive export <id>` |
| **GCal** | gcal_list_calendars, gcal_list_events, gcal_get_event, gcal_create_event, gcal_update_event, gcal_delete_event | `gcal calendars`, `gcal events`, `gcal event <id>`, `gcal create "title" "start" "end"`, `gcal update <id>`, `gcal delete <id>` |
| **Gmail** | gm_get_profile, gm_list_messages, gm_get_message, gm_send_message, gm_list_labels, gm_get_thread, gm_search_messages | `gmail profile`, `gmail messages`, `gmail message <id>`, `gmail send <to> "subject" "body"`, `gmail labels`, `gmail thread <id>`, `gmail search "query"` |

---

## Phase 3: CLI Implementation Pattern (P2)

### 3.1 New file structure

```
src/cli/
  run.rs              # Main CLI dispatch (keep existing)
  mod.rs              # Module declarations
  mcp_bridge.rs       # mcp-call fallback (keep existing)
  platforms/          # NEW: platform CLI handlers
    mod.rs
    tiktok.rs
    threads.rs
    bluesky.rs
    mastodon.rs
    facebook.rs
    instagram.rs
    linkedin_page.rs
    youtube.rs
    discord.rs
    slack.rs
    telegram_bot.rs
    telegram_user.rs
    whatsapp.rs
    pinterest.rs
    github.rs
    wordpress.rs
    hashnode.rs
    medium.rs
    devto.rs
    skool.rs
    google.rs
    drive.rs
    gcal.rs
    gmail.rs
  system/             # NEW: system CLI handlers
    mod.rs
    webhooks.rs
    notifications.rs
    tags.rs
    feed.rs
    analytics.rs
```

### 3.2 Implementation pattern per platform

Each platform file follows this pattern:

```rust
// src/cli/platforms/tiktok.rs
use serde_json::{json, Value};
use crate::api::AppState;

pub async fn handle_tiktok(sub: &str, args: &[String], state: &AppState, user_id: uuid::Uuid) -> anyhow::Result<()> {
    match sub {
        "profile" => {
            let profile = PlatformService::new(state, user_id).get_profile("tiktok").await?;
            output_json(&profile);
        }
        "post" => {
            let text = args.first().ok_or("Usage: social-forge tiktok post \"text\"")?;
            let result = PlatformService::new(state, user_id).create_post("tiktok", text, Default::default()).await?;
            output_json(&result);
        }
        "videos" => {
            let limit = args.first().and_then(|s| s.parse().ok()).unwrap_or(10);
            let feed = PlatformService::new(state, user_id).get_feed("tiktok", limit).await?;
            output_json(&feed);
        }
        _ => {
            eprintln!("Unknown tiktok action: {sub}. Use: profile, post, videos");
            std::process::exit(1);
        }
    }
    Ok(())
}
```

### 3.3 MCP handler refactoring

Replace direct provider calls with `PlatformService`:

```rust
// Before (260+ lines of boilerplate per handler):
pub async fn handle_tt_profile(state: &AppState, input: &TtProfileInput) -> Result<Json<Value>, String> {
    let user_id = resolve_first_user(state).await?;
    let integrations = list_integrations(&state.db, user_id).await.map_err(|e| e.to_string())?;
    let integration = integrations.iter().find(|i| i.provider_identifier == "tiktok").ok_or("Not connected")?;
    let token = decrypt_token(state, integration).await?;
    let provider = TikTokProvider::new(&state.config);
    // ... 20 more lines
}

// After (1 line):
pub async fn handle_tt_profile(state: &AppState, _input: &TtProfileInput) -> Result<Json<Value>, String> {
    let user_id = resolve_first_user(state).await?;
    PlatformService::new(state, user_id).get_profile("tiktok").await.map(Json)
}
```

---

## Phase 4: Skill Rewrite (P3)

After all CLI commands exist, rewrite `social-forge-agent` skill.

### 4.1 `SKILL.md` — Complete rewrite

**Structure**:
```
# Social Forge Agent

## Step Zero: Discovery
- `social-forge doctor` — system health (show output example)
- `social-forge providers --pretty` — connected accounts (show output example)

## Output Format
- All commands output JSON
- `--pretty` = human-readable, `--json` = machine-parseable
- Exit code 0 = success, exit code 1 = error (stderr JSON)

## Finding IDs
- Integration UUID: `social-forge providers --pretty`
- LinkedIn page: `social-forge linkedin-page list --pretty`
- Instagram account: `social-forge instagram media <ig_id> --limit 1`
- Reddit target: `social-forge reddit browse rust --limit 1`
- Facebook page: `social-forge facebook pages --pretty`

## CLI vs MCP Decision Tree
Does `social-forge <platform> <action>` exist?
  YES → Use CLI
  NO  → Use `social-forge mcp-call <tool_name> '<json>'`

## Recipes (copy-paste)
[Every workflow as concrete commands with real output examples]

## Error Recovery
[Exit code 1 patterns, stderr JSON parsing]
```

### 4.2 `references/providers.md` — Platform-by-platform reference

Group by platform with CLI commands and MCP fallbacks:

```markdown
## TikTok
CLI: `tiktok profile`, `tiktok post`, `tiktok videos`
MCP: none needed (CLI covers all)

## Instagram
CLI: `instagram media`, `instagram post`, `instagram reels`, ...
MCP: `ig_create_container` + `ig_poll_container` + `ig_publish_container` (2-step flow)
```

### 4.3 `references/quick-reference.md` — Cheat sheet

```markdown
# Social Forge Quick Reference

## Post
social-forge posts create "text" --integrations <uuid> --json
social-forge x post "text" --json
social-forge linkedin post "text" --json

## Read
social-forge x timeline --count 5 --json
social-forge reddit feed rust --limit 5 --json
social-forge linkedin feed --limit 5 --json

## Media
social-forge media upload ./file.jpg --json
social-forge media list --limit 10 --json

## Schedule
social-forge posts create "text" --integrations <uuid> --schedule 2026-07-01T09:00:00Z --json
social-forge posts find-slot --integration <uuid> --json

## Analytics
social-forge analytics get instagram --json
social-forge instagram insights <ig_id> --json

## System
social-forge doctor --json
social-forge providers --json
social-forge webhooks list --json
social-forge notifications list --json
social-forge tags list --json
```

### 4.4 `evals/evals.json` — Expanded with expected commands

Each eval gets an `expected_commands` array:

```json
{
  "id": "post_to_x",
  "prompt": "Post 'Hello world' to X/Twitter",
  "expected_commands": ["social-forge x post 'Hello world' --json"],
  "expected_output_pattern": "\"status\":\"published\""
}
```

---

## Phase 5: Testing & Verification (P4)

### 5.1 CLI integration tests

For each new CLI command, add a test that:
1. Calls the command with `--json` output
2. Parses the JSON response
3. Verifies the expected structure exists

### 5.2 MCP handler tests

Refactor existing MCP handler tests to verify they produce identical output to the new CLI commands when given the same input.

### 5.3 Parity verification script

```bash
#!/bin/bash
# Verify CLI covers all MCP tools
MCP_TOOLS=$(grep "pub async fn handle_" src/mcp/tools_*.rs | wc -l)
CLI_COMMANDS=$(grep -E '^\s+"[a-z]' src/cli/run.rs src/cli/platforms/*.rs | wc -l)
echo "MCP tools: $MCP_TOOLS, CLI commands: $CLI_COMMANDS"
# Should be roughly equal (CLI may have more due to subcommands)
```

---

## Implementation Order

| Phase | What | Est. Files | Priority |
|-------|------|-----------|----------|
| **P1** | `PlatformService` + MCP refactoring | 1 new, ~32 modified | P0 |
| **P1** | 5 read-only platforms (TikTok, Threads, Bluesky, Mastodon, Facebook) | 5 new | P0 |
| **P2** | Instagram, LinkedIn Page, YouTube, Discord, Slack, Telegram, WhatsApp, Pinterest, GitHub | 15 new | P1 |
| **P2** | Blogging platforms (WordPress, Hashnode, Medium, Dev.to, Skool) | 5 new | P1 |
| **P2** | System features (Webhooks, Notifications, Tags, Feed, DM, Comments, Google, Drive, GCal, Gmail) | 10 new | P1 |
| **P3** | Analytics CLI commands | 1 new | P2 |
| **P4** | Skill rewrite (SKILL.md, providers.md, quick-reference.md, evals.json) | 4 modified | P2 |
| **P5** | Integration tests + parity verification | 1 new, test files | P2 |

**Total new files**: ~36 CLI platform handlers + 1 service + tests
**Total modified files**: ~32 MCP tool files (refactored)

---

## CLI Command Count After Implementation

| Category | Current | After |
|----------|---------|-------|
| Posts | 7 | 7 |
| Media | 4 | 4 |
| Scheduling | 3 | 3 |
| X/Twitter | 1 | 1 |
| Reddit | 2 | 2 |
| LinkedIn | 2 | 7 |
| Facebook | 1 | 12 |
| Instagram | 0 | 15 |
| TikTok | 0 | 3 |
| Threads | 0 | 6 |
| Bluesky | 0 | 6 |
| Mastodon | 0 | 5 |
| YouTube | 0 | 8 |
| Discord | 0 | 5 |
| Slack | 0 | 4 |
| Telegram Bot | 0 | 5 |
| Telegram User | 0 | 4 |
| WhatsApp | 0 | 6 |
| Pinterest | 0 | 7 |
| GitHub | 0 | 14 |
| WordPress | 0 | 4 |
| Hashnode | 0 | 3 |
| Medium | 0 | 3 |
| Dev.to | 0 | 3 |
| Skool | 0 | 4 |
| Google | 0 | 11 |
| Drive | 0 | 6 |
| GCal | 0 | 6 |
| Gmail | 0 | 7 |
| Analytics | 0 | 2 |
| Webhooks | 0 | 6 |
| Notifications | 0 | 4 |
| Tags | 0 | 5 |
| Feed | 0 | 2 |
| DM (unified) | 0 | 3 |
| Comments (unified) | 0 | 3 |
| **Total** | **~45** | **~220** |
