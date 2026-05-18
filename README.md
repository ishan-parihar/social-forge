# Social Forge 🔥

**A high-performance social media orchestration engine with a triple-interface architecture: CLI, REST API, and MCP protocol.**

[![Rust](https://img.shields.io/badge/rust-1.85%2B-dea584?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![MCP](https://img.shields.io/badge/Protocol-MCP-purple.svg)](https://modelcontextprotocol.io/)

---

## What is this?

Social Forge is a single Rust binary that manages **16 social platforms** through three interfaces:

1. **CLI** — AI agents run `social-forge x post "hello"` directly from the terminal
2. **REST API** — SvelteKit dashboard for human operators
3. **MCP Server** — 130+ tools for Claude/Cursor-style AI integrations

One binary. One codebase. Zero runtime dependencies beyond PostgreSQL.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Social Forge Binary                        │
├──────────────┬──────────────────┬───────────────────────────────┤
│   CLI Mode   │   REST API Mode  │        MCP Stdio Mode         │
│  (clap)      │  (axum :3444)    │   (rmcp, 130+ tools)          │
├──────────────┴──────────────────┴───────────────────────────────┤
│                    Shared Business Logic                          │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  ProviderRegistry → 16 providers (trait-based, async)    │   │
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

---

## Quick Start

### Option 1: Docker (recommended)

```bash
git clone https://github.com/ishan-parihar/social-forge.git
cd social-forge
cp .env.example .env
# Edit .env with your API keys
docker compose up -d
```

The server starts at `http://localhost:3444`. Visit `/` for the onboarding dashboard.

### Option 2: From source

```bash
git clone https://github.com/ishan-parihar/social-forge.git
cd social-forge
cp .env.example .env

# Start PostgreSQL
docker compose up -d postgres

# Build and run
cargo build --release
./target/release/social-forge serve
```

### Option 3: CLI only (no server)

```bash
# After building, use CLI commands directly:
social-forge providers                    # List connected accounts
social-forge x timeline --count 5        # View X timeline
social-forge reddit browse rust          # Browse r/rust
social-forge linkedin profile            # View LinkedIn profile
```

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

This exposes 130+ tools with full JSON Schema descriptions.

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
- Managed via the onboarding dashboard at `/`
- Tokens encrypted at rest (AES-GCM, optional)

Priority resolution: `DB cookie tokens → Browser extraction → OAuth tokens → Env vars`

---

## Technical Highlights

- **Language**: Rust (Edition 2021)
- **Web Framework**: Axum 0.8
- **Database**: PostgreSQL via sqlx (compile-time checked queries)
- **MCP**: rmcp 1.6 with 130+ tools
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
src/
├── main.rs              # Entry point, CLI dispatch
├── cli/                 # CLI subcommands (clap)
│   ├── mod.rs           # Command definitions
│   └── run.rs           # Handler implementations
├── api/                 # REST API (axum routes)
│   ├── mod.rs           # Router + AppState
│   ├── onboard.rs       # OAuth flows + cookie forms
│   └── integrations.rs  # CRUD for connected accounts
├── mcp/                 # MCP server (130+ tools)
│   ├── mod.rs           # Tool registry
│   ├── tools_x.rs       # X/Twitter tools
│   ├── tools_reddit.rs  # Reddit tools
│   └── ...              # Per-provider tool modules
├── social/              # Provider implementations
│   ├── mod.rs           # SocialProvider trait
│   ├── x.rs             # X/Twitter (GraphQL + OAuth)
│   ├── x_cookies.rs     # Browser cookie extraction
│   ├── reddit.rs        # Reddit (dual-path)
│   ├── reddit_cookies.rs
│   ├── linkedin.rs
│   └── ...              # 16 providers total
├── db/                  # Database (sqlx, migrations)
├── scheduler/           # Background post publisher
└── config.rs            # Environment configuration
```

---

## Development

```bash
# Run in development mode (auto-reload)
cargo watch -x run -- serve

# Run tests
cargo test --lib

# Run MCP integration tests
cargo test --test mcp_meta_audit

# Prepare sqlx offline cache (after schema changes)
cargo sqlx prepare
```

---

## License

MIT — [Ishan Parihar](https://github.com/ishan-parihar)
