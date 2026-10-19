<!-- T2I HERO SPEC — Subject: a social media forge — a single anvil-like server core (Rust) with three spokes: CLI terminal, REST dashboard, and MCP protocol link — radiating post-flows to a ring of 25 platform logos (X, Reddit, LinkedIn, Instagram, Threads, YouTube, TikTok, Discord, Slack, Telegram, WhatsApp, Bluesky…). Composition: hub-and-spoke, concentric pulse rings. Palette: forge-fire orange #f97316 → deep charcoal #1c1917, ember gold #fbbf24 sparks, clean UI white text. Style: dark industrial flat vector, glowing embers, no readable logos, no text. 16:9. -->

# Social-Forge

<p align="center">
  <img src="./assets/readme/hero.svg" width="100%" alt="Social Forge: one Rust binary, 26 social platforms, three interfaces — CLI, REST, MCP">
</p>

**A high-performance social media orchestration engine with a triple-interface architecture: CLI, REST API, and MCP protocol.**

[![Rust](https://img.shields.io/badge/rust-1.85%2B-dea584?logo=rust)](https://www.rust-lang.org/)
<img src="https://img.shields.io/badge/Status-Active-brightgreen" alt="Status">
[![CI](https://github.com/ishan-parihar/social-forge/actions/workflows/ci.yml/badge.svg)](https://github.com/ishan-parihar/social-forge/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![MCP](https://img.shields.io/badge/Protocol-MCP-purple.svg)](https://modelcontextprotocol.io/)
[![Platforms](https://img.shields.io/badge/platforms-26-orange)](https://github.com/ishan-parihar/social-forge)
[![Tests](https://img.shields.io/badge/tests-lib%20+%20mcp--audit-green)](https://github.com/ishan-parihar/social-forge)

---

## What it is

Social Forge is a single Rust binary that manages **26 social platforms** (12 Tier-1 + 14 Tier-2, verified: `src/social/tier.rs`) through three interfaces:

1. **CLI** — 100+ commands for AI agents and terminal power users
2. **REST API** — SvelteKit dashboard for human operators
3. **MCP Server** — 389 tools across 44 modules for Claude/Cursor-style AI integrations

---

## Quick start

```bash
# Build (Rust 1.85+)
cargo build --release

# Configure (writes ~/.social-forge/.env)
./target/release/social-forge init

# Post now to X + Bluesky
social-forge post "Hello from Social-Forge!" --platforms x,bluesky

# Schedule for later (ISO8601)
social-forge post "Morning post" --platforms linkedin --schedule "2026-10-01T09:00:00Z"
```

---

## MCP Server (for agents)

```bash
social-forge mcp
```

**389 MCP tools** across 44 modules, all defined in `src/mcp/mod.rs` and dispatched to per-provider handler modules in `src/mcp/tools_*.rs`. Names are namespaced by platform:

| Namespace | Tools | Namespace | Tools |
|-----------|------:|-----------|------:|
| `goog_*` (Gmail/Calendar/Drive/YouTube data) | 28 | `tb_*` (Telegram Bot) | 10 |
| `x_*` | 27 | `di_*` (Discord) | 10 |
| `reddit_*` | 23 | `li_*` (LinkedIn) | 15 |
| `ig_*` (Instagram) | 21 | `lip_*` (LinkedIn Page) | 11 |
| `gh_*` (GitHub) | 18 | `fb_*` (Facebook) | 15 |
| `posts_*` (cross-platform) | 17 | `th_*` (Threads) | 9 |
| `wa_*` (WhatsApp) | 9 | `pi_*` (Pinterest) | 8 |
| `ias_*` (Instagram standalone) | 7 | `tu_*` (Telegram User) | 7 |
| `bs_*` (Bluesky) | 7 | `ms_*` (Mastodon) | 6 |
| `sk_*` (Skool) | 5 | `tt_*` (TikTok) | 4 |
| `sl_*` (Slack) | 4 | `wp_*` (WordPress) | 4 |
| `yt_*` (YouTube) | 2 | `md_*` / `dv_*` / `hn_*` (Medium / Dev.to / Hashnode) | 3 each |

The remaining 68 are cross-platform: `posts_*`, `analytics_*`, `feed_*`, `integrations_*`, `media_*`, `tag_*`, `notif_*`, `wh_*` (webhooks), `calendar_get`, `auth_status`, `setup_*`, and the automation rule tools.

`social-forge mcp` enumerates the live set; 389 is the number of `#[tool(…)]` definitions in `src/mcp/`.

---

## Features

| Feature | Details |
|---------|---------|
| **Multi-platform** | 26 platforms (12 Tier-1 + 14 Tier-2), one trait-based registry |
| **Scheduling** | Cron + one-time, timezone-aware |
| **Media** | Images, videos, GIFs, carousels |
| **Analytics** | Engagement, reach, follower growth |
| **Rate limiting** | Per-platform, auto-backoff, concurrency limit + circuit breaker |
| **MCP** | 389 tools across 44 modules for agent orchestration |

---

## Configuration

All config lives in `~/.social-forge/.env` (created by `social-forge init`).
See `.env.example` for the full documented list. Nothing is required — every
value has a default:

```bash
DATABASE_URL=sqlite://data/social-forge.db?mode=rwc
APP_PASSWORD=choose-a-strong-password   # single-user gate for the WebUI
APP_URL=http://localhost:6543           # public URL (OAuth redirects)
TOKEN_ENCRYPTION_KEY=<64 hex chars>     # AES-256-GCM for tokens at rest
```

---

## Commands

| Command | Description |
|---------|-------------|
| `social-forge post "text" --platforms x,bluesky` | Create (or schedule via `--schedule`) post |
| `social-forge posts schedule <id> <at>` | Schedule an existing draft |
| `social-forge analytics get <PROVIDER>` | Get analytics |
| `social-forge mcp` | Start MCP server (stdio) |
| `social-forge config set KEY VALUE` | Manage config |
| `social-forge audit` | Disabled integrations, `--platforms` prefix collisions, writer-lock posture |

Targeting, the single-writer rule, platform tiers, and the repository layout are in [docs/platforms-and-cli.md](docs/platforms-and-cli.md). Planning, audit, and decision records are indexed in [`docs/planning/README.md`](docs/planning/README.md); the full 2026-10-14 audit is [`docs/audit/AUDIT_2026-10-14.md`](docs/audit/AUDIT_2026-10-14.md) and the 2026-10-19 session self-reflection is [`docs/planning/SESSION_2026-10-19.md`](docs/planning/SESSION_2026-10-19.md).

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
│   (clap)      │  (axum :6543)    │   (rmcp, 389 tools)           │
├──────────────┴──────────────────┴───────────────────────────────┤
│                    Shared Business Logic                          │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  ProviderRegistry → 26 providers (12 T1 + 14 T2, async) │   │
│  │  Scheduler → Tokio background worker (30s poll)          │   │
│  │  Auth → JWT + Argon2 + OAuth2 + Cookie dual-path         │   │
│  │  Realtime → SSE broadcast (tokio::sync::broadcast)       │   │
│  └──────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│                    SQLite (single .db file)                      │
└─────────────────────────────────────────────────────────────────┘
```

## How it compares

| Capability | **Social Forge** | Buffer / Hootsuite | n8n / Make | Postiz |
|---|---|---|---|---|
| **AI-agent native** | ✅ CLI + MCP + REST all same engine | ❌ human dashboards | ⚠️ workflow only | ⚠️ some API |
| **Platforms** | 26, trait-based registry, tiered depth | ~6–10 | via nodes | ~10 |
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

This exposes 389 tools with full JSON Schema descriptions.

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

Environment variables, tunnel and reverse-proxy setup, onboarding, and OAuth redirect URIs are in [docs/deployment.md](docs/deployment.md).

## Technical Highlights

- **Language**: Rust (Edition 2021)
- **Web Framework**: Axum 0.8
- **Database**: SQLite via sqlx (compile-time checked queries)
- **MCP**: rmcp 1.6 with 389 tools (44 modules)
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

## Requirements

- Rust 1.85+ (backend), Node 20+ with pnpm (frontend)
- Platform API credentials
- No database server — SQLite is embedded (the `.db` file is created and migrated on first run)

---

## License

MIT — [Ishan Parihar](https://github.com/ishan-parihar)

---

Skill install and the Claude Code, Codex and OpenCode session hooks are in [docs/agent-integration.md](docs/agent-integration.md).

## ☕ Support & Sponsorship

If you find this project useful, consider supporting ongoing development:

[![Sponsor](https://img.shields.io/badge/Sponsor-GitHub%20Sponsors-ea4aaa?style=flat-square&logo=github)](https://github.com/sponsors/ishan-parihar)
[![Donate](https://img.shields.io/badge/Donate-Razorpay-3395FF?style=flat-square)](https://rzp.io/rzp/ishan-parihar)

Your support funds new features, releases, and infrastructure for the whole ecosystem.