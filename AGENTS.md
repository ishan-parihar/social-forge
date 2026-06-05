# AGENTS.md — AI Agent Knowledge Base

## Quick Reference

### CLI Usage (preferred for AI agents)
```bash
social-forge --help              # Discover all commands
social-forge providers           # List connected accounts
social-forge x timeline          # Read X timeline
social-forge reddit browse rust  # Browse subreddit
social-forge linkedin profile    # Get LinkedIn profile
```

### MCP Usage (for Claude Desktop / Cursor)
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

### Authentication Priority
1. DB-stored cookie tokens (submitted via web form)
2. Browser cookie extraction (auto-detected from Chrome/Brave/Firefox/Zen)
3. OAuth tokens from DB
4. Environment variables

### Database
- **Connection**: `DATABASE_URL` env var (PostgreSQL)
- **Port**: `5433` (local Docker postgres maps 5433→5432)
- **User/DB**: `social_forge` / `social_forge`
- **Migrations**: `migrations/` directory, auto-applied on startup

### Key Patterns
- All CLI output is JSON (machine-readable)
- Errors output JSON to stderr with exit code 1
- Cookie auth enables full platform access (voting, moderation, GraphQL)
- OAuth is the fallback for platforms without cookie support

---

## Development & Deployment

### Architecture
```
System Boot
  ├── docker.service (enabled)
  │    └── postgres container (restart: unless-stopped, port 5433)
  └── social-forge.service (enabled)
       └── /usr/local/bin/social-forge serve  (pre-built binary, NOT built on boot)
```

- **Postgres** runs as a Docker container with `restart: unless-stopped` — auto-starts on boot
- **App binary** runs via systemd — pre-built binary at `/usr/local/bin/social-forge`
- **Frontend** is built separately with `pnpm build`, served by the Rust binary
- **NO cargo build on boot** — the binary must be pre-built before deployment

### Quick redeploy (after code changes) — THE ONE-LINER
```bash
cargo build --release && sudo install -m 755 target/release/social-forge /usr/local/bin/social-forge && sudo systemctl restart social-forge
```

### Or use the Makefile
```bash
make redeploy    # same as the one-liner above
make deploy      # also rebuilds frontend
```

### Auto-watch (auto-rebuild + restart on file changes)
```bash
# Requires cargo-watch:
#   cargo install cargo-watch
make watch
```

### Service management
```bash
make status     # Show systemd + docker status
make logs       # Tail journalctl logs
make restart    # Reload systemd + restart service
```

### Manual steps (first-time setup)
```bash
# 1. Copy the systemd service & startup script
sudo cp scripts/social-forge-start.sh /usr/local/bin/social-forge-start.sh
sudo cp scripts/social-forge.service /etc/systemd/system/social-forge.service
sudo systemctl daemon-reload
sudo systemctl enable social-forge --now

# 2. Copy the pre-built binary
sudo install -m 755 target/release/social-forge /usr/local/bin/social-forge

# 3. Ensure Docker starts on boot
sudo systemctl enable docker --now
```
