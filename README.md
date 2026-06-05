# Social Forge 🔥

**A high-performance social media orchestration engine with a triple-interface architecture: CLI, REST API, and MCP protocol.**

[![Rust](https://img.shields.io/badge/rust-1.85%2B-dea584?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![MCP](https://img.shields.io/badge/Protocol-MCP-purple.svg)](https://modelcontextprotocol.io/)

---

## What is this?

Social Forge is a single Rust binary that manages **21 social platforms** through three interfaces:

1. **CLI** — AI agents run `social-forge x post "hello"` directly from the terminal
2. **REST API** — SvelteKit dashboard for human operators
3. **MCP Server** — 130+ tools for Claude/Cursor-style AI integrations

One binary. One codebase. Zero runtime dependencies beyond PostgreSQL.

---

## Quick Start

> **Requirements**: [Rust](https://rustup.rs/) 1.85+, Node.js 20+, Docker (for Postgres)

```bash
git clone https://github.com/ishan-parihar/social-forge.git
cd social-forge
cp .env.example .env
```

### 1. Start PostgreSQL

```bash
docker compose up -d postgres
```

This starts Postgres on `localhost:5432` with user `social_forge` / database `social_forge` — matching the default in `.env.example`.

### 2. Build everything

```bash
# Rust binary
cargo build --release

# Frontend dashboard
cd frontend && npm install && npm run build && cd ..
```

### 3. Start the server

```bash
./target/release/social-forge serve
```

Open **https://localhost:6543** for the dashboard. Visit **https://localhost:6543/setup** to connect social accounts.

---

## Production Deployment

### Systemd + Docker (recommended — fastest iteration)

**Postgres** runs as a Docker container. **The app binary** runs directly via systemd (no Docker image rebuild needed after code changes).

```bash
git clone https://github.com/ishan-parihar/social-forge.git
cd social-forge
cp .env.example .env
# Edit .env with your DATABASE_URL and any API keys you need

# 1. Start Postgres
docker compose up -d postgres

# 2. Build everything
cargo build --release
cd frontend && pnpm install && pnpm build && cd ..

# 3. Install systemd service (one-time)
sudo cp target/release/social-forge /usr/local/bin/social-forge
sudo cp scripts/social-forge-start.sh /usr/local/bin/social-forge-start.sh
sudo cp scripts/social-forge.service /etc/systemd/system/social-forge.service
sudo systemctl daemon-reload
sudo systemctl enable social-forge --now
```

Open **https://localhost:6543** for the dashboard.

**Daily development — redeploy after code changes:**
```bash
make redeploy
# Or the one-liner:
# cargo build --release && sudo cp target/release/social-forge /usr/local/bin/social-forge && sudo systemctl restart social-forge
```

The binary is swapped in under a second. No Docker image rebuild required.

For HTTPS, put a reverse proxy (Caddy, Nginx) or tunnel (Cloudflare, ngrok) in front.

### From source on a VPS (faster builds, direct control)

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/ishan-parihar/social-forge.git
cd social-forge
cp .env.example .env
# Edit DATABASE_URL to point to your Postgres instance

cargo build --release
cd frontend && pnpm install && pnpm build && cd ..

# Install binary globally and set up systemd service
sudo install -m 755 target/release/social-forge /usr/local/bin/social-forge
sudo cp scripts/social-forge-start.sh /usr/local/bin/social-forge-start.sh
sudo cp scripts/social-forge.service /etc/systemd/system/social-forge.service
sudo systemctl daemon-reload
sudo systemctl enable social-forge --now
```

See the [Auto-start on boot](#auto-start-on-boot-systemd) section below for full systemd setup.

### Auto-start on boot (systemd)

> **This project uses a hybrid architecture:** Postgres runs via Docker (auto-restart), the app binary runs directly via systemd (fastest dev iteration, no Docker image rebuild).

```
System Boot
  ├── docker.service (enabled)
  │    └── postgres container (restart: unless-stopped, port 5433)
  └── social-forge.service (enabled)
       └── /usr/local/bin/social-forge serve  (pre-built binary, NOT built on boot)
```

**Prerequisites:**
- Docker (for Postgres) — enabled on boot: `sudo systemctl enable docker --now`
- The Rust binary pre-built at `target/release/social-forge`

**1. Copy the service files (one-time setup):**

```bash
sudo cp scripts/social-forge-start.sh /usr/local/bin/social-forge-start.sh
sudo cp scripts/social-forge.service /etc/systemd/system/social-forge.service
sudo cp target/release/social-forge /usr/local/bin/social-forge
sudo systemctl daemon-reload
sudo systemctl enable social-forge --now
```

**2. Daily development — redeploy after code changes:**

```bash
# One-liner: build → install → restart (takes ~1 second for restart)
cargo build --release && sudo install -m 755 target/release/social-forge /usr/local/bin/social-forge && sudo systemctl restart social-forge

# Or use the Makefile:
make redeploy

# Auto-watch (auto-rebuild on file changes):
make watch
```

The binary is swapped in under a second with zero downtime risk. No Docker image build required.

### CLI only (no server)

```bash
# Initialize user config directory
social-forge init

# Edit ~/.social-forge/.env with DATABASE_URL and credentials
# Then use CLI commands from any directory:
social-forge providers                    # List connected accounts
social-forge x timeline --count 5        # View X timeline
social-forge reddit browse rust          # Browse r/rust
social-forge linkedin profile            # View LinkedIn profile
```

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
│  │  ProviderRegistry → 21 providers (trait-based, async)    │   │
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
│   ├── mcp/                 # MCP server (130+ tools)
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
│   │   └── ...              # 21 providers total
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

## Development

### Architecture

```
System Boot
  ├── docker.service (enabled)
  │    └── postgres container (restart: unless-stopped, port 5433)
  └── social-forge.service (enabled)
       └── /usr/local/bin/social-forge serve  (pre-built binary, NOT built on boot)
```

- **Postgres**: Docker container with `restart: unless-stopped` — auto-starts on boot
- **App binary**: Pre-built at `/usr/local/bin/social-forge`, run via systemd
- **Frontend**: Built with `pnpm build`, served by the Rust binary

### Workflow

```bash
# Make code changes, then redeploy:
make redeploy
# = cargo build --release && sudo cp target/release/social-forge /usr/local/bin/social-forge && sudo systemctl restart social-forge

# Or with auto-watch (auto-rebuild + restart on every file change):
make watch       # requires: cargo install cargo-watch

# Run tests
cargo test --lib

# Run MCP integration tests
cargo test --test mcp_meta_audit

# Prepare sqlx offline cache (after schema changes)
cargo sqlx prepare
```

### Frontend-only development

```bash
cd frontend
pnpm dev          # HMR dev server on http://localhost:3000
```

---

## License

MIT — [Ishan Parihar](https://github.com/ishan-parihar)
