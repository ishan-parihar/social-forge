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
| **Integration UUID** | `social-forge providers --pretty` → `internal_id` field |
| **LinkedIn Page ID** | `social-forge linkedin-page list --pretty` → `page_id` field |
| **Instagram Account ID** | `social-forge providers --pretty` → `internal_id` for provider `instagram` |
| **Reddit Subreddit** | Use the name directly (e.g., `rust`, `javascript`) |
| **Facebook Page ID** | `social-forge providers --pretty` → `internal_id` for provider `facebook` |
| **Post ID** | `social-forge posts list --limit 5 --pretty` → `id` field |

Chain a discovered ID into the next command, for example with `jq`:

```bash
UUID=$(social-forge providers --pretty | jq -r '.providers[] | select(.provider=="linkedin-page") | .internal_id')
social-forge posts create "Update" --integrations "$UUID" --pretty
```

---

## CLI vs MCP Decision Tree

```
Does `social-forge <provider> <action>` exist in the recipes below?
  → YES: Use CLI (faster, simpler)
  → NO:  Use `social-forge mcp-call <tool_name> --args '<json>'`
```

### Toolsets (MCP budget)

`social-forge mcp` boots the `core` toolset only (cross-platform lifecycle tools plus the meta tools), so a session that only touches X does not pay the full tool-schema cost.

- `SOCIAL_FORGE_TOOLSETS=x,reddit` (or `all`) selects sets at boot; `tools_enable` adds them at runtime.
- Call `tools_search` before `tools_enable` to find the set name for a provider.
- Size a set before enabling: `social-forge mcp-tools --toolset x --pretty`.
- `mcp-call`, staging v2 (`overrides`/`dry_run`/`force`/`next-slot`), group verbs and `utm_campaign` are in `references/quick-reference.md` § "MCP tool contracts (v11 — current)" (repo copy; not installed).

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
- Telegram Bot → `social-forge telegram-bot send <CHAT_ID> "text"`
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

## Where to look next

| Need | Open |
|------|------|
| Exact CLI or MCP command for one provider | `references/providers.md` |
| Read or engage commands for a platform (TikTok, Discord, GitHub, Google, votes, replies) | `references/platform-matrices.md` |
| Feed, analytics, media, carousel, comments, DMs, staging, automation, direct MCP calls | `references/recipes-extended.md` |
| Unknown flag or tool | `social-forge <command> --help` or `social-forge mcp-tools --pretty` |

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

### Post a carousel (multiple images)

```bash
social-forge carousel "5 Tips for Devs" --integration <UUID> --media https://example.com/tip1.jpg,https://example.com/tip2.jpg --pretty
```

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
social-forge x timeline --count 5 --pretty    # X home timeline (flag is --count, not --limit)
```

Other platforms' feed and read commands are in `references/recipes-extended.md`.

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
