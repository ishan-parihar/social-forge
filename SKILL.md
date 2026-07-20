---
name: social-forge
description: >
  Post to 21 social platforms from a single CLI. Manage X, Reddit, LinkedIn,
  Instagram, Bluesky, Mastodon, YouTube, TikTok, and more with auto
  content-splitting, scheduling, and analytics.
---

# Social Forge Skill

Post to 21 social platforms from a single CLI with auto content-splitting.

<!-- Static skill — regenerate: social-forge --help -->
<!-- Install: npx skills add <owner/social-forge> --skill social-forge -->
<!-- CI check: diff <(social-forge --help) SKILL.md && exit 1 -->
<!-- Install: npx skills add <owner/social-forge> --skill social-forge -->

## Quick Start

```bash
# Post to all connected platforms
social-forge post "Hello from Social Forge!" --platforms x,bluesky,linkedin

# Stage a post with preview
social-forge stage "My post content" --preview

# Check provider health
social-forge doctor

# Import browser cookies for X/Reddit
social-forge connect-all
```

## Key Commands

| Command | Description |
|---------|-------------|
| `social-forge post <text>` | Post to any platform |
| `social-forge stage <text>` | Stage with auto content-splitting |
| `social-forge providers` | List connected providers |
| `social-forge doctor` | Health-check all providers |
| `social-forge connect <provider>` | Connect a new provider |
| `social-forge feed` | View unified feed |
| `social-forge posts list` | List scheduled/published posts |
| `social-forge serve` | Start HTTP dashboard |
| `social-forge mcp` | Start MCP server for AI agents |

## Supported Platforms

x, reddit, linkedin, linkedin-page, facebook, instagram, youtube, bluesky, mastodon, tiktok, threads, discord, slack, telegram-bot, telegram-user, whatsapp, pinterest, github, wordpress, hashnode, medium, devto, skool

## Configuration

1. Run `social-forge init` to create config
2. Run `social-forge connect <provider>` for each platform
3. Or set env vars in `~/.social-forge/.env`
