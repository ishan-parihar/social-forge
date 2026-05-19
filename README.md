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
│  (clap)      │  (axum :6543)    │   (rmcp, 130+ tools)          │
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

### Option 1: From source (recommended for development)

```bash
git clone https://github.com/ishan-parihar/social-forge.git
cd social-forge
cp .env.example .env

# Start PostgreSQL (or use your own)
docker compose up -d postgres

# Build
cargo build --release

# Build the frontend dashboard
cd frontend && npm install && npm run build && cd ..

# Start the server (serves API + dashboard on port 6543 with HTTPS)
./target/release/social-forge serve
```

Open `https://localhost:6543` for the dashboard. Visit `https://localhost:6543/setup` to connect social accounts via OAuth.

### Option 2: Docker

```bash
git clone https://github.com/ishan-parihar/social-forge.git
cd social-forge
cp .env.example .env
# Edit .env with your API keys
docker compose up -d
```

### Option 3: CLI only (no server)

```bash
# Initialize user config
social-forge init

# Edit ~/.social-forge/.env with DATABASE_URL and credentials
# Then use CLI commands from any directory:
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
- Managed via the onboarding dashboard at `/setup`
- Tokens encrypted at rest (AES-GCM, optional)

Priority resolution: `DB cookie tokens → Browser extraction → OAuth tokens → Env vars`

---

## Self-Hosting & Onboarding

### Environment Variables

The only required variable is `DATABASE_URL`. Everything else has sensible defaults:

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | *(required)* | PostgreSQL connection string |
| `APP_URL` | `https://localhost:6543` | Public URL of your instance. Used for OAuth redirect URIs. |
| `FRONTEND_URL` | Same as `APP_URL` | CORS allowed origin. Set separately only if frontend is on a different domain. |
| `JWT_SECRET` | Auto-generated | Secret for signing auth tokens. Set a strong value in production. |
| `TOKEN_ENCRYPTION_KEY` | *(optional)* | 64 hex chars. Encrypts OAuth tokens at rest (AES-GCM). |
| `FRONTEND_DIR` | `./frontend/build` | Path to the SvelteKit static build directory. |

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
