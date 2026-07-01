---
name: social-forge-agent
description: >
  Use Social Forge to manage social media accounts via CLI. Trigger this skill whenever the user wants to post, read, schedule, comment, DM, or manage any social media — X/Twitter, Reddit, LinkedIn, Facebook, Instagram, YouTube, Bluesky, Mastodon, TikTok, Threads, Discord, Slack, Telegram, WhatsApp, Pinterest, GitHub, WordPress, Hashnode, Medium, Dev.to, Skool, Google (Drive/Calendar/Gmail), or any connected provider. Also trigger on "post to X", "check my LinkedIn", "browse Reddit", "manage my socials", "schedule a tweet", "what's on my feed", "get Instagram insights", "post to Facebook page", "send a Telegram message", "manage Discord", "check WhatsApp", or any social media operation.
---

# Social Forge — AI Agent Guide

Social Forge manages social media across 25+ platforms from one CLI. All output is JSON (stdout). Errors are JSON on stderr with exit code 1.

**Interface**: `social-forge <command> [args]` — always use `--pretty` when reading output yourself.

---

## Output Format

All commands output JSON to stdout. Errors are JSON on stderr with exit code 1.

**Exit codes**: `0` = success, `1` = error (check stderr for `{"error":"..."}`).

**`--pretty`** = human-readable formatting (for reading). **`--json`** = compact single-line (for parsing/chaining). Default is `--json`.

### What `providers` returns:

```json
{"providers": [
  {"provider": "x", "name": "John Doe", "internal_id": "1234567890", "disabled": false},
  {"provider": "linkedin-page", "name": "My Company", "internal_id": "urn:li:page:123", "disabled": false}
]}
```

**The `internal_id` field is the integration UUID you need for scheduling and posting commands.**

### What `doctor` returns:

```json
{"checks": [
  {"provider": "x", "name": "John Doe", "internal_id": "123", "status": "healthy"},
  {"provider": "linkedin", "name": "John Doe", "internal_id": "456", "status": "error", "detail": "Token expired"}
], "healthy": 1, "total": 2, "needs_attention": [{"provider": "linkedin", "hint": "..."}]}
```

### What `posts list` returns:

```json
{"total": 10, "limit": 5, "offset": 0, "count": 5, "posts": [
  {"id": "a1b2c3d4-...", "content": "Hello!", "state": "published", "integration_id": "...", "scheduled_at": null, "published_at": "2026-07-01T12:00:00Z", "platform_post_id": "12345", "platform_post_url": "https://x.com/..."}
]}
```

### What `posts get` returns:

```json
{"id": "a1b2c3d4-...", "content": "Hello!", "state": "queued", "title": null, "integration_id": "...", "scheduled_at": "2026-07-02T09:00:00Z", "published_at": null, "media": [], "first_comment": null}
```

---

## Finding IDs

**Every command that takes `<UUID>`, `<PAGE_ID>`, or `<ACCOUNT_ID>` needs a prior discovery step.** Never guess IDs.

| ID Type | How to find it |
|---------|----------------|
| **Integration UUID** | `social-forge providers --pretty` → `internal_id` field |
| **LinkedIn Page ID** | `social-forge linkedin-page list --pretty` → `page_id` field |
| **Instagram Account ID** | `social-forge instagram accounts --pretty` → `account_id` field |
| **Reddit Subreddit** | Use the name directly (e.g., `rust`, `javascript`) |
| **Facebook Page ID** | `social-forge providers --pretty` → `internal_id` for provider `facebook` |
| **Post ID** | `social-forge posts list --limit 5 --pretty` → `id` field |

---

## Chaining Commands

When a command needs an ID from a previous command, chain them:

```bash
# Get integration UUID, then use it
social-forge providers --pretty
# → find the provider you want, note its internal_id

social-forge posts create "Hello" --integrations <INTERNAL_ID> --pretty
```

For shell scripting with `jq`:

