#!/bin/bash
# social-forge-start.sh — Start social-forge server
# Called by systemd on boot. Sources .env, ensures Postgres is up, then
# starts the pre-built binary at /usr/local/bin/social-forge.
#
# This script does NOT build from source — the binary must be pre-built
# via `make redeploy` or `cargo build --release && sudo make restart`.
set -e

APP_DIR="/home/ishanp/Documents/GitHub/MY-PROJECTS/MCP-AND-CLIS/social-forge"
ENV_FILE="$APP_DIR/.env"

export PATH="$HOME/.cargo/bin:$PATH"

# Guard: ensure .env exists
if [ ! -f "$ENV_FILE" ]; then
    echo "[social-forge] ERROR: .env not found at $ENV_FILE"
    exit 1
fi

# Source .env for runtime variables (DATABASE_URL, API keys, etc.)
set -a
source "$ENV_FILE"
set +a

# ── Guard: ensure pg_isready is available ──────────────────────
if ! command -v pg_isready &>/dev/null; then
    echo "[social-forge] ERROR: pg_isready not found. Install postgresql-client."
    exit 1
fi

# ── Ensure PostgreSQL container is running ──────────────────────
cd "$APP_DIR"
docker compose up -d postgres 2>/dev/null || true

# ── Wait for PostgreSQL to be ready ─────────────────────────────
echo "[social-forge] Waiting for PostgreSQL at localhost:5433..."
for i in $(seq 1 30); do
  if pg_isready -h localhost -p 5433 -U social_forge -d social_forge > /dev/null 2>&1; then
    echo "[social-forge] PostgreSQL is ready"
    break
  fi
  if [ "$i" -eq 30 ]; then
    echo "[social-forge] ERROR: PostgreSQL not reachable after 60s"
    exit 1
  fi
  sleep 2
done

# ── Start the pre-built binary ──────────────────────────────────
echo "[social-forge] Starting server..."
cd "$APP_DIR"
exec /usr/local/bin/social-forge serve
