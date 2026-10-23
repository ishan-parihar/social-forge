---
name: switchpost-agent
description: >
  Use Switchpost to manage social media accounts via CLI. Trigger this skill whenever the user wants to post, read, schedule, comment, DM, or manage any social media — X/Twitter, Reddit, LinkedIn, Facebook, Instagram, YouTube, Bluesky, Mastodon, TikTok, Threads, Discord, Slack, Telegram, WhatsApp, Pinterest, GitHub, WordPress, Hashnode, Medium, Dev.to, Skool, Google (Drive/Calendar/Gmail), or any connected provider. Also trigger on "post to X", "check my LinkedIn", "browse Reddit", "manage my socials", "schedule a tweet", "what's on my feed", "get Instagram insights", "post to Facebook page", "send a Telegram message", "manage Discord", "check WhatsApp", or any social media operation.
---

# Switchpost — AI Agent Guide

Switchpost manages social media across 25+ platforms from one CLI. All output is JSON (stdout). Errors are JSON on stderr with exit code 1.

**Interface**: `switchpost <command> [args]` — always use `--pretty` when reading output yourself.

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

### What `posts list` returns:

```json
{"total": 10, "limit": 5, "offset": 0, "count": 5, "posts": [
  {"id": "a1b2c3d4-...", "content": "Hello!", "state": "published", "integration_id": "...", "scheduled_at": null, "published_at": "2026-07-01T12:00:00Z", "platform_post_id": "12345", "platform_post_url": "https://x.com/..."}
]}
```

---

## Finding IDs

**Every command that takes `<UUID>`, `<PAGE_ID>`, or `<ACCOUNT_ID>` needs a prior discovery step.** Never guess IDs.

| ID Type | How to find it |
|---------|----------------|
| **Integration UUID** | `switchpost providers --pretty` → `internal_id` field |
| **LinkedIn Page ID** | `switchpost linkedin-page list --pretty` → `page_id` field |
| **Instagram Account ID** | `switchpost providers --pretty` → `internal_id` for provider `instagram` |
| **Reddit Subreddit** | Use the name directly (e.g., `rust`, `javascript`) |
| **Facebook Page ID** | `switchpost providers --pretty` → `internal_id` for provider `facebook` |
| **Post ID** | `switchpost posts list --limit 5 --pretty` → `id` field |

Chain a discovered ID into the next command, for example with `jq`:

```bash
UUID=$(switchpost providers --pretty | jq -r '.providers[] | select(.provider=="linkedin-page") | .internal_id')
switchpost posts create "Update" --integrations "$UUID" --pretty
```

---

## CLI vs MCP Decision Tree

```
Does `switchpost <provider> <action>` exist in the recipes below?
  → YES: Use CLI (faster, simpler)
  → NO:  Use `switchpost mcp-call <tool_name> --args '<json>'`
```

### Toolsets (MCP budget)

`switchpost mcp` boots the `core` toolset only (cross-platform lifecycle tools plus the meta tools), so a session that only touches X does not pay the full tool-schema cost.

- `SWITCHPOST_TOOLSETS=x,reddit` (or `all`) selects sets at boot; `tools_enable` adds them at runtime.
- Call `tools_search` before `tools_enable` to find the set name for a provider.
- Size a set before enabling: `switchpost mcp-tools --toolset x --pretty`.
- `mcp-call`, staging v2 (`overrides`/`dry_run`/`force`/`next-slot`), group verbs and `utm_campaign` are in `references/quick-reference.md` § "MCP tool contracts (v11 — current)" (repo copy; not installed).

**Platforms with native CLI posting**:
- X/Twitter → `switchpost x post "text"`
- LinkedIn personal → `switchpost linkedin post "text"`
- LinkedIn page → `switchpost linkedin-page post <PAGE_ID> "text"`
- Reddit → `switchpost reddit post --title "t" --text "b" --target sub`
- Facebook → `switchpost facebook create <PAGE_ID> "message"`
- Bluesky → `switchpost bluesky post "text"`
- Mastodon → `switchpost mastodon post "text"`
- TikTok → `switchpost tiktok post "text"`
- Threads → `switchpost threads post <ACCOUNT_ID> "text"`
- Discord → `switchpost discord send <CHANNEL_ID> "text"`
- Slack → `switchpost slack send <CHANNEL_ID> "text"`
- Telegram Bot → `switchpost telegram-bot send <CHAT_ID> "text"`
- Telegram User → `switchpost telegram-user send <PEER> "text"`
- WhatsApp → `switchpost whatsapp send <PHONE> "text"`
- WordPress → `switchpost wordpress post "title" "content"`
- Dev.to → `switchpost devto post "content"`
- Hashnode → `switchpost hashnode post <PUB_ID> "title" "content"`
- Medium → `switchpost medium-blog post "content"`
- Skool → `switchpost skool post <GROUP_ID> "title" "content"`