```bash
# Extract UUID automatically
UUID=$(social-forge providers --pretty | jq -r '.providers[] | select(.provider=="linkedin-page") | .internal_id')
social-forge posts create "Update" --integrations "$UUID" --pretty
```

---

## CLI vs MCP Decision Tree

```
Does `social-forge <provider> <action>` exist in the recipes below?
  → YES: Use CLI (faster, simpler)
  → NO:  Use `social-forge mcp-call <tool_name> '<json>'`
```

**Platforms with native CLI posting**:
- X/Twitter → `social-forge x post "text"`
- LinkedIn personal → `social-forge linkedin post "text"`
- LinkedIn page → `social-forge linkedin-page post <PAGE_ID> "text"`
- Reddit → `social-forge reddit post --title "t" --text "b" --target sub`
- Facebook → `social-forge facebook create <PAGE_ID> "message"`
- Bluesky → `social-forge bluesky post "text"`
- Mastodon → `social-forge mastodon post "text"`
- TikTok → `social-forge tiktok post "text"`
- Threads → `social-forge threads post <ACCOUNT_ID> "text"`
- Discord → `social-forge discord send <CHANNEL_ID> "text"`
- Slack → `social-forge slack send <CHANNEL_ID> "text"`
- Telegram Bot → `social-forge telegram-bot send --chat-id <ID> "text"`
- Telegram User → `social-forge telegram-user send <PEER> "text"`
- WhatsApp → `social-forge whatsapp send <PHONE> "text"`
- WordPress → `social-forge wordpress post "title" "content"`
- Dev.to → `social-forge devto post "content"`
- Hashnode → `social-forge hashnode post <PUB_ID> "title" "content"`
- Medium → `social-forge medium-blog post "content"`
- Skool → `social-forge skool post <GROUP_ID> "title" "content"`

**Platforms that need MCP for posting** (no native CLI post command):
- Instagram → `social-forge mcp-call ig_create_container` + `ig_publish_container` (3-step, see below)

---

## Step Zero: Discovery

**Before doing anything**, run these commands to understand what's available:

```bash
social-forge --help                    # See all top-level commands
social-forge <command> --help          # See args for a specific command
social-forge providers --pretty        # See what accounts are connected
social-forge doctor                    # Check health of all providers
```

If no providers are connected, run:
```bash
social-forge connect <provider> --pretty   # Shows the URL to visit
```

**Never guess command names.** Always check `--help` first.

---

## Task Recipes

### Post to one platform

```bash
# X/Twitter
social-forge x post "Hello world!" --pretty

# LinkedIn personal
social-forge linkedin post "New update!" --pretty

# LinkedIn company page
social-forge linkedin-page post <PAGE_ID> "New update!" --pretty

# Reddit
social-forge reddit post --title "My post" --text "Body text here" --target rust --pretty

# Instagram (requires media)
social-forge post "Check this out" --platforms instagram --media ./photo.jpg --first-comment "Link in bio!"
```

**Verify**: `social-forge posts list --limit 1 --pretty` — check the post appears with `state: "published"`.

### Post to multiple platforms at once

```bash
social-forge post "Shipping new features!" --platforms x,linkedin,bluesky --pretty
```

Content is auto-split to fit each platform's character limit. Threads are auto-numbered (1/N) for X, Bluesky, and Mastodon.

**Verify**: `social-forge posts list --limit 3 --pretty` — check all platform posts appear.

### Schedule a post

```bash
# Create and arm for a future time
social-forge posts create "Morning update" --integrations <UUID> --schedule "2026-07-02T09:00:00Z" --pretty

# Find the next available slot
social-forge posts find-slot --integration <UUID> --pretty

# List scheduled posts
social-forge posts list --limit 10 --pretty

# Cancel/reschedule
social-forge posts delete <POST_ID> --pretty
```

**Verify**: `social-forge posts get <POST_ID> --pretty` — check `state: "queued"` and `scheduled_at` matches.

### Read a feed

