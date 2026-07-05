# AGENTS.md — Social Forge Development Protocol

> **This document is the single source of truth for any AI agent (or human developer) working in this repository.** Read it completely before making any changes. It covers the architecture, the build/test workflow, the git protocol, the security model, and the known pitfalls that have burned previous iterations.

---

## 0. THE GOLDEN RULES (read these first)

1. **ALWAYS PUSH COMMITS AFTER EACH ITERATION.** After every meaningful unit of work — a bug fix, a feature, a refactor phase — you MUST `git commit` AND `git push origin master`. The user tests locally on their machine by pulling from `origin/master`. If you don't push, your work doesn't exist as far as the user is concerned. Do not batch 5 phases of work into one giant commit; commit each phase as you complete it.

2. **NEVER break the build.** Before committing, verify:
   - `cargo check --lib --bin social-forge` → 0 errors (warnings are OK)
   - `cd frontend && pnpm build` → succeeds
   - If you added Rust unit tests: `cargo test --lib` → all pass
   - If you can't verify (e.g. missing `cmake` for boringssl), say so explicitly in the commit message and explain what the user needs to verify locally.

3. **NEVER skip the security review.** Every change that touches HTML interpolation, file upload, auth, or SQL must be reviewed against the security rules in §6. The v7-v9 audit history is full of XSS and encryption-bypass bugs that were introduced by agents who didn't read this file.

4. **NEVER use `sqlx::query!` / `query_as!` macros for NEW queries** unless you have a running Postgres + can regenerate the `.sqlx/` offline cache. Use runtime `sqlx::query()` / `sqlx::query_as()` with `FromRow` structs instead. See §5 for details.

5. **NEVER commit secrets.** The `.env` file is gitignored. `.env.example` is the template. API keys, OAuth secrets, and `APP_PASSWORD` values must never appear in code or commits.

6. **ALWAYS read the worklog before starting.** If a `worklog.md` exists in the project root, read it to see what previous agents did. Append your own entry when you finish. (If the session uses the multi-agent worklog protocol — see §8.)

---

## 1. What is Social Forge?

Social Forge is a **single-user, self-hosted social media management platform** designed for AI agents. It provides a triple interface — CLI, REST API, and MCP server — over 30+ social platforms (X, Reddit, LinkedIn, Facebook, Instagram, YouTube, Threads, TikTok, Bluesky, Mastodon, Pinterest, Discord, Slack, Telegram, WhatsApp, WordPress, Medium, Dev.to, Hashnode, GitHub, etc.).

**Architecture in one paragraph:** A single Rust binary (`social-forge`) runs an axum HTTP server (REST API + embedded SvelteKit frontend), an rmcp MCP server (stdio, for AI agents like Claude/Cursor), an in-process scheduler (polls for due posts every 30s), an SSE broadcaster (realtime updates to the frontend), and background tasks (RSS poller, feed refresher, analytics cache refresher). All state lives in PostgreSQL. OAuth tokens are AES-256-GCM encrypted at rest when `TOKEN_ENCRYPTION_KEY` is set.

**Key numbers (as of v9):**
- 328 MCP tools across 44 files in `src/mcp/`
- 31 providers registered in `src/social/registry.rs` (25 with MCP coverage)
- 18 SQL migrations in `migrations/`
- ~58,000 LOC of Rust + ~13,000 LOC of Svelte/TS frontend

---

## 2. Repository Layout

