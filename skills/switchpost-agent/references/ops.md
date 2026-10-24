# Ops — install, daemon, config, connect

Loaded on demand from SKILL.md. All commands verified against the real CLI.

## Connect a provider

```bash
switchpost connect <provider>              # prints auth URL (OAuth) or imports cookies (X/Reddit)
switchpost connect <provider> --wait       # OAuth: prints auth_url, polls until the login lands (agent loops: run with --wait, drive a browser, exits 0 on success)
switchpost connect <provider> --browser    # drives the consent page in the operator's signed-in browser (browseros-neo); works WITHOUT the server running
switchpost connect-all                     # cookie-based providers (X, Reddit) all at once
switchpost setup                           # full guided onboarding
```

`--wait` details: prints `{status, auth_url, redirect_uri}`. If the platform
says `redirect_uri_mismatch`, register the printed `redirect_uri` in the
provider's developer console. Timeout: `--timeout-secs` (default 300).
`--browser-endpoint <URL>` overrides the browseros-neo MCP endpoint
(default `http://127.0.0.1:9012/mcp`).

## The serve daemon

```bash
switchpost serve --port 6543               # HTTP+scheduler+pollers; binds 127.0.0.1
switchpost service install                 # systemd USER unit + enable-linger (no sudo)
switchpost service uninstall
switchpost doctor                          # health-check all providers
switchpost audit                           # read-only: disabled integrations, platform-name collisions, SQLite writer posture
```

**Single-writer rule**: only ONE long-lived process may own a DATABASE_URL
(the `serve` daemon). A second `serve` on the same DB fails the writer-lock
claim loudly. `mcp-call` and all other CLI commands are fine alongside the
daemon — they take the shared path and the daemon's scheduler publishes.

**CSRF/port trap**: `serve --port N` must match `APP_URL`/`FRONTEND_URL`'s
port, or every browser write 403s (`csrf_origin_mismatch`). The binary warns
at boot when they disagree. Fix with `--port` matching the env, or
`switchpost config set FRONTEND_URL https://localhost:<port>`.

## Config

```bash
switchpost config set <KEY> <VALUE>        # writes ~/.switchpost/.env (e.g. BLUESKY_HANDLE)
switchpost config get <KEY>
switchpost config list                     # redacts secrets
switchpost config reset-password           # resets the WebUI APP_PASSWORD
switchpost init                            # create ~/.switchpost/ with default .env
```

Env precedence: the repo's `.env` (CWD) wins over `~/.switchpost/.env` for
keys defined in both.

## Install

```bash
curl -fsSL https://switchpost.dev/install.sh | sh    # pre-built binary + systemd unit
# or from source: cargo build --release, then make install-skill for the agent skill
```

Key env vars (see `.env.example` for all): `DATABASE_URL` (SQLite only),
`APP_PASSWORD` (WebUI gate), `APP_URL`/`FRONTEND_URL` (must match serve port),
`TOKEN_ENCRYPTION_KEY` (AES-256-GCM token encryption at rest),
`BIND_HOST=0.0.0.0` (opt-in LAN exposure; default loopback),
`SWITCHPOST_TOOLSETS` (MCP boot toolsets).

## The agent skill itself

This skill lives in the repo at `skills/switchpost-agent/` and installs to
`~/.agents/skills/switchpost-agent/` via `make install-skill`. After editing
the repo copy, re-run `make install-skill` or the installed copy drifts.