```bash
# X/Twitter timeline
social-forge x timeline --count 5 --pretty

# Reddit subreddit
social-forge reddit browse rust --sort hot --pretty

# Reddit search
social-forge reddit search "async rust" --pretty

# Instagram posts
social-forge instagram posts <ACCOUNT_ID> --pretty

# LinkedIn company posts
social-forge linkedin-page posts <PAGE_ID> --pretty

# Facebook page feed
social-forge facebook posts <PAGE_ID> --pretty

# Import from a provider into local store
social-forge import x --count 20 --pretty

# Browse unified feed (all providers)
social-forge feed --pretty
social-forge feed --provider x --pretty
```

### Check analytics

```bash
# Instagram insights
social-forge instagram insights <ACCOUNT_ID> --metric reach,follower_count --pretty

# LinkedIn page analytics
social-forge linkedin-page analytics <PAGE_ID> --pretty

# LinkedIn page followers
social-forge linkedin-page followers <PAGE_ID> --pretty
```

**Instagram metrics** (comma-separated):
- Day period: `reach`, `follower_count`, `online_followers`
- Lifetime: `total_interactions`, `comments`, `shares`, `saves`, `likes`, `accounts_engaged`, `profile_views`, `website_clicks`

### Manage media

```bash
# Upload a file
social-forge media upload ./photo.jpg --alt "Description" --pretty

# Upload from URL
social-forge media upload https://example.com/image.png --pretty

# List uploaded media
social-forge media list --limit 10 --pretty

# Upload multiple files
social-forge media upload-batch ./img1.jpg ./img2.jpg ./img3.jpg --pretty
```

### Create a carousel

```bash
social-forge carousel create "5 Tips for Devs" --images tip1.jpg tip2.jpg tip3.jpg --platforms x,linkedin --pretty
```

### Comment and reply

```bash
# Get comments on a post
social-forge comment get <INTEGRATION_ID> <POST_ID> --limit 20 --pretty

# Reply to a comment
social-forge comment reply <INTEGRATION_ID> <COMMENT_ID> "Thanks for feedback!" --pretty

# Delete a comment
social-forge comment delete <INTEGRATION_ID> <COMMENT_ID> --pretty
```

### Send a DM

```bash
# List conversations
social-forge dm list <INTEGRATION_ID> --limit 20 --pretty

# Get messages in a conversation
social-forge dm messages <INTEGRATION_ID> <CONVERSATION_ID> --limit 20 --pretty

# Send a message
social-forge dm send <INTEGRATION_ID> <RECIPIENT_ID> "Hello!" --pretty
```

### Stage a post (review before publish)

```bash
# Stage across multiple platforms
social-forge stage "Long content that will be split..." --integrations <UUID1>,<UUID2> --pretty

# Preview splits without creating drafts
social-forge stage "Preview mode" --platforms x,linkedin --preview --pretty
```

### Set up automation

```bash
# Auto-reply to comments
social-forge automation create <INTEGRATION_ID> "Auto-thanks" --trigger comment --response "Thanks for commenting!" --type fixed --pretty

# List rules
social-forge automation list --pretty

# View execution logs
social-forge automation logs <RULE_ID> --limit 50 --pretty
```

### Use MCP tools directly (advanced)

If a CLI command doesn't exist for what you need, use `mcp-call`:

```bash
# List all 311 MCP tools
social-forge mcp-tools --pretty

# Call any tool by name
social-forge mcp-call fb_create_post '{"page_id":"123","message":"Hello"}'
social-forge mcp-call x_home_timeline '{"count":5}'
social-forge mcp-call posts_create '{"content":"test","platforms":["x"]}'
```

### Read from any platform