```
social-forge/
├── src/
│   ├── main.rs              # Entry point: arg parsing, server startup, graceful shutdown
│   ├── config.rs            # Config::from_env() — all env var loading
│   ├── crypto.rs            # AES-256-GCM encrypt/decrypt for tokens at rest
│   ├── error.rs             # AppError type
│   ├── lib.rs               # Module declarations
│   ├── api/                 # axum HTTP router + route handlers
│   │   ├── mod.rs           # Router builder, AppState struct, /health, /ready, /api/metrics, CSRF middleware
│   │   ├── auth.rs          # POST /api/auth/login (password gate + session cookie)
│   │   ├── posts.rs         # CRUD for posts
│   │   ├── media.rs         # Upload/list/delete/serve media (MIME allowlist + magic-byte sniff)
│   │   ├── onboard.rs       # /setup page + OAuth/cookie connect flows (html_escape + js_escape helpers)
│   │   ├── integrations.rs  # List/connect/disconnect/refresh integrations
│   │   └── ...              # calendar, feed, comments, dms, automation, tags, signatures, webhooks, rss, analytics, developer, notifications, billing
│   ├── auth/
│   │   ├── mod.rs           # JWT create/validate
│   │   ├── middleware.rs    # auth_middleware (validates sf_session cookie), AuthenticatedUser extractor, DEFAULT_USER_ID
│   │   └── jwt.rs           # hash_password / verify_password (argon2), create_token / validate_token (jsonwebtoken)
│   ├── cli/                 # CLI commands (mirrors MCP tools)
│   │   ├── run.rs           # Main CLI dispatcher (~1975 LOC)
│   │   └── platforms/       # 32 platform shims (thin wrappers around unified commands)
│   ├── mcp/                 # MCP server (328 tools)
│   │   ├── mod.rs           # SocialForgeMcpServer impl + #[tool] method registration (~2800 LOC)
│   │   ├── tools_posts.rs   # Post CRUD MCP tools
│   │   ├── tools_admin.rs   # v9 parity wrappers (posts_repeat, posts_set_tags, media_delete, etc.)
│   │   ├── tools_x.rs       # X-specific tools (create_tweet, reply, like, retweet, search, timeline)
│   │   ├── tools_{platform}.rs  # Per-platform tool modules
│   │   └── auth.rs          # MCP auth (stdio = trusted local)
│   ├── social/              # Provider trait + 31 provider implementations
│   │   ├── mod.rs           # SocialProvider trait, ProviderError enum, PostContent, MediaAttachment
│   │   ├── registry.rs      # ProviderRegistry (provider lookup + per-provider Semaphore)
│   │   ├── x.rs             # X/Twitter (wreq + GraphQL cookie auth, ~2200 LOC)
│   │   ├── reddit.rs        # Reddit (OAuth + cookie auth, ~1500 LOC)
│   │   └── ...              # linkedin, facebook, instagram, threads, youtube, tiktok, bluesky, mastodon, pinterest, discord, slack, telegram_bot, telegram_user, whatsapp, wordpress, medium, devto, hashnode, github, google, vk, kick, whop, skool, lemmy, farcaster, google_my_business
│   ├── db/
│   │   ├── mod.rs           # create_pool() — PgPoolOptions with env-configurable limits
│   │   ├── models.rs        # PostState enum (Draft/Queued/Publishing/Published/Error), Post, PostWithIntegration, Integration, etc.
│   │   └── queries.rs       # All SQL queries (~2100 LOC)
│   ├── scheduler/
│   │   └── mod.rs           # In-process scheduler: process_due_posts (JoinSet + Semaphore), publish_post (retry + backoff), proactive_token_refresh, analytics cache refresh
│   ├── services/
│   │   ├── posts.rs         # PostService (shared by API + MCP + CLI) — publish, resolve_token
│   │   ├── content_splitter.rs  # Split long content into platform-specific chunks
│   │   ├── webhook_dispatcher.rs  # send_webhook + dispatch_event
│   │   └── notifications.rs
│   ├── realtime/
│   │   └── mod.rs           # Broadcaster (SSE fan-out to frontend)
│   ├── feed/                # Feed import/refresh (background task)
│   ├── rss/                 # RSS auto-post (background task)
│   └── wa/                  # WhatsApp Web client (wa-rs)
├── frontend/                # SvelteKit 5 frontend
│   ├── src/
│   │   ├── routes/          # 25 page routes (dashboard, posts, calendar, feed, analytics, search, channels, settings/*)
│   │   ├── lib/
│   │   │   ├── api/         # API client modules (posts.ts, analytics.ts, integrations.ts, etc.)
│   │   │   ├── ui/          # Reusable components (Button, Modal, Badge, Icon, Spinner, Card, Dropdown)
│   │   │   ├── composer/    # Post composer components (RichTextEditor, ChannelSelector, SchedulePicker, etc.)
│   │   │   ├── stores/      # realtime.ts (SSE client), toast.ts, calendar.svelte.ts
│   │   │   └── channels/    # Channel connect flows (ApiKeyConnect, Web3Connect, TimeSlotEditor, etc.)
│   │   ├── app.css          # Global styles (Inter font, prefers-reduced-motion, skeleton shimmer)
│   │   └── app.html         # HTML shell (Google Fonts)
│   ├── tailwind.config.js   # Semantic color tokens (surface, line, muted, content, brand)
│   ├── svelte.config.js     # adapter-static (SPA fallback)
│   └── tsconfig.json        # Extends .svelte-kit/tsconfig.json
├── migrations/              # 18 SQL migrations (001-018, additive-only, no destructive ops)
├── scripts/
│   ├── install.sh           # One-line installer (downloads pre-built binary, sets up systemd)
│   ├── social-forge.service # systemd unit
│   └── social-forge-start.sh
├── Dockerfile               # Multi-stage: rust:1.94-slim-bookworm → debian:bookworm-slim (non-root)
├── docker-compose.yml       # Postgres-only (app runs via systemd)
├── Makefile                 # build, deploy, redeploy, restart, logs, watch
├── Cargo.toml               # Rust dependencies
├── .env.example             # Template for .env (all env vars documented)
└── AGENTS.md                # THIS FILE
```

