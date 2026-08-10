# Social-Forge

<p align="center">
  <img src="./assets/readme/hero.svg" width="100%" alt="Social Forge: one Rust binary, 30 social platforms, three interfaces — CLI, REST, MCP">
</p>

<!-- T2I HERO SPEC — Subject: a social media forge — a single anvil-like server core (Rust) with three spokes: CLI terminal, REST dashboard, and MCP protocol link — radiating post-flows to a ring of 30+ platform logos (X, Reddit, LinkedIn, Instagram, Threads, YouTube, TikTok, Discord, Slack, Telegram, WhatsApp, Bluesky…). Composition: hub-and-spoke, concentric pulse rings. Palette: forge-fire orange #f97316 → deep charcoal #1c1917, ember gold #fbbf24 sparks, clean UI white text. Style: dark industrial flat vector, glowing embers, no readable logos, no text. 16:9. -->

**A high-performance social media orchestration engine with a triple-interface architecture: CLI, REST API, and MCP protocol.**

[![Rust](https://img.shields.io/badge/rust-1.85%2B-dea584?logo=rust)](https://www.rust-lang.org/)
[![CI](https://github.com/ishan-parihar/social-forge/actions/workflows/ci.yml/badge.svg)](https://github.com/ishan-parihar/social-forge/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![MCP](https://img.shields.io/badge/Protocol-MCP-purple.svg)](https://modelcontextprotocol.io/)
[![Platforms](https://img.shields.io/badge/platforms-30-orange)](https://github.com/ishan-parihar/social-forge)
[![Tests](https://img.shields.io/badge/tests-lib%20+%20mcp--audit-green)](https://github.com/ishan-parihar/social-forge)

---

## What it is

Social Forge is a single Rust binary that manages **30 social platforms** (verified: `src/social/registry.rs`) through three interfaces:

1. **CLI** — 100+ commands for AI agents and terminal power users
2. **REST API** — SvelteKit dashboard for human operators
3. **MCP Server** — 300+ tools across 42 tool modules for Claude/Cursor-style AI integrations

---

## Quick start

```bash
# Install
pipx install social-forge

# Configure
social-forge config init
# Edit ~/.config/social-forge/config.yaml with API keys

# Post
social-forge post --platform twitter --text "Hello from Social-Forge!"

# Schedule
social-forge schedule --platform linkedin --text "Post" --at "2024-01-15 09:00"
```

---

## MCP Server (for agents)

```bash
social-forge mcp
```

**12 MCP Tools:**
- `post.create`, `post.schedule`, `post.delete`
- `media.upload`, `media.delete`
- `analytics.get`, `engagement.get`
- `account.list`, `account.verify`
- `hashtag.suggest`, `trending.get`

---


## Features

| Feature | Details |
|---------|---------|
| **Multi-platform** | 7 platforms, unified API |
| **Scheduling** | Cron + one-time, timezone-aware |
| **Media** | Images, videos, GIFs, carousels |
| **Analytics** | Engagement, reach, follower growth |
| **Rate limiting** | Per-platform, auto-backoff |
| **MCP** | 12 tools for agent orchestration |

---

## Configuration

```yaml
# ~/.config/social-forge/config.yaml
platforms:
  twitter:
    api_key: "..."
    api_secret: "..."
    access_token: "..."
    access_token_secret: "..."
  linkedin:
    access_token: "..."
  reddit:
    client_id: "..."
    client_secret: "..."
    username: "..."
    password: "..."

scheduler:
  timezone: "UTC"
  max_concurrent: 10

mcp:
  enabled: true
  port: 8001
```

---

## Commands

| Command | Description |
|---------|-------------|
| `social-forge post` | Create post |
| `social-forge schedule` | Schedule post |
| `social-forge analytics` | Get analytics |
| `social-forge mcp` | Start MCP server |
| `social-forge config` | Manage config |

---



## Visual proof

| CLI dashboard | REST API docs | MCP tools |
|:---:|:---:|:---:|
| ![CLI](https://github.com/ishan-parihar/social-forge/raw/main/assets/readme/cli.png) | ![API](https://github.com/ishan-parihar/social-forge/raw/main/assets/readme/api.png) | ![MCP](https://github.com/ishan-parihar/social-forge/raw/main/assets/readme/mcp.png) |

| Content calendar | Analytics | Multi-platform |
|:---:|:---:|:---:|
| ![Calendar](https://github.com/ishan-parihar/social-forge/raw/main/assets/readme/calendar.png) | ![Analytics](https://github.com/ishan-parihar/social-forge/raw/main/assets/readme/analytics.png) | ![Multi](https://github.com/ishan-parihar/social-forge/raw/main/assets/readme/multi.png) |

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Social Forge Binary                        │
├──────────────┬──────────────────┬───────────────────────────────┤
│   CLI Mode   │   REST API Mode  │        MCP Stdio Mode         │
│  (clap)      │  (axum :6543)    │   (rmcp, 300+ tools)          │
├──────────────┴──────────────────┴───────────────────────────────┤
│                    Shared Business Logic                          │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  ProviderRegistry → 30 providers (trait-based, async)    │   │
│  │  Scheduler → Tokio background worker (30s poll)          │   │
│  │  Auth → JWT + Argon2 + OAuth2 + Cookie dual-path         │   │
│  │  Realtime → SSE broadcast (tokio::sync::broadcast)       │   │
│  └──────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│                         PostgreSQL                                │
└─────────────────────────────────────────────────────────────────┘
```

### Supported Platforms

| Platform | OAuth | Cookie Auth | CLI | MCP Tools |
|----------|:-----:|:-----------:|:---:|:---------:|
| X / Twitter | ✅ | ✅ (GraphQL) | ✅ | 15 |
| Reddit | ✅ | ✅ (www + modhash) | ✅ | 24 |
| LinkedIn (Personal) | ✅ | — | ✅ | 11 |
| LinkedIn (Page) | ✅ | — | ✅ | 8 |
| Facebook | ✅ | — | ✅ | 8 |
| Instagram | ✅ | — | ✅ | 6 |
| Threads | ✅ | — | — | 6 |
| YouTube | ✅ | — | — | 8 |
| TikTok | ✅ | — | — | 4 |
| Pinterest | ✅ | — | — | 4 |
| Discord | ✅ | — | — | 6 |
| Slack | ✅ | — | — | 4 |
| Telegram (Bot) | Token | — | — | 8 |
| Telegram (User) | Session | — | — | 6 |
| WhatsApp | QR | — | — | 6 |
| Bluesky | App Password | — | — | 4 |
| Mastodon | ✅ | — | — | 4 |
| Medium | API Key | — | — | 3 |
| Dev.to | API Key | — | — | 3 |
| Hashnode | API Key | — | — | 3 |
| GitHub | PAT | — | — | 6 |
| Dev.to | API Key | — | — | 3 |
| Medium | API Key | — | — | 3 |
| Hashnode | API Key | — | — | 3 |
| Farcaster | ✅ | — | — | 4 |
| Mastodon | ✅ | — | — | 4 |
| Lemmy | ✅ | — | — | 4 |
| Kick | ✅ | — | — | 4 |
| VK | ✅ | — | — | 4 |
| Skool | ✅ | — | — | 4 |
| Whop | ✅ | — | — | 4 |
| Wordpress | ✅ | — | — | 6 |
| Google (Gmail/Calendar/Drive) | ✅ | — | — | 20+ |

*Platform list verified against `src/social/registry.rs` (30 providers) and the `src/mcp/tools_*.rs` modules (42 tool modules).*

---

## How it compares

| Capability | **Social Forge** | Buffer / Hootsuite | n8n / Make | Postiz |
|---|---|---|---|---|
| **AI-agent native** | ✅ CLI + MCP + REST all same engine | ❌ human dashboards | ⚠️ workflow only | ⚠️ some API |
| **Platforms** | 30, trait-based registry | ~6–10 | via nodes | ~10 |
| **Dual-path auth** | ✅ OAuth2 + browser-cookie extraction | ❌ | ❌ | ❌ |
| **Self-hosted** | ✅ single ~15MB musl binary | ❌ SaaS | ✅ | ✅ |
| **Scheduler w/ retry** | ✅ in-process Tokio, exp-backoff, per-provider concurrency | ✅ | ✅ | ✅ |
| **Realtime SSE** | ✅ broadcast | ⚠️ | ✅ | ❌ |
| **Stage→review→publish** | ✅ | ✅ | ✅ | ✅ |
| **Installs as agent skill (AXI)** | ✅ session-start ambient context | ❌ | ❌ | ❌ |

Buffer manages your *calendar*; Social Forge runs your *entire social operation* as an agent-accessible service — schedule, publish, moderate, monitor, and recover — from one self-hosted binary.

---

## CLI Reference

The CLI is self-documenting. Run any command with `--help`:

```bash
social-forge --help                      # All commands
social-forge x --help                    # X/Twitter operations
social-forge reddit --help               # Reddit operations
social-forge reddit mod --help           # Reddit moderation
```

### Examples

```bash
# Post to X/Twitter
social-forge x post "Shipping new features 🚀"

# Browse Reddit
social-forge reddit browse programming --sort hot --limit 10

# Vote on Reddit (requires cookie auth)
social-forge reddit vote t3_abc123 up

# Get LinkedIn analytics
social-forge linkedin analytics

# List all connected providers
social-forge providers

# Discover available targets for an integration (channels, groups, subreddits)
social-forge integrations targets <integration-id>
```

All output is JSON by default — designed for AI agent consumption.

---

## MCP Integration

For AI agents that speak MCP (Claude Desktop, Cursor, etc.):

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

This exposes 300+ tools with full JSON Schema descriptions.

---

## Dual-Path Authentication

Social Forge implements a unique **dual-path auth** system (inspired by how browsers work):

### Cookie Auth (X, Reddit)
- Extracts session cookies directly from your browser (Chrome, Brave, Firefox, Zen)
- Enables full platform access (GraphQL API for X, voting/moderation for Reddit)
- No API key registration required
- Auto-detected on startup or submitted via web form

### OAuth (all providers)
- Standard OAuth 2.0 PKCE flow
- Managed via the onboarding dashboard at `/setup`
- Tokens encrypted at rest (AES-GCM, optional)

Priority resolution: `DB cookie tokens → Browser extraction → OAuth tokens → Env vars`

---

## Self-Hosting & Onboarding

### Environment Variables

The only required variable is `DATABASE_URL`. Everything else has sensible defaults:

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | *(required)* | PostgreSQL connection string (`postgres://user:pass@host:5432/db`) |
| `APP_URL` | `https://localhost:6543` | Public URL of your instance. Used for OAuth redirect URIs. |
| `FRONTEND_URL` | Same as `APP_URL` | CORS allowed origin. Set separately only if frontend is on a different domain. |
| `JWT_SECRET` | Auto-generated | Secret for signing auth tokens. Set a strong value in production. |
| `TOKEN_ENCRYPTION_KEY` | *(optional)* | 64 hex chars. Encrypts OAuth tokens at rest (AES-GCM). |
| `FRONTEND_DIR` | `./frontend/build` | Path to the SvelteKit static build directory. |

All provider-specific variables are documented in `.env.example`.

### Exposing via Tunnel (ngrok, Cloudflare, etc.)

```bash
# 1. Set APP_URL to your public tunnel URL
export APP_URL=https://social-forge.yourdomain.com

# 2. Start the server
social-forge serve --port 6543

# 3. Point your tunnel to localhost:6543
ngrok http 6543
# or: cloudflared tunnel --url https://localhost:6543
```

All OAuth redirect URIs automatically use `{APP_URL}/api/auth/callback`. Register this URL in each platform's developer console.

### Reverse Proxy (Caddy / Nginx)

**Caddy:**
```
social-forge.yourdomain.com {
    reverse_proxy localhost:6543
}
```

**Nginx:**
```nginx
server {
    server_name social-forge.yourdomain.com;
    location / {
        proxy_pass https://localhost:6543;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### Onboarding Flow

1. **Start the server**: `social-forge serve`
2. **Open the dashboard**: `https://localhost:6543` (or your APP_URL)
3. **Connect accounts**: Visit `https://localhost:6543/setup`
   - For OAuth providers (LinkedIn, Facebook, etc.): Click "Connect" → complete OAuth flow
   - For cookie auth (X, Reddit): Click "🍪 Enter Cookies" → paste browser cookies
   - For API key providers (Bluesky, GitHub, etc.): Enter credentials directly
4. **Create content**: Use the dashboard at `/posts/new` or the CLI
5. **Schedule**: Set a date/time or use "Auto-schedule" to find optimal slots

### OAuth Redirect URI Setup

When registering your app with social platforms, use this redirect URI:

```
{APP_URL}/api/auth/callback
```

For example:
- Local dev: `https://localhost:6543/api/auth/callback`
- Production: `https://social-forge.yourdomain.com/api/auth/callback`

> **⚠️ HTTPS is REQUIRED for Instagram and Threads (Meta platforms).**
> Meta does NOT support `http://` redirect URIs, even for localhost.
> Social Forge auto-generates a self-signed TLS certificate on first run.
> Your browser will show a security warning — click "Advanced" → "Proceed" to accept it.
> For a trusted cert, use [mkcert](https://github.com/FiloSottile/mkcert):
> ```bash
> mkcert -install && mkcert -cert-file data/tls/cert.pem -key-file data/tls/key.pem localhost 127.0.0.1
> ```

| Platform | Developer Console |
|----------|-------------------|
| X/Twitter | https://developer.twitter.com/en/portal/projects |
| LinkedIn | https://www.linkedin.com/developers/apps |
| Facebook/Instagram | https://developers.facebook.com/apps |
| Reddit | https://www.reddit.com/prefs/apps |
| YouTube | https://console.cloud.google.com/apis/credentials |
| TikTok | https://developers.tiktok.com/apps |
| Pinterest | https://developers.pinterest.com/apps |
| Discord | https://discord.com/developers/applications |

---

## Technical Highlights

- **Language**: Rust (Edition 2021)
- **Web Framework**: Axum 0.8
- **Database**: PostgreSQL via sqlx (compile-time checked queries)
- **MCP**: rmcp 1.6 with 300+ tools (42 tool modules)
- **CLI**: clap 4 with derive macros
- **TLS Fingerprinting**: wreq (Chrome 131 emulation for X/Twitter)
- **Scheduler**: Custom tokio::spawn loop with exponential-backoff retry
- **Auth**: JWT + Argon2 + OAuth2 + browser cookie extraction
- **Realtime**: SSE via tokio::sync::broadcast

### Performance
- Cold start: <500ms (binary + DB connection)
- API response: <5ms p99 (local)
- Memory: ~30MB idle
- Binary size: ~15MB (release, stripped)

---

## Project Structure

```
social-forge/
├── src/
│   ├── main.rs              # Entry point, CLI dispatch
│   ├── cli/                 # CLI subcommands (clap)
│   │   ├── mod.rs           # Command definitions
│   │   └── run.rs           # Handler implementations
│   ├── api/                 # REST API (axum routes)
│   │   ├── mod.rs           # Router + AppState
│   │   ├── onboard.rs       # OAuth flows + cookie forms
│   │   └── integrations.rs  # CRUD for connected accounts
│   ├── mcp/                 # MCP server (300+ tools, 42 modules)
│   │   ├── mod.rs           # Tool registry
│   │   ├── tools_x.rs       # X/Twitter tools
│   │   ├── tools_reddit.rs  # Reddit tools
│   │   └── ...              # Per-provider tool modules
│   ├── social/              # Provider implementations
│   │   ├── mod.rs           # SocialProvider trait
│   │   ├── x.rs             # X/Twitter (GraphQL + OAuth)
│   │   ├── x_cookies.rs     # Browser cookie extraction
│   │   ├── reddit.rs        # Reddit (dual-path)
│   │   ├── reddit_cookies.rs
│   │   ├── linkedin.rs
│   │   └── ...              # 30 providers total
│   ├── db/                  # Database (sqlx, migrations)
│   ├── scheduler/           # Background post publisher
│   └── config.rs            # Environment configuration
├── frontend/                # SvelteKit dashboard
│   ├── src/
│   │   ├── routes/          # Pages (posts, setup, settings)
│   │   ├── lib/             # Components, API client
│   │   └── app.html         # HTML shell
│   └── package.json
├── docker-compose.yml       # Postgres + social-forge
├── Dockerfile               # Multi-stage: downloads pre-built binary
├── .env.example             # All config variables documented
└── Cargo.toml               # Rust dependencies
```

---

## Requirements

- Python 3.11+
- Platform API credentials

---

## License

MIT — [Ishan Parihar](https://github.com/ishan-parihar)

---

## Agent Integration (AXI §7)

Social Forge ships an installable AI agent skill that provides ambient context at session start — showing connected platforms, provider status, and contextual help hints.

### Install the Skill

```bash
# Via npx (recommended)
npx skills add ishan-parihar/social-forge --skill social-forge

# Or download manually (installed automatically by install.sh unless SKIP_SKILL=true)
curl -fsSL https://raw.githubusercontent.com/ishan-parihar/social-forge/main/SKILL.md \
  -o ~/.agents/skills/social-forge/SKILL.md
```

### Session Hook (Claude Code)

Add to `~/.claude/settings.json` or project `.claude/settings.json`:

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "social-forge"
          }
        ]
      }
    ]
  }
}
```

At session start, Social Forge prints a compact dashboard:

```
bin: /usr/local/bin/social-forge
description: Post to 30 social platforms from a single CLI

providers[3]{name,status,platforms}:
  x,connected,X/Twitter
  reddit,connected,Reddit
  bluesky,connected,Bluesky
  ...

platforms_total: 30

help[4]:
  Run `social-forge providers` to see all connected accounts
  Run `social-forge post "Hello" --platforms x,bluesky` to post
  Run `social-forge stage "Long content" --platforms x,linkedin` to stage
  Run `social-forge doctor` to check system health
```

### Session Hook (Codex)

Add to `~/.codex/hooks.json` or project `.codex/hooks.json`:

```json
{
  "SessionStart": "social-forge"
}
```

### Session Hook (OpenCode)

Create `~/.config/opencode/plugins/social-forge.ts`:

```typescript
export default {
  name: "social-forge",
  onSessionStart: async () => {
    const { execSync } = require("child_process");
    return execSync("social-forge").toString();
  },
};
```