```bash
# TikTok
social-forge tiktok profile --pretty
social-forge tiktok videos --limit 10 --pretty

# Threads
social-forge threads list <ACCOUNT_ID> --limit 10 --pretty
social-forge threads profile <ACCOUNT_ID> --pretty

# Discord
social-forge discord channels <GUILD_ID> --pretty
social-forge discord messages <CHANNEL_ID> --limit 10 --pretty
social-forge discord server <GUILD_ID> --pretty

# Slack
social-forge slack channels --pretty
social-forge slack history <CHANNEL_ID> --limit 10 --pretty
social-forge slack users --pretty

# Telegram Bot
social-forge telegram-bot updates --pretty
social-forge telegram-bot chat --chat-id <ID> --pretty

# Telegram User
social-forge telegram-user dialogs --limit 10 --pretty
social-forge telegram-user contacts --pretty
social-forge telegram-user search "query" --pretty

# WhatsApp
social-forge whatsapp chats --limit 10 --pretty
social-forge whatsapp contacts --pretty
social-forge whatsapp groups --pretty

# Pinterest
social-forge pinterest profile <BOARD_ID> --pretty
social-forge pinterest board <BOARD_ID> --pretty
social-forge pinterest pins <BOARD_ID> --limit 10 --pretty
social-forge pinterest search "query" --pretty

# GitHub
social-forge github me --pretty
social-forge github my-repos --limit 10 --pretty
social-forge github issues <OWNER> <REPO> --pretty
social-forge github prs <OWNER> <REPO> --pretty
social-forge github commits <OWNER> <REPO> --limit 10 --pretty
social-forge github search "query" --pretty

# WordPress
social-forge wordpress list --limit 10 --pretty
social-forge wordpress get <POST_ID> --pretty
social-forge wordpress categories --pretty

# Hashnode
social-forge hashnode list <PUB_ID> --pretty
social-forge hashnode get <POST_ID> --pretty

# Medium
social-forge medium-blog list --pretty
social-forge medium-blog get <POST_ID> --pretty

# Dev.to
social-forge devto list --pretty
social-forge devto get <POST_ID> --pretty

# Skool
social-forge skool posts <SLUG> --pretty
social-forge skool info <SLUG> --pretty

# Google Workspace
social-forge google youtube-search <CHANNEL_ID> "query" --pretty
social-forge google video <CHANNEL_ID> <VIDEO_ID> --pretty
social-forge google playlists <CHANNEL_ID> --pretty
social-forge google channel-stats <CHANNEL_ID> --pretty

# Google Drive
social-forge gdrive files --limit 10 --pretty
social-forge gdrive search "query" --pretty
social-forge gdrive folders --pretty

# Google Calendar
social-forge gcal calendars --pretty
social-forge gcal events --limit 10 --pretty
social-forge gcal event --event-id <ID> --pretty

# Gmail
social-forge gmail-ops messages --limit 10 --pretty
social-forge gmail-ops search "query" --pretty
social-forge gmail-ops labels --pretty
social-forge gmail-ops profile --pretty

# Webhooks
social-forge webhooks list --pretty
social-forge webhooks get <ID> --pretty

# Notifications
social-forge notifications list --limit 10 --pretty

# Tags
social-forge tags list --pretty
social-forge tags get <ID> --pretty

# Analytics
social-forge analytics get <PROVIDER> --days 7 --pretty
social-forge analytics post <POST_ID> --pretty
```

### Engage on any platform