---

## 3. Build & Test Workflow

### 3.1 Prerequisites

The backend requires `cmake` + `libclang` for `boring-sys` (BoringSSL, used by `wreq` for TLS fingerprinting). On Debian/Ubuntu:
```bash
sudo apt-get install -y cmake libclang-dev pkg-config libssl-dev libsqlite3-dev
```

The frontend requires Node 20+ and pnpm:
```bash
cd frontend && pnpm install
```

### 3.2 Backend build

```bash
# Fast check (no codegen) — use this during development:
cargo check --lib --bin social-forge

# Full release build:
cargo build --release

# If you get "SQLX_OFFLINE=true but there is no cached data for this query":
# Either (a) run against a live DB with DATABASE_URL set, or
#        (b) use runtime sqlx::query() instead of sqlx::query! macro (see §5)

# Run unit tests (fast, no DB needed for most):
cargo test --lib

# Run a specific test:
cargo test --lib -- html_escape
```

**Note on boringssl build time:** The first `cargo build` will compile BoringSSL from source (~5-10 minutes). Subsequent builds use the cached artifacts in `target/debug/build/boring-sys2-*/`. If you delete `target/`, you'll wait again.

### 3.3 Frontend build

```bash
cd frontend
pnpm install          # first time only
pnpm build            # production build → frontend/build/
pnpm exec svelte-check --threshold error  # type-check (must be 0 errors)
```

The frontend is embedded into the Rust binary at compile time via `rust-embed` (`src/api/mod.rs:15-18`, `#[folder = "frontend/build"]`). So you must run `pnpm build` BEFORE `cargo build --release` if frontend assets changed.

### 3.4 Full deploy (on the target VPS)

```bash
make deploy      # builds frontend + Rust, installs skill, restarts systemd
# or the fast path (skip frontend rebuild):
make redeploy    # cargo zigbuild --release + install binary + restart
```

### 3.5 Running locally

```bash
# Start postgres:
docker compose up -d postgres

# Run the server (HTTP, no TLS, loopback only):
BIND_HOST=127.0.0.1 APP_URL=http://localhost:6543 cargo run -- serve

# Run MCP server on stdio (for Claude Desktop / Cursor):
cargo run -- mcp

# Use the CLI:
cargo run -- posts list
cargo run -- x timeline
```

---

## 4. Git Protocol

### 4.1 Commit discipline

**ALWAYS PUSH AFTER EACH ITERATION.** The user pulls from `origin/master` to test locally. The workflow is:

```bash
# 1. Make your changes
# 2. Verify the build (see §3.2 + §3.3)
cargo check --lib --bin social-forge   # must be 0 errors
cd frontend && pnpm build              # must succeed
cargo test --lib                       # must pass

# 3. Stage + commit with a descriptive message
git add -A
git commit -m "vN Phase X: <one-line summary>

<detailed description of what changed and why>

Verification:
- Backend cargo check: 0 errors
- Frontend pnpm build: succeeds
- Backend cargo test --lib: 53/53 pass"

# 4. PUSH IMMEDIATELY — do not batch multiple phases
git push origin master

# 5. Confirm the push succeeded
git log --oneline -1
```

### 4.2 Commit message conventions

- Start with a version prefix: `v9 Stage 1:`, `v9 Stage 2:`, `v10:`, etc.
- First line: imperative mood, ≤72 chars, summarize the phase
- Body: what changed, why, and the verification results
- Reference file paths + line numbers when fixing specific bugs
- If you couldn't verify the build (e.g. sandbox missing `cmake`), say so explicitly

### 4.3 Branching

This repo uses a single `master` branch. No feature branches, no PRs — commit directly to `master` and push. The user reviews the diff on GitHub and tests locally.

---

## 5. SQL Query Rules (critical — read this)

### 5.1 The `.sqlx/` offline cache problem

`sqlx::query!` and `sqlx::query_as!` macros require **either** a live database connection at compile time (via `DATABASE_URL`) **or** pre-generated offline cache files in `.sqlx/`. If you add a new query using these macros, the build will fail with:

```
error: `SQLX_OFFLINE=true` but there is no cached data for this query
```

### 5.2 When to use which form

| Situation | Use |
|---|---|
| Modifying an EXISTING `query!` macro query | Keep the macro form — the cache entry already exists |
| Adding a NEW query, and you have a live DB | `sqlx::query!` macro + run `cargo sqlx prepare` to update `.sqlx/` |
| Adding a NEW query, and you DON'T have a live DB | **Runtime `sqlx::query()` / `sqlx::query_as()` with `FromRow` structs** |

### 5.3 Runtime query pattern

```rust
use serde::{Deserialize, Serialize};

#[derive(Default, sqlx::FromRow)]
struct MyCounts {
    total: i64,
    published: i64,
}

let counts: MyCounts = sqlx::query_as(
    "SELECT COUNT(*) as total, COUNT(*) FILTER (WHERE state = 'published') as published
     FROM posts WHERE user_id = $1",
)
.bind(user_id)
.fetch_one(&db)
.await
.map_err(|e| format!("DB error: {e}"))?;
```

**Trade-off:** No compile-time column type checking. But the query compiles without a DB connection, which is essential in sandboxed CI environments.

### 5.4 All SQL must be parameterized

NEVER use `format!()` to interpolate values into SQL. Always use `$1, $2, ...` bind parameters. The only exception is building a `LIKE` pattern: `format!("%{}%", query)` is OK because the result is passed as a bind value, not interpolated into the SQL string.

---

## 6. Security Rules (non-negotiable)

### 6.1 HTML interpolation

**Every** value interpolated into HTML in `src/api/onboard.rs` (or anywhere else that emits HTML) MUST go through `html_escape()`. This includes:
- Query string parameters (`?connected=`, `?error=`, `?name=`, etc.)
- DB-sourced fields (`profile_name`, `profile_picture`, `provider_identifier`)
- Upstream API responses (Telegram bot API, etc.)

The `html_escape()` and `js_escape()` helpers live at the bottom of `src/api/onboard.rs`. Use them.

```rust
// CORRECT:
format!(r#"<strong>{name}</strong>"#, name = html_escape(name))

// WRONG (XSS):
format!(r#"<strong>{name}</strong>"#, name = name)
```

For values interpolated into JavaScript string literals (inside `<script>` blocks), use BOTH `js_escape()` AND `html_escape()`:
```rust
let iid_safe = html_escape(&js_escape(iid));
```

### 6.2 File upload security

`src/api/media.rs` enforces:
- MIME allowlist: `image/png`, `image/jpeg`, `image/webp`, `image/gif`, `video/mp4`, `video/quicktime`
- Magic-byte sniffing (`sniff_mime()`) — client-supplied `Content-Type` is never trusted alone
- `serve_media` always sets `X-Content-Type-Options: nosniff`
- Non-image MIME types get `Content-Disposition: attachment`

