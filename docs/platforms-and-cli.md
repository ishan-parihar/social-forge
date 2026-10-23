# Platforms and CLI

Moved out of the README to keep it short. Content is unchanged.

### Targeting: `--platforms` collisions and `--integrations`

Two registered providers are a strict prefix of another, so platform **names are
matched exactly**:

| `--platforms` value | Targets | Does NOT also target |
|---|---|---|
| `instagram` | `instagram` | `instagram-standalone` |
| `linkedin` | `linkedin` | `linkedin-page` |

```bash
# Exact names only — no double drafts
switchpost post "hello" --platforms instagram

# Name a suffixed variant explicitly to include it
switchpost post "hello" --platforms instagram,instagram-standalone

# Or skip name resolution entirely and pick exact accounts
switchpost providers                                   # get integration UUIDs
switchpost stage "hello" --integrations 6f1c...-...     # mutually exclusive with --platforms
```

`telegram` still fans out to `telegram-bot` and `telegram-user`, because no
integration is registered under the bare name `telegram`.

### Single-writer rule

SQLite admits one writer per file. `serve` and `mcp` both run the publish
scheduler, so exactly one long-lived process may own a given `DATABASE_URL` —
the second one fails at boot on an exclusive lock file rather than silently
double-publishing. Short CLI reads (`providers`, `posts list`, `audit`) never
take the lock. `switchpost audit` reports the live `journal_mode` /
`busy_timeout` alongside the disabled-integration inventory.

Full detail: [`docs/planning/TARGETING_AND_WRITER_DISCIPLINE.md`](docs/planning/TARGETING_AND_WRITER_DISCIPLINE.md).

### Logging a provider in (for AI agents)

`switchpost connect <provider>` is the one entry point. Three shapes:

|Provider kind|What `connect` does|What the agent does next|
|---|---|---|
|Cookie (x, reddit)|Imports browser cookies, then falls back to `X_AUTH_TOKEN`+`X_CT0` / `REDDIT_COOKIE_TOKEN` env; validates against the live API; writes the row|Nothing — it either prints `connected` or names what is missing|
|Token env (github, telegram-bot)|Validates the env token against the live API (`GET /user`, `getMe`) and writes the row NOW|Nothing — `connected`, or the exact validation error|
|Session env (bluesky)|Creates a real session with the app password (`createSession`), stores the accessJwt + did|Nothing — `connected`, or the exact error|
|OAuth (linkedin, facebook, instagram, threads, discord, slack, pinterest, tiktok, mastodon, youtube, google)|Emits a **provider-native** `auth_url` (the platform's own consent page, e.g. `https://www.threads.net/oauth/authorize?...`) when the client credentials are configured; `not_configured` + `requires` when they are not|Opens the URL in a browser, completes the platform login; the provider redirects to `/api/auth/callback` and the server writes the row|
|One-time-token (telegram user)|Prints the code + instructions|Sends the code to the bot|
|QR scan (telegram-user)|`qr_login_required` + the env names|Scans the QR in the dashboard's Telegram connect flow|
|Per-site / QR / extension (wordpress, whatsapp, skool)|Names the exact web-UI path|Follows it in the dashboard|

**The URL is usable with NO session cookie.** The CLI mints the
authorization URL itself (code_verifier + state + `oauth_states` row +
the provider's `generate_auth_url`), exactly as the web UI does. An agent
driving its own browser goes straight to the platform's consent page —
it never needs the founder's `sf_session` cookie, a `?token=` JWT, or a
popup. The provider redirects to `{APP_URL}/api/auth/callback`, an
auth-exempt route, and the server exchanges the code there.

**The wait loop.** `connect <provider> --wait [--timeout-secs N]` (default
300) turns the OAuth/OTT shapes into a single blocking command: it prints
the URL (or code), then polls every 2s and exits the moment the credential
row lands — `status: connected` and exit 0. Agent loop, end to end:

```sh
switchpost connect linkedin --wait &     # emits auth_url, polls
# drive a browser to the auth_url, complete the login
# -> the CLI prints: status: connected, accounts: [...]
```

`--wait` prints the `auth_url` BEFORE it starts polling, so the agent
never has to run the command twice.

**`redirect_uri` is printed alongside the URL.** When a platform answers
`redirect_uri_mismatch`, the URI the provider's developer app must have
registered is in the CLI's own output — no guessing:

```sh
switchpost connect linkedin --json
# {"status":"not_connected","auth_url":"https://www.linkedin.com/oauth/v2/authorization?...",
#  "redirect_uri":"https://localhost:6544/api/auth/callback", ...}
```

**Output format.** TOON is the default (~40% fewer tokens for an agent).
`--json` emits compact JSON and `--pretty` emits indented JSON — both
are global flags and apply to every command.

Seed rows never count as connected: a `seed-token` placeholder (from
`scripts/seed_demo.py`) is treated as absent, so `connect` re-imports and
replaces it instead of answering `already_connected` forever.

---

### Supported Platforms

Providers are tiered: **Tier-1** gets full depth (analytics, engagement, comments, mentions, target discovery); **Tier-2** is publish-maintained. Tier membership is defined once in `src/social/tier.rs` and reported as a `tier` field by `/api/providers`. Full rationale: `docs/planning/PLAN_PARITY_DEPTH_SINGLEUSER_v25.md` §1.

#### Tier-1 — depth (12)

| Platform | OAuth | Cookie Auth | CLI | MCP Tools |
|----------|:-----:|:-----------:|:---:|:---------:|
| X / Twitter | ✅ | ✅ (GraphQL) | ✅ | 27 |
| Reddit | ✅ | ✅ (www + modhash) | ✅ | 23 |
| LinkedIn (Personal) | ✅ | — | ✅ | 15 |
| LinkedIn (Page) | ✅ | — | ✅ | 11 |
| Facebook | ✅ | — | ✅ | 15 |
| Instagram | ✅ | — | ✅ | 21 |
| Instagram (Standalone) | ✅ | — | — | 7 |
| Threads | ✅ | — | — | 9 |
| YouTube | ✅ | — | — | 2 |
| TikTok | ✅ | — | — | 4 |
| Pinterest | ✅ | — | — | 8 |
| Bluesky | App Password | — | — | 7 |

#### Tier-2 — publish-maintained (14)

| Platform | OAuth | Cookie Auth | CLI | MCP Tools |
|----------|:-----:|:-----------:|:---:|:---------:|
| Discord | ✅ | — | — | 10 |
| Slack | ✅ | — | — | 4 |
| Telegram (Bot) | Token | — | — | 10 |
| Telegram (User) | Session | — | — | 7 |
| WhatsApp | QR | — | — | 9 |
| Mastodon | ✅ | — | — | 6 |
| WordPress | ✅ | — | — | 4 |
| Medium | API Key | — | — | 3 |
| Dev.to | API Key | — | — | 3 |
| Hashnode | API Key | — | — | 3 |
| GitHub | PAT | — | — | 18 |
| Google (Gmail/Calendar/Drive) | ✅ | — | ✅ | 28 |
| Google My Business | ✅ | — | — | 0 |
| Skool | ✅ (Chrome ext) | — | — | 5 |

**26 platforms by default.** `kick`, `vk`, `whop` and `lemmy` were removed outright (v25 §1).

#### Tier-3 — archive (1)

| Platform | Enabled by | Notes |
|----------|-----------|-------|
| Farcaster | `ENABLE_ARCHIVE_PROVIDERS=1` | Web3/Neynar. Code kept under `src/social/archive/`; absent from `/api/providers` and the MCP tool list by default. |

*Mastodon custom instances use `MASTODON_INSTANCE_URL` — no separate provider. TikTok business accounts publish through the same Content Posting API as creator accounts — no separate provider.*

*Google My Business has no MCP tools of its own — it is reachable through the REST API and the channels UI. Per-platform tool counts are the `#[tool(…)]` names in `src/mcp/`; the total is the `#[tool(` count across that directory.*

*Platform list verified against `src/social/tier.rs` + `src/social/registry.rs` (26 default + 1 archived) and the `src/mcp/tools_*.rs` handler modules (42 modules).*

---

## Project Structure

```
switchpost/
├── src/
│   ├── main.rs              # Entry point, CLI dispatch
│   ├── cli/                 # CLI subcommands (clap, 56 commands)
│   │   ├── mod.rs           # Command definitions
│   │   ├── run.rs           # Cross-platform handler implementations
│   │   └── platforms/       # 31 thin shims (call shared MCP handlers)
│   ├── api/                 # REST API (axum routes)
│   │   ├── mod.rs           # Router + AppState
│   │   ├── onboard.rs       # OAuth flows + cookie forms
│   │   └── integrations.rs  # CRUD for connected accounts
│   ├── mcp/                 # MCP server (389 tools, 44 modules)
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
│   │   ├── tier.rs           # Tier-1/2/3 membership (v25 §1)
│   │   └── ...              # 26 providers (+ archive/ behind flag)
│   ├── db/                  # Database (sqlx, migrations)
│   ├── scheduler/           # Background post publisher
│   └── config.rs            # Environment configuration
├── frontend/                # SvelteKit dashboard
│   ├── src/
│   │   ├── routes/          # Pages (posts, setup, settings)
│   │   ├── lib/             # Components, API client
│   │   └── app.html         # HTML shell
│   └── package.json
├── Dockerfile               # Multi-stage: downloads pre-built binary
├── .env.example             # All config variables documented
└── Cargo.toml               # Rust dependencies
```

Planning, audit, and decision records are indexed in [`docs/planning/README.md`](docs/planning/README.md).

---
