---
name: social-forge-agent
description: >
  Use Social Forge to manage social media accounts via CLI or MCP. Trigger this skill whenever the user wants to post, read, search, schedule, analyze, or manage any social media — X/Twitter, Reddit, LinkedIn, Facebook, Instagram, Bluesky, Discord, Telegram, WhatsApp, GitHub, or any connected provider. Also trigger on "post to X", "check my LinkedIn", "browse Reddit", "manage my socials", "schedule a tweet", "what's on my feed", "get Instagram insights", "post to Facebook page", "send a Telegram message", "manage Discord", "check WhatsApp", or any social media operation. Covers all 190+ MCP tools and CLI commands.
---

# Social Forge — AI Agent Guide

Social Forge manages social media across 15+ platforms from one tool. Two interfaces:

- **CLI**: `social-forge <provider> <action> [args]` — quick single commands
- **MCP**: `social-forge mcp` — 190+ tools via JSON-RPC stdio (for programmatic/agent use)

All CLI output is JSON (stdout). Errors are JSON on stderr with exit code 1.

---

## Core Workflow

### 1. Always check connected providers first

```bash
social-forge providers --pretty
```

This shows every connected account with its provider, name, and ID. You need these IDs for provider-specific operations (e.g., `<PAGE_ID>` for Facebook, `<ACCOUNT_ID>` for Instagram). If a provider is missing, tell the user to connect it:

```bash
social-forge connect <provider> --pretty   # Shows the URL to visit
```

### 2. Read before you write

Explore a provider's data before posting. Use `--pretty` when reading output yourself:

```bash
social-forge x timeline --count 5 --pretty           # See what's on X
social-forge reddit browse rust --pretty              # Browse a subreddit
social-forge instagram posts <ACCOUNT_ID> --pretty    # Recent IG posts
social-forge linkedin-page list --pretty              # Your LinkedIn pages
social-forge facebook posts <PAGE_ID> --pretty        # FB page feed
```

### 3. Post and engage

Each provider has its own posting syntax. See the [Provider Reference](references/providers.md) for the full command catalog.

```bash
social-forge x post "Hello world!" --pretty
social-forge reddit post --title "My post" --text "Body" --target rust --pretty
social-forge linkedin-page post <PAGE_ID> "New update!" --pretty
social-forge facebook comment <POST_ID> "Great post!" --pretty
social-forge instagram comment <MEDIA_ID> "Love this!" --pretty
```

### 4. Import and browse across platforms

Pull posts from any provider into a unified local feed:

```bash
social-forge import x --count 20 --pretty    # Import recent X posts
social-forge feed --pretty                    # Browse everything together
social-forge feed --provider x --pretty      # Filter by provider
```

---

## Practical Workflows

### Cross-platform posting

Post the same content to multiple platforms:

1. Check providers: `social-forge providers --pretty`
2. Post to X: `social-forge x post "<content>" --pretty`
3. Post to LinkedIn Page: `social-forge linkedin-page post <PAGE_ID> "<content>" --pretty`
4. Post to Facebook: Use MCP `fb_create_post` with each page ID

### Content scheduling

Use MCP tools for scheduling:

1. `posts_create` — Create the post with provider, content, and targets
2. `posts_schedule` — Set the publish datetime
3. `posts_find_slot` — Find the next available time slot

### Monitoring and engagement

1. `social-forge x timeline --pretty` — Check what's trending
2. `social-forge x search "<topic>" --pretty` — Find relevant conversations
3. `social-forge x like <tweet_id>` / `social-forge x retweet <tweet_id>` — Engage
4. `social-forge reddit browse <subreddit> --pretty` — Check subreddits
5. `social-forge reddit comment <thing_id> "..." --pretty` — Reply

### Analytics check

```bash
social-forge instagram insights <ACCOUNT_ID> --metric reach,follower_count --pretty
social-forge linkedin-page analytics <PAGE_ID> --pretty
social-forge linkedin-page followers <PAGE_ID> --pretty
```

Instagram insights supports these metrics (comma-separated):
- **Day period**: `reach`, `follower_count`, `online_followers`
- **Lifetime**: `total_interactions`, `comments`, `shares`, `saves`, `likes`, `accounts_engaged`, `profile_views`, `website_clicks`

---

## Error Recovery

| Error | Cause | Fix |
|-------|-------|-----|
| `No X/Twitter integration found` | X not connected | Run `social-forge connect x` and have user visit the URL |
| `No Reddit integration found` | Reddit not connected | Run `social-forge connect reddit` — needs cookie auth |
| `No LinkedIn account connected` | LinkedIn personal not connected | Run `social-forge connect linkedin` — OAuth flow |
| `Facebook page not connected` | Specific page ID not in DB | Check `social-forge providers --pretty` for correct page IDs |
| `Instagram account not connected` | Wrong account ID | Check providers for the Instagram Business Account ID |
| `API error: (#100)` | Invalid metric or permission | Check metric names; may need to reconnect with proper scopes |
| `Token expired` | OAuth token expired | User needs to re-authenticate via the connect flow |
| `Rate limit exceeded` (HTTP 429) | Too many API calls | Wait and retry; space out requests |
| `No Facebook account connected` | Wrong page ID or no FB connected | Check `social-forge providers --pretty` for correct page IDs |
| `Instagram account not connected` | Wrong account ID | The account ID is the Instagram Business Account ID, not the username |
| `No virtual resource found` | LinkedIn page has no share stats | Page may be too new or lack post history; try a different page |

When a provider isn't connected, always run `social-forge connect <provider> --pretty` first to get the connection URL, then guide the user through it.

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

See [Provider Reference](references/providers.md) for the complete list of MCP tools per provider.

---

## Authentication Priority

1. DB-stored cookie tokens (richest access — enables voting, moderation, GraphQL)
2. Browser cookie extraction (auto-detected from Chrome/Brave/Firefox/Zen)
3. OAuth tokens from DB (standard API access)
4. Environment variables (fallback: `X_AUTH_TOKEN`, `X_CT0`, etc.)

Cookie auth gives the fullest access. OAuth is the fallback for platforms without cookie support.