If you add a new upload endpoint, follow the same pattern.

### 6.3 Token encryption at rest

When `TOKEN_ENCRYPTION_KEY` is set (64 hex chars = 32 bytes), ALL paths that write OAuth access tokens to the DB MUST encrypt them first:

```rust
let enc_access_token = if let Some(ref k) = token_key {
    crate::crypto::encrypt_string(&token.access_token, k)
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to encrypt token: {e}");
            token.access_token.clone()
        })
} else {
    token.access_token.clone()
};
queries::update_integration_token(db, integration_id, &enc_access_token, ...).await?;
```

**There are 4 call sites that refresh tokens** — all must encrypt:
1. `src/scheduler/mod.rs::publish_post` (mid-publish refresh) ✅
2. `src/scheduler/mod.rs::resolve_token` (pre-publish refresh) ✅
3. `src/services/posts.rs::resolve_token` (manual publish path) ✅
4. `src/api/integrations.rs::refresh` (manual refresh endpoint) ✅
5. `src/mcp/tools_admin.rs::handle_integrations_refresh` (MCP refresh tool) ✅

If you add a 6th refresh path, encrypt there too.

### 6.4 Auth gate

All endpoints except `/health`, `/ready`, `/api/metrics`, `/api/auth/login`, `/api/auth/callback`, `/api/events` (SSE), `/api/media/{id}` (serve), `/api/proxy-media`, and `/api/billing/webhook` require the `sf_session` cookie (validated by `auth_middleware`).

The `/setup` page and `/api/public/connect/*` routes accept EITHER the `sf_session` cookie OR a `?token=<jwt>` query param (via `resolve_authed_user()` in `onboard.rs`). They do NOT mint fresh JWTs for anonymous visitors — that was a security hole closed in v7.

### 6.5 CSRF defense

All state-changing routes (POST/PUT/DELETE) pass through `csrf_origin_check` middleware which validates the `Origin` (or `Referer` fallback) header against `FRONTEND_URL`. Safe methods (GET/HEAD/OPTIONS) are not checked.

### 6.6 Bind address

The server binds to `127.0.0.1` by default. To expose on the LAN, set `BIND_HOST=0.0.0.0` (with a startup warning). Never change the default back to `0.0.0.0`.

---

## 7. Scheduler Internals (read before touching `src/scheduler/`)

### 7.1 Post state machine

```
draft → queued → publishing → published
                    ↓
                  error
```

- `draft`: created but not scheduled
- `queued`: scheduled, waiting for `scheduled_at`
- `publishing`: scheduler has claimed it and is mid-API-call (added in v9 migration 018)
- `published`: successfully published
- `error`: failed after MAX_RETRIES (3)

### 7.2 How `process_due_posts` works (v9-v10)

1. `get_due_posts()` atomically transitions `queued → publishing` for due posts via a CTE (`UPDATE ... WHERE id IN (SELECT ... FOR UPDATE SKIP LOCKED) RETURNING`). This prevents dual-instance double-publish.
2. **Circuit breaker check (v10)**: for each claimed post, check the per-provider circuit breaker. If open, push the post back to `queued` state (skip publishing). This prevents cascading failures during platform outages.
3. For each post that passes the circuit breaker, acquire a per-provider `Semaphore` permit (default 1 for strict platforms, 3 for high-headroom).
4. Spawn the publish task on a **tracked `JoinSet`** (not detached `tokio::spawn`).
5. After each publish, call `circuit_breaker.record_success()` or `record_failure()` to update the breaker state.
6. Wait for all tasks to complete (with 5min timeout per `join_next`) before returning.
7. On startup, `reclaim_stuck_publishing()` marks posts stuck in `publishing` > 5min as `error`.

### 7.3 Circuit breaker (v10)

Each provider has a `CircuitBreaker` with three states:
- **Closed**: all requests pass through. Failure count tracked.
- **Open** (after 5 consecutive failures): all requests rejected for 60s. Posts pushed back to `queued`.
- **Half-open** (after cooldown): one request allowed. If success → closed. If fail → open.

