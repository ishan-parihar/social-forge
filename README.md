# Postiz Rust — Social Media Scheduling Platform

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
![Rust](https://img.shields.io/badge/rust-1.85%2B-dea584?logo=rust)
[![CI](https://github.com/ishanpm/postiz-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/ishanpm/postiz-rust/actions/workflows/ci.yml)
[![Docker](https://img.shields.io/badge/docker-multi--arch-blue?logo=docker)](Dockerfile)

**Postiz Rust** is a dual-interface social media scheduling engine written in Rust. It exposes the same business logic through two protocols simultaneously:

- **REST API** (axum 0.8) — consumed by human users via a SvelteKit frontend dashboard
- **MCP** (rmcp / Model Context Protocol) — consumed by AI agents for programmatic scheduling and oversight

An in-process tokio scheduler polls the database every 30 seconds, publishes due posts across 5 social providers (X/Twitter, LinkedIn, Bluesky, Facebook, Instagram), and emits real-time events via Server-Sent Events.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         postiz-rust                              │
│                                                                  │
│  ┌──────────────┐    ┌──────────────────┐                       │
│  │  Human Users  │    │   AI Agents      │                       │
│  │  (SvelteKit)  │    │  (Claude, etc.)  │                       │
│  └──────┬───────┘    └────────┬─────────┘                       │
│         │                     │                                  │
│  ┌──────▼─────────────────────▼─────────┐                       │
│  │         axum 0.8 HTTP Router         │                       │
│  │                                      │                       │
│  │  ┌────────────────┐  ┌────────────┐  │                       │
│  │  │  REST API       │  │  MCP SSE   │  │  Port 3000           │
│  │  │  /api/auth/*    │  │  /mcp/sse  │  │  Port 3001 (MCP)     │
│  │  │  /api/posts/*   │  │            │  │                      │
│  │  │  /api/accounts/*│  │  rmcp      │  │                      │
│  │  │  /api/media/*   │  │  stdio     │  │                      │
│  │  │  /api/events    │  │  transport │  │                      │
│  │  └────────┬───────┘  └──────┬─────┘  │                       │
│  └───────────┼──────────────────┼────────┘                       │
│              │                  │                                │
│  ┌───────────▼──────────────────▼────────┐                       │
│  │          AppState (Arc<RwLock<...>>)  │                       │
│  │                                       │                       │
│  │  ┌──────────┐  ┌──────────────────┐   │                       │
│  │  │ Provider  │  │   Scheduler      │   │                       │
│  │  │ Registry  │  │   (tokio::spawn) │   │                       │
│  │  │           │  │   30s poll +     │   │                       │
│  │  │ X/Twitter │  │   exp. backoff   │   │                       │
│  │  │ LinkedIn  │  │                  │   │                       │
│  │  │ Bluesky   │  │   ┌──────────┐   │   │                       │
│  │  │ Facebook  │  │   │ Broadcast │   │   │                       │
│  │  │ Instagram │  │   │  Channel  │   │   │                       │
│  │  └──────────┘  │   └──────────┘   │   │                       │
│  └────────────────────────────────────┘   │                       │
│                                           │                       │
│  ┌────────────────────────────────────┐   │                       │
│  │          PostgreSQL (sqlx)         │   │                       │
│  │  users | accounts | posts | media  │   │                       │
│  │  scheduled_posts                   │   │                       │
│  └────────────────────────────────────┘   │                       │
└─────────────────────────────────────────────────────────────────┘
```

### Dual-Interface Design

| Interface     | Protocol                   | Audience                | Port            |
| ------------- | -------------------------- | ----------------------- | --------------- |
| **REST API**  | HTTP/JSON (axum)           | Human users (SvelteKit) | `3000`          |
| **MCP SSE**   | Server-Sent Events (rmcp)  | AI agents               | `3000 /mcp/sse` |
| **MCP Stdio** | JSON-RPC over stdio (rmcp) | AI agents (subprocess)  | —               |

Both interfaces share the same `AppState` — no code duplication between REST and MCP paths. Every social provider implements a single `SocialProvider` trait used by both.

---

## Features

- **Dual-Interface Architecture** — REST for humans, MCP for AI agents, one codebase
- **5 Social Providers** — X/Twitter, LinkedIn, Bluesky, Facebook, Instagram via `SocialProvider` trait
- **Pluggable Provider System** — `ProviderRegistry` with `#[async_trait]` for adding new networks
- **In-Process Scheduler** — tokio `spawn` + 30s interval + exponential-backoff retry + token refresh
- **Real-Time Events** — SSE via `tokio::sync::broadcast` for post status, errors, schedule updates
- **JWT Authentication** — Bearer tokens, Argon2 password hashing, `Extension<UserId>` middleware
- **Multi-Account Profiles** — per-user profile system for managing multiple social accounts
- **Media Upload** — Multipart upload API with metadata extraction
- **Multi-Arch Docker** — Single Dockerfile builds for `linux/amd64` and `linux/arm64` via musl
- **12-Factor Config** — Environment-driven with `dotenvy` for local dev

### Size & Performance

| Metric       | Postiz Rust       | Python Equivalent | Node.js Equivalent |
| ------------ | ----------------- | ----------------- | ------------------ |
| Binary Size  | ~12 MB (stripped) | —                 | —                  |
| Docker Image | ~15 MB            | ~150–300 MB       | ~200–400 MB        |
| Idle RSS     | ~4 MB             | ~30–60 MB         | ~25–50 MB          |
| Cold Start   | < 100 ms          | ~2–8 s            | ~500 ms–2 s        |
| Throughput   | ~10k req/s (est.) | ~1–2k req/s       | ~5–10k req/s       |

---

## API Reference

### REST Endpoints

| Method   | Path                       | Auth | Description                             |
| -------- | -------------------------- | ---- | --------------------------------------- |
| `POST`   | `/api/auth/register`       | —    | Register user (Argon2 hash)             |
| `POST`   | `/api/auth/login`          | —    | Login, returns JWT                      |
| `GET`    | `/api/auth/me`             | JWT  | Current user info                       |
| `GET`    | `/api/posts`               | JWT  | List scheduled posts                    |
| `POST`   | `/api/posts`               | JWT  | Create scheduled post                   |
| `GET`    | `/api/posts/:id`           | JWT  | Get post details                        |
| `PUT`    | `/api/posts/:id`           | JWT  | Update scheduled post                   |
| `DELETE` | `/api/posts/:id`           | JWT  | Cancel scheduled post                   |
| `GET`    | `/api/accounts`            | JWT  | List connected social accounts          |
| `POST`   | `/api/accounts`            | JWT  | Connect social account (OAuth callback) |
| `DELETE` | `/api/accounts/:id`        | JWT  | Disconnect social account               |
| `POST`   | `/api/profiles`            | JWT  | Create multi-account profile            |
| `GET`    | `/api/profiles`            | JWT  | List profiles                           |
| `PUT`    | `/api/profiles/:id/active` | JWT  | Switch active profile                   |
| `POST`   | `/api/media/upload`        | JWT  | Upload media file (multipart)           |
| `GET`    | `/api/events`              | JWT  | SSE event stream (sse-rs)               |

### MCP Tools (rmcp)

Available to AI agents via the `--mcp` flag or SSE transport:

| Tool                                                  | Description                                                       |
| ----------------------------------------------------- | ----------------------------------------------------------------- |
| `list_posts(filter)`                                  | Query scheduled/accepted posts with optional status filter        |
| `create_post(text, media?, schedule_at?, accounts[])` | Schedule a new post                                               |
| `get_post(id)`                                        | Full detail including retry history and publish log               |
| `cancel_post(id)`                                     | Remove a pending post from the scheduler queue                    |
| `update_post(id, fields...)`                          | Partial update: reschedule, edit text, reassign accounts          |
| `list_accounts()`                                     | Connected social accounts with auth status                        |
| `connect_account(provider, auth_code)`                | Initiate OAuth connection for a provider                          |
| `disconnect_account(id)`                              | Remove a connected social account                                 |
| `list_profiles()`                                     | List multi-account profiles                                       |
| `switch_profile(name)`                                | Switch the active profile                                         |
| `list_providers()`                                    | List enabled SocialProvider implementations                       |
| `get_scheduler_status()`                              | Current scheduler state: running/paused, pending count, last poll |
| `subscribe_events()`                                  | Live stream of post-publish, error, and token-refresh events      |

---

## Quick Start

### Prerequisites

- Rust 1.85+
- PostgreSQL 16+
- Docker & Docker Compose (optional)

### 1. Clone and configure

```bash
git clone https://github.com/ishanpm/postiz-rust.git
cd postiz-rust
cp .env.example .env
```

### 2. Database

```bash
# Using Docker for Postgres
docker compose up -d postgres

# Or local Postgres
createdb postiz
psql postiz < migrations/0001_initial.sql
```

### 3. Build and run

```bash
# Development (hot-reload with cargo-watch)
cargo watch -x run

# Release build (~12 MB binary)
cargo build --release
./target/release/postiz-rust
```

### 4. Verify

```bash
# REST API
curl http://localhost:3000/api/health

# MCP over stdio
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | ./target/release/postiz-rust --mcp
```

---

### Docker

```bash
# Build (multi-stage, ~15 MB final image)
docker build -t postiz-rust .

# Run with Docker Compose (Postgres + app)
docker compose up --build
```

### Docker Compose Stack

```yaml
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: postiz
      POSTGRES_PASSWORD: postiz
    ports:
      - "5432:5432"
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postiz"]
      interval: 5s
      timeout: 3s
      retries: 5

  app:
    build: .
    ports:
      - "3000:3000"
    env_file: .env
    depends_on:
      postgres:
        condition: service_healthy
```

---

## Authentication & OAuth

### JWT Authentication

Postiz Rust uses **JSON Web Tokens** (jsonwebtoken crate) with Argon2 password hashing:

1. User registers → password hashed with Argon2 (`argon2` crate)
2. User logs in → password verified, JWT returned (signed with `JWT_SECRET`)
3. Subsequent requests → `Authorization: Bearer <token>` header
4. axum middleware validates token, injects `Extension(UserId)` into handlers
5. Optional `FromRequestParts` impl for ergonomic handler extraction

### Social Provider OAuth

Each provider follows its platform's OAuth flow. The `SocialProvider` trait abstracts the common pattern:

```rust
#[async_trait]
pub trait SocialProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn authorize(&self, auth_code: &str) -> Result<ProviderToken, AppError>;
    async fn refresh_token(&self, token: &ProviderToken) -> Result<ProviderToken, AppError>;
    async fn publish_post(&self, post: &ScheduledPost, token: &ProviderToken) -> Result<String, AppError>;
    async fn validate_token(&self, token: &ProviderToken) -> Result<bool, AppError>;
}
```

| Provider      | Flow                       | Token Lifecycle               | Notes                               |
| ------------- | -------------------------- | ----------------------------- | ----------------------------------- |
| **Facebook**  | OAuth 2.0 + Meta Graph API | 60-day page tokens            | No refresh_token; re-auth at expiry |
| **Instagram** | OAuth 2.0 (via Facebook)   | 60-day via fb_exchange_token  | Container → publish two-phase       |
| **LinkedIn**  | OAuth 2.0 (Voyager)        | Session-based cookie auth     | `li_at` cookie profile              |
| **Bluesky**   | AT Protocol (ATP)          | Session JWT                   | `auth_token` cookie profile         |
| **X/Twitter** | OAuth 2.0 PKCE             | 2-hour access + refresh token | v2 API                              |

---

## Multi-Account Profiles

Postiz Rust supports managing multiple social accounts across all providers through a profile system:

```
~/.postiz-rust/profiles/
├── default/
│   └── cookies.json        # li_at, auth_token, sessionid, etc.
├── work/
│   └── cookies.json
└── personal/
    └── cookies.json
```

- **Profile Manager** — manages cookie storage per profile
- **MCP Tools** — `list_profiles`, `switch_profile`, `delete_profile`
- **Fallback Chain** — `--profile` CLI flag → active profile file → `default`
- **Cookie Auth** — Supports Chromium AES-GCM and Firefox plaintext cookie import

---

## Project Structure

```
postiz-rust/
├── Cargo.toml                  # Workspace (or single crate)
├── Cargo.lock
├── Dockerfile                  # Multi-stage musl build
├── docker-compose.yml          # Postgres + app stack
├── .env.example                # 20+ env vars with docs
├── LICENSE
│
├── src/
│   ├── main.rs                 # Entry: HTTP server + MCP stdio + scheduler spawn
│   ├── lib.rs                  # Module re-exports
│   ├── config.rs               # 12-factor env config (figment + env vars)
│   ├── error.rs                # Unified AppError → HTTP status (thiserror + IntoResponse)
│   │
│   ├── api/                    # axum HTTP Router (REST for humans)
│   │   ├── mod.rs              # Router composition + middleware chain
│   │   ├── auth.rs             # POST login, register, GET me
│   │   ├── posts.rs            # CRUD scheduled posts
│   │   ├── accounts.rs         # Connect/list/disconnect social accounts
│   │   ├── media.rs            # Multipart upload
│   │   ├── profiles.rs         # Multi-account profile management
│   │   └── health.rs           # Health check endpoint
│   │
│   ├── mcp/                    # rmcp MCP Server (tools for AI agents)
│   │   ├── mod.rs              # Router → tool dispatcher
│   │   ├── tools.rs            # Tool definitions + call_tool match
│   │   └── types.rs            # JSON-RPC type wrappers
│   │
│   ├── auth/                   # Authentication & authorization
│   │   ├── mod.rs
│   │   ├── jwt.rs              # Token creation/validation (jsonwebtoken)
│   │   ├── password.rs         # Argon2 hashing + verification
│   │   └── middleware.rs       # axum from_fn auth middleware
│   │
│   ├── social/                 # Social provider system
│   │   ├── mod.rs              # SocialProvider trait + ProviderRegistry
│   │   ├── twitter.rs          # X/Twitter v2 API client (OAuth 2.0 PKCE)
│   │   ├── linkedin.rs         # LinkedIn Voyager API client (cookie auth)
│   │   ├── bluesky.rs          # Bluesky AT Protocol client
│   │   ├── facebook.rs         # Meta Graph API client (page tokens)
│   │   └── instagram.rs        # Instagram Graph API client (container + publish)
│   │
│   ├── scheduler/              # In-process task scheduler
│   │   ├── mod.rs              # tokio::spawn + 30s interval + broadcast
│   │   └── worker.rs           # Poll + dispatch + retry + token refresh
│   │
│   ├── realtime/               # Real-time event system
│   │   ├── mod.rs              # ServerEvent enum (serde)
│   │   └── sse.rs              # SSE response stream (BroadcastStream)
│   │
│   ├── db/                     # Database access
│   │   ├── mod.rs              # Pool setup (sqlx::PgPool)
│   │   └── models.rs           # User, Account, Post, Media, ScheduledPost
│   │
│   └── middleware/             # Shared middleware
│       └── mod.rs              # CORS, logging, rate limiting
│
├── migrations/                 # SQL migrations
│   ├── 0001_initial.sql        # users, accounts, posts, media, scheduled_posts
│   └── ...
│
└── tests/                      # Integration tests
    ├── api_tests.rs            # REST endpoint tests via reqwest
    └── mcp_tests.rs            # MCP stdio tests via Python subprocess
```

---

## Configuration

All configuration is driven by environment variables (12-factor app pattern). See `.env.example` for defaults.

| Variable                  | Required | Default   | Description                                |
| ------------------------- | -------- | --------- | ------------------------------------------ |
| `DATABASE_URL`            | Yes      | —         | PostgreSQL connection string               |
| `JWT_SECRET`              | Yes      | —         | HMAC secret for JWT signing (min 32 chars) |
| `HOST`                    | No       | `0.0.0.0` | HTTP bind address                          |
| `PORT`                    | No       | `3000`    | HTTP port                                  |
| `RUST_LOG`                | No       | `info`    | Log level (env_logger)                     |
| `SCHEDULER_INTERVAL_MS`   | No       | `30000`   | Scheduler poll interval                    |
| `SCHEDULER_MAX_RETRIES`   | No       | `3`       | Max publish retries per post               |
| `SCHEDULER_RETRY_BASE_MS` | No       | `1000`    | Exponential backoff base                   |
| `MAX_UPLOAD_SIZE_MB`      | No       | `10`      | Media upload limit                         |
| `TWITTER_CLIENT_ID`       | —\*      | —         | X/Twitter OAuth 2.0 client ID              |
| `TWITTER_CLIENT_SECRET`   | —\*      | —         | X/Twitter OAuth 2.0 client secret          |
| `LINKEDIN_COOKIE`         | —\*      | —         | LinkedIn `li_at` session cookie            |
| `BLUESKY_HANDLE`          | —\*      | —         | Bluesky account handle                     |
| `BLUESKY_APP_PASSWORD`    | —\*      | —         | Bluesky app password                       |
| `FACEBOOK_CLIENT_ID`      | —\*      | —         | Meta app ID                                |
| `FACEBOOK_CLIENT_SECRET`  | —\*      | —         | Meta app secret                            |
| `INSTAGRAM_CLIENT_ID`     | —\*      | —         | Instagram app ID                           |
| `INSTAGRAM_CLIENT_SECRET` | —\*      | —         | Instagram app secret                       |

\*Required per provider you enable.

---

## Key Design Decisions

### 1. Dual Interface Over Shared Logic

**Decision:** A single binary serves both REST and MCP from the same `AppState` instead of two separate services.

**Trade-off:** Slightly larger binary; eliminates sync/drift problems between human-facing and agent-facing APIs. Both interfaces hit identical provider and database code paths.

### 2. In-Process Scheduler vs External Queue

**Decision:** `tokio::spawn` + periodic DB polling instead of Redis/Celery/Sidekiq.

**Trade-off:** Simpler deployment (no Redis dependency) at the cost of no persistent queue across restarts. For a single-instance scheduler serving <10k accounts, DB polling at 30s is sufficient. External queue can be added later behind the same trait.

### 3. Trait-Based Provider System

**Decision:** `SocialProvider` trait with dynamic dispatch via `ProviderRegistry`.

**Trade-off:** Slight runtime overhead from trait objects vs compile-time monomorphization. Gain: adding a new network requires exactly one file and one `register()` call. The trait is shared by REST handlers, MCP tools, and the scheduler — cross-cutting concerns (rate limiting, retry) live in the trait wrapper, not per-provider.

### 4. Cookie-Based Auth for LinkedIn

**Decision:** LinkedIn uses session cookie auth (`li_at`) instead of OAuth 2.0.

**Rationale:** LinkedIn's Voyager API (used by first-party clients) has no stable OAuth for non-enterprise. Cookie auth works reliably for single-account setups. Multi-account profiles bridge the gap.

### 5. SSE via Broadcast Channel

**Decision:** `tokio::sync::broadcast` feedings `axum::response::sse::Sse` with `tokio_stream::wrappers::BroadcastStream`.

**Rationale:** Zero external deps for real-time events. Clients subscribe with `EventSource` (browser) or SSE client (AI agents). The broadcast channel is subscriber-count-aware — it drops `Receiver` when clients disconnect, preventing unbounded channel growth.

### 6. Unified Error Handling

**Decision:** Single `AppError` enum (thiserror) with `IntoResponse` for HTTP + JSON error body.

**Rationale:** Both REST handlers and MCP tools can return `Result<_, AppError>`. Error JSON format is consistent across the entire API. `From` impls for sqlx, jsonwebtoken, argon2, and anyhow keep handler code clean.

### 7. Multi-Arch Docker via musl

**Decision:** Build with `x86_64-unknown-linux-musl` / `aarch64-unknown-linux-musl` targets for Alpine.

**Rationale:** ~15 MB final image (vs ~800 MB for debian-based). Static linking eliminates libc dependency. `TARGETARCH` build arg detects architecture at build time.

---

## Docker Multi-Stage Build

```dockerfile
# Stage 1: Build
FROM rust:alpine AS builder
RUN apk add --no-cache musl-dev pkgconfig openssl-dev
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
RUN cargo build --release --target x86_64-unknown-linux-musl

# Stage 2: Runtime
FROM scratch
COPY --from=builder /app/target/release/postiz-rust /postiz-rust
EXPOSE 3000
ENTRYPOINT ["/postiz-rust"]
```

- **Multi-architecture:** `docker buildx build --platform linux/amd64,linux/arm64 ...`
- **Image size:** ~15 MB compressed
- **RSS:** ~4 MB idle (single tenant)
- **Startup:** near-instant (< 100 ms from cold)

---

## Testing

```bash
# Unit + integration tests
cargo test

# With specific database for integration tests
DATABASE_URL=postgres://postiz:postiz@localhost/postiz cargo test --test api_tests

# MCP protocol-level tests (Python subprocess)
python tests/mcp_protocol_test.py
```

The test suite includes:

- Rust `#[cfg(test)]` unit tests for validation, JWT, config parsing
- Rust integration tests for REST endpoints via `reqwest`
- Python subprocess tests for MCP stdio protocol compliance (JSON-RPC initialise → tool list → call → response)

---

## License

MIT — see [LICENSE](LICENSE).

---

_Built with [axum](https://github.com/tokio-rs/axum), [rmcp](https://github.com/modelcontextprotocol/rust-sdk), [sqlx](https://github.com/launchbadge/sqlx), and [tokio](https://github.com/tokio-rs/tokio)._