**Platforms that need MCP for posting** (no native CLI post command):
- Instagram → `switchpost mcp-call ig_create_container` + `ig_publish_container` (3-step, see below)

---

## Step Zero: Discovery

**Before doing anything**, run these commands to understand what's available:

```bash
switchpost --help                    # See all top-level commands
switchpost <command> --help          # See args for a specific command
switchpost providers --pretty        # See what accounts are connected
switchpost doctor                    # Check health of all providers
```

If no providers are connected, run:
```bash
switchpost connect <provider> --pretty   # Shows the URL to visit
```

**Never guess command names.** Always check `--help` first.

---

## Where to look next

| Need | Open |
|------|------|
| Exact CLI or MCP command for one provider | `references/providers.md` |
| Read or engage commands for a platform (TikTok, Discord, GitHub, Google, votes, replies) | `references/platform-matrices.md` |
| Feed, analytics, media, carousel, comments, DMs, staging, automation, direct MCP calls | `references/recipes-extended.md` |
| Unknown flag or tool | `switchpost <command> --help` or `switchpost mcp-tools --pretty` |

---

## Task Recipes

### Post to one platform

```bash
# X/Twitter
switchpost x post "Hello world!" --pretty

# LinkedIn personal
switchpost linkedin post "New update!" --pretty

# LinkedIn company page
switchpost linkedin-page post <PAGE_ID> "New update!" --pretty

# Reddit
switchpost reddit post --title "My post" --text "Body text here" --target rust --pretty

# Instagram (requires media)
switchpost post "Check this out" --platforms instagram --media ./photo.jpg --first-comment "Link in bio!"
```

**Verify**: `switchpost posts list --limit 1 --pretty` — check the post appears with `state: "published"`.

### Post to multiple platforms at once

```bash
switchpost post "Shipping new features!" --platforms x,linkedin,bluesky --pretty
```

Content is auto-split to fit each platform's character limit. Threads are auto-numbered (1/N) for X, Bluesky, and Mastodon.

**Verify**: `switchpost posts list --limit 3 --pretty` — check all platform posts appear.

### Post a carousel (multiple images)

```bash
switchpost carousel "5 Tips for Devs" --integration <UUID> --media https://example.com/tip1.jpg,https://example.com/tip2.jpg --pretty
```

### Schedule a post

```bash
# Create and arm for a future time
switchpost posts create "Morning update" --integrations <UUID> --schedule "2026-07-02T09:00:00Z" --pretty

# Find the next available slot
switchpost posts find-slot --integration <UUID> --pretty

# List scheduled posts
switchpost posts list --limit 10 --pretty

# Cancel/reschedule
switchpost posts delete <POST_ID> --pretty
```

**Verify**: `switchpost posts get <POST_ID> --pretty` — check `state: "queued"` and `scheduled_at` matches.

### Read a feed

```bash
switchpost x timeline --count 5 --pretty    # X home timeline (flag is --count, not --limit)
```

Other platforms' feed and read commands are in `references/recipes-extended.md`.

## Error Recovery

| Error | Cause | Fix |
|-------|-------|-----|
| `No X/Twitter integration found` | X not connected | `switchpost connect x` → visit URL |
| `No Reddit integration found` | Reddit not connected | `switchpost connect reddit` → cookie auth |
| `No LinkedIn account connected` | LinkedIn not connected | `switchpost connect linkedin` → OAuth flow |
| `Facebook page not connected` | Wrong page ID | `switchpost providers --pretty` → find correct ID |
| `Instagram account not connected` | Wrong account ID | Use Instagram Business Account ID, not username |
| `Token expired` | OAuth token expired | User must re-authenticate via connect flow |
| `Rate limit exceeded` (HTTP 429) | Too many API calls | Wait, space out requests |
| `No virtual resource found` | LinkedIn page too new | Try a different page or wait |
| `API error: (#100)` | Invalid metric/permission | Check metric names; reconnect with proper scopes |
| `command not found: switchpost` | Binary not installed | Run install script or build from source |
| `connection refused` | Server not running | Run `switchpost serve` or check systemd status |
| `permission denied` | File permissions | Check `.switchpost/` directory permissions |
| `No integration found for provider` | Provider not connected | `switchpost connect <provider>` |
| `unknown flag` | Wrong CLI syntax | Run `switchpost <command> --help` to see correct flags |
| `Failed to parse` / `invalid value` | Wrong argument type | Check `--help` for expected types (IDs are strings, not numbers) |
| `Error: Serialize` / JSON error | Unexpected output format | Check if output is valid JSON; use `--pretty` for debugging |

**Always run `switchpost doctor` first** to check which providers are healthy.

---

## MCP Server Setup

For MCP clients (Claude Desktop, Cursor, etc.):

```json
{
  "mcpServers": {
    "switchpost": {
      "command": "switchpost",
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