Configurable via env vars:
- `PROVIDER_CB_THRESHOLD_{ID}` (default 5, range 1-50) — failures before opening
- `PROVIDER_CB_COOLDOWN_{ID}` (default 60s, range 10-3600) — open duration

### 7.4 Retry policy

- `TokenExpired`: refresh once, retry immediately. If refresh fails, mark `integration.refresh_needed = true`.
- `RateLimited`: exponential backoff `2^attempt × 5s ± 25% jitter`. Max 3 retries.
- `Network`: same exponential backoff (was: immediate fail — a bug fixed in v9).
- `Auth` / `Api` / `InvalidRequest`: no retry (won't succeed).

### 7.5 Thread linking (v10)

When the scheduler publishes a post with `group_id` and `sequence > 1`, it looks up the previous post in the same group (by `sequence - 1`) and passes its `platform_post_id` as `in_reply_to` in the `PostContent`. Providers that support threading (X, Bluesky, Mastodon, Threads) use this to create linked replies instead of standalone posts.

### 7.6 Webhook dispatch

`dispatch_webhook_background()` fires `post.published` and `post.failed` events to all active webhooks matching the event type. Runs in a detached task so it doesn't block the scheduler tick. Records delivery attempts in `webhook_deliveries` table.

### 7.7 Audit trail

Every publish attempt (success or failure) is recorded in the `publish_attempts` table via `record_publish_attempt()`. The operator can query this to see the full retry history.

---

## 8. Multi-Agent Worklog Protocol

If multiple agents are working on the repo in sequence, use the shared worklog at `/home/z/my-project/worklog.md`:

1. **Before starting work:** Read `worklog.md` to see what previous agents did.
2. **After finishing work:** Append a new section (do NOT overwrite) using this template:

```markdown
---
Task ID: <task id, e.g. "v9-stage-1">
Agent: <agent name>
Task: <what you were asked to do>

Work Log:
- <step 1>
- <step 2>
- ...

Stage Summary:
- <key results>
- <artifacts produced>
- <known issues / follow-ups>
```

---

## 9. Known Pitfalls (things that have burned previous agents)

### 9.1 `wreq` version pinning

`wreq` is pinned to `=6.0.0-rc.23` in `Cargo.toml`. Do NOT upgrade to `rc.29+` — it has breaking API changes. The `wreq` crate provides Chrome TLS fingerprinting for X/Twitter GraphQL API (regular `reqwest` gets blocked).

### 9.2 `core2` yanked crate

There's a local stub at `vendor/core2/` that re-exports `std::error`, with a `[patch.crates-io]` section in `Cargo.toml`. This works around the yanked `core2 0.4.0` dependency. Do NOT remove this.

### 9.3 `rmcp` API

`ServerHandler::call_tool` needs `RequestContext`. The CLI `mcp_bridge.rs` is stubbed to redirect users to `social-forge mcp` (the stdio server). Don't try to call MCP tools from the CLI directly — use the shared `services/` layer.

### 9.4 BoringSSL build requires cmake + libclang

If you see `is cmake not installed?` or `Unable to find libclang`, install:
```bash
sudo apt-get install -y cmake libclang-dev
```

In sandboxed environments without root, you can sometimes extract the `.deb` files manually (see `/home/z/my-project/scripts/cargo-env.sh` for the env var setup).

### 9.5 SQLX offline cache

If you add a `sqlx::query!` macro and the build fails with "no cached data", either:
- Run `cargo sqlx prepare` against a live DB (requires `cargo install sqlx-cli`), OR
- Convert the query to runtime `sqlx::query()` form (see §5.3)

### 9.6 Svelte 5 runes

The frontend uses Svelte 5 runes (`$state`, `$derived`, `$effect`, `$props`). Do NOT use Svelte 4 syntax (`let x = writable()`, `$: x = y`). The `svelte.config.js` suppresses `a11y_*` and `state_referenced_locally` warnings.

### 9.7 Tailwind semantic colors

Use semantic Tailwind classes (`bg-surface`, `text-muted`, `border-line`, `bg-brand-500`, etc.) defined in `frontend/tailwind.config.js`. Do NOT use hardcoded hex colors (`bg-[#1e2435]`) in CSS classes — the v8 audit replaced all of them. JS data values (provider color metadata, color picker palettes) are fine as hex strings.

### 9.8 Frontend realtime events

The SSE client (`frontend/src/lib/stores/realtime.ts`) subscribes to these events:
`post_created`, `post_scheduled`, `post_published`, `post_failed`, `post_deleted`, `integration_connected`, `integration_disconnected`, `notification_new`.

If you add a new broadcast event in the backend (`broadcaster.send(...)`), add it to the `realtime.ts` subscription list AND wire up listeners in the relevant routes.

---

## 10. Environment Variables

See `.env.example` for the full list. Critical ones:

| Var | Required | Default | Purpose |
|---|---|---|---|
| `DATABASE_URL` | Yes | — | Postgres connection string |
| `APP_PASSWORD` | No | auto-generated | Single-user password gate (persisted to `~/.social-forge/.env`) |
| `APP_URL` | No | `https://localhost:6543` | Public URL (used for OAuth redirect URIs + TLS decision) |
| `BIND_HOST` | No | `127.0.0.1` | Network interface to bind (`0.0.0.0` for LAN) |
| `FRONTEND_URL` | No | = `APP_URL` | CORS allow-list + CSRF origin |
| `JWT_SECRET` | No | derived from `APP_PASSWORD` | HMAC key for session cookies |
| `TOKEN_ENCRYPTION_KEY` | No | — | 64 hex chars (32 bytes) for AES-256-GCM token encryption at rest |
| `DB_MAX_CONNECTIONS` | No | `20` | PgPool max connections |
| `DB_ACQUIRE_TIMEOUT` | No | `5` | seconds to wait for a pool connection |
| `PROVIDER_CONCURRENCY_X` | No | `1` | Per-provider concurrent publish limit (override) |
| `PROVIDER_CB_THRESHOLD_X` | No | `5` | Per-provider circuit breaker failure threshold (v10) |
| `PROVIDER_CB_COOLDOWN_X` | No | `60` | Per-provider circuit breaker cooldown seconds (v10) |

---

## 11. Quick Reference: CLI + MCP Commands

### CLI (preferred for shell-based AI agents)
```bash
social-forge --help
social-forge providers               # List connected accounts
social-forge post "hello" --platforms x,linkedin
social-forge posts list
social-forge posts publish <id>
social-forge media upload ./photo.jpg
social-forge x timeline
social-forge reddit browse rust
social-forge automation list
```

### MCP (for Claude Desktop / Cursor)
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

### Key MCP tool categories (328 total)
| Category | Sample tools |
|---|---|
| Posts | `posts_create`, `posts_list`, `posts_publish`, `posts_repeat`, `posts_set_tags`, `posts_stage` |
| Media | `posts_media_upload`, `posts_media_list`, `media_delete` |
| Engagement | `comments_get`, `comments_reply`, `dm_send`, `dm_list` |
| Per-platform | `x_create_tweet`, `reddit_search`, `bs_timeline`, `ms_get_analytics`, `yt_reply_comment` |
| Integrations | `integrations_list`, `integrations_refresh`, `integrations_disconnect` |
| Signatures | `signatures_list`, `signatures_create`, `signatures_update`, `signatures_delete` |
| Analytics | `analytics_summary`, `feed_analytics`, `analytics_get_post` |
| Automation | `automation_create`, `automation_list`, `automation_logs` |
| Webhooks | `wh_create`, `wh_list`, `wh_test` |

---

## 12. When You're Done

Before declaring a task complete, verify ALL of these:

- [ ] `cargo check --lib --bin social-forge` → 0 errors
- [ ] `cargo test --lib` → all tests pass
- [ ] `cd frontend && pnpm build` → succeeds
- [ ] `cd frontend && pnpm exec svelte-check --threshold error` → 0 errors
- [ ] `git add -A && git commit -m "..."` with a descriptive message
- [ ] **`git push origin master`** ← THIS IS THE MOST IMPORTANT STEP
- [ ] Confirm the push succeeded: `git log --oneline -1` shows your commit on `origin/master`

If you can't verify one of these (e.g. missing `cmake`), say so explicitly to the user and explain what they need to verify locally.