```bash
# X
social-forge x like <TWEET_ID>
social-forge x retweet <TWEET_ID>
social-forge x bookmark <TWEET_ID>
social-forge x reply <TWEET_ID> "reply text"

# Reddit
social-forge reddit vote <THING_ID> up
social-forge reddit save <THING_ID>
social-forge reddit comment <THING_ID> "reply text"

# Facebook
social-forge facebook react <POST_ID> LIKE
social-forge facebook comment <POST_ID> "reply text"

# Instagram
social-forge instagram comment <MEDIA_ID> "reply text"

# LinkedIn
social-forge linkedin reply <COMMENT_ID> "reply text"

# Discord
social-forge discord send <CHANNEL_ID> "reply text"
social-forge discord forum <CHANNEL_ID> "title" "content"

# Slack
social-forge slack send <CHANNEL_ID> "reply text"

# WhatsApp
social-forge whatsapp send <PHONE> "reply text"

# Telegram
social-forge telegram-bot send --chat-id <ID> "reply text"
social-forge telegram-user send <PEER> "reply text"

# GitHub
social-forge github create-issue <OWNER> <REPO> "title" --body "description"
social-forge github close-issue <OWNER> <REPO> <NUMBER>

# WordPress
social-forge wordpress post "title" "content"

# Hashnode
social-forge hashnode post <PUB_ID> "title" "content"

# Medium
social-forge medium-blog post "content"

# Dev.to
social-forge devto post "content"

# Skool
social-forge skool post <GROUP_ID> "title" "content"
social-forge skool comment <POST_ID> <GROUP_ID> "reply text"

# Google Calendar
social-forge gcal create --title "Meeting" --start "2026-07-02T09:00:00Z" --end "2026-07-02T10:00:00Z"
social-forge gcal delete --event-id <ID>

# Gmail
social-forge gmail-ops send --to "user@example.com" --subject "Hi" --body "Hello!"

# Webhooks
social-forge webhooks create "https://example.com/hook" --name "my-hook"
social-forge webhooks test <ID>
social-forge webhooks delete <ID>

# Tags
social-forge tags create "work" --color "#ff0000"
social-forge tags delete <ID>

# Notifications
social-forge notifications read <ID>
social-forge notifications read-all

# Analytics
social-forge analytics get instagram --days 7
social-forge analytics post <POST_ID>
```

---

## Error Recovery

| Error | Cause | Fix |
|-------|-------|-----|
| `No X/Twitter integration found` | X not connected | `social-forge connect x` → visit URL |
| `No Reddit integration found` | Reddit not connected | `social-forge connect reddit` → cookie auth |
| `No LinkedIn account connected` | LinkedIn not connected | `social-forge connect linkedin` → OAuth flow |
| `Facebook page not connected` | Wrong page ID | `social-forge providers --pretty` → find correct ID |
| `Instagram account not connected` | Wrong account ID | Use Instagram Business Account ID, not username |
| `Token expired` | OAuth token expired | User must re-authenticate via connect flow |
| `Rate limit exceeded` (HTTP 429) | Too many API calls | Wait, space out requests |
| `No virtual resource found` | LinkedIn page too new | Try a different page or wait |
| `API error: (#100)` | Invalid metric/permission | Check metric names; reconnect with proper scopes |
| `command not found: social-forge` | Binary not installed | Run install script or build from source |
| `connection refused` | Server not running | Run `social-forge serve` or check systemd status |
| `permission denied` | File permissions | Check `.social-forge/` directory permissions |
| `No integration found for provider` | Provider not connected | `social-forge connect <provider>` |
| `unknown flag` | Wrong CLI syntax | Run `social-forge <command> --help` to see correct flags |
| `Failed to parse` / `invalid value` | Wrong argument type | Check `--help` for expected types (IDs are strings, not numbers) |
| `Error: Serialize` / JSON error | Unexpected output format | Check if output is valid JSON; use `--pretty` for debugging |

**Always run `social-forge doctor` first** to check which providers are healthy.

---

## MCP Server Setup

For MCP clients (Claude Desktop, Cursor, etc.):

```json
{
  "mcpServers": {
    "social-forge": {
      "command": "social-forge",
      "args": ["mcp"]
    }
  }
}
```

See [Provider Reference](references/providers.md) for the complete list of CLI commands and MCP tools per provider.

---

## Authentication Priority

1. DB-stored cookie tokens (richest access — enables voting, moderation, GraphQL)
2. Browser cookie extraction (auto-detected from Chrome/Brave/Firefox/Zen)
3. OAuth tokens from DB (standard API access)
4. Environment variables (fallback: `X_AUTH_TOKEN`, `X_CT0`, etc.)

Cookie auth gives the fullest access. OAuth is the fallback for platforms without cookie support.
