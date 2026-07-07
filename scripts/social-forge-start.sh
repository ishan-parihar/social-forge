#!/bin/bash
# ─── social-forge-start.sh ─────────────────────────────────────────────────
# Start script for the social-forge systemd service.
# Fully path-agnostic — works on any system regardless of username or install dir.
#
# Path resolution order for INSTALL_DIR:
#   1. SOCIAL_FORGE_DIR env var (set in the systemd service's Environment= line)
#   2. ~/.social-forge/ (XDG-style user config)
#   3. ~/social-forge/  (install.sh default)
#   4. /opt/social-forge/ (system-wide install)
#
# This script does NOT build from source — the binary at /usr/local/bin/social-forge
# must be pre-installed (e.g. via install.sh or a GitHub Releases download).
# ───────────────────────────────────────────────────────────────────────────────
set -e

TAG="[social-forge]"

# ── 1. Resolve INSTALL_DIR ──────────────────────────────────────────────────
INSTALL_DIR="${SOCIAL_FORGE_DIR:-}"

if [ -z "$INSTALL_DIR" ]; then
    # Try common locations in priority order
    for candidate in \
        "${HOME}/.social-forge" \
        "${HOME}/social-forge" \
        "/opt/social-forge"; do
        if [ -f "${candidate}/.env" ]; then
            INSTALL_DIR="$candidate"
            break
        fi
    done
fi

if [ -z "$INSTALL_DIR" ]; then
    echo "$TAG ERROR: Could not find .env in any of: ~/.social-forge, ~/social-forge, /opt/social-forge"
    echo "$TAG Set SOCIAL_FORGE_DIR=/path/to/install in the systemd service Environment= or export it."
    exit 1
fi

ENV_FILE="${INSTALL_DIR}/.env"

echo "$TAG Using install dir: $INSTALL_DIR"

# ── 2. Source .env ───────────────────────────────────────────────────────────
if [ ! -f "$ENV_FILE" ]; then
    echo "$TAG ERROR: .env not found at $ENV_FILE"
    exit 1
fi

set -a
# shellcheck source=/dev/null
source "$ENV_FILE"
set +a

# ── 3. Resolve Docker compose file ──────────────────────────────────────────
COMPOSE_FILE="${INSTALL_DIR}/docker-compose.yml"

# ── 4. Ensure Docker (if available) starts Postgres ─────────────────────────
if command -v docker &>/dev/null && [ -f "$COMPOSE_FILE" ]; then
    echo "$TAG Starting Postgres via docker compose..."
    docker compose -f "$COMPOSE_FILE" up -d postgres 2>/dev/null || true
else
    echo "$TAG Skipping docker compose — docker not found or no compose file at $COMPOSE_FILE"
fi

# ── 5. Determine Postgres connection params from DATABASE_URL ────────────────
# DATABASE_URL format: postgres://user:pass@host:port/db
# Defaults that match the bundled docker-compose.yml
DB_HOST="localhost"
DB_PORT="5433"
DB_USER="social_forge"
DB_NAME="social_forge"

if [ -n "${DATABASE_URL:-}" ]; then
    # Extract host
    _host=$(echo "$DATABASE_URL" | sed -E 's|postgres://[^@]+@([^:/]+).*|\1|' 2>/dev/null || true)
    [ -n "$_host" ] && DB_HOST="$_host"
    # Extract port (if present)
    _port=$(echo "$DATABASE_URL" | sed -E 's|postgres://[^@]+@[^:/]+:([0-9]+)/.*|\1|' 2>/dev/null || true)
    [ -n "$_port" ] && [ "$_port" != "$DATABASE_URL" ] && DB_PORT="$_port"
    # Extract user
    _user=$(echo "$DATABASE_URL" | sed -E 's|postgres://([^:@]+)[^@]*@.*|\1|' 2>/dev/null || true)
    [ -n "$_user" ] && DB_USER="$_user"
    # Extract dbname
    _db=$(echo "$DATABASE_URL" | sed -E 's|.*/([^?]+).*|\1|' 2>/dev/null || true)
    [ -n "$_db" ] && DB_NAME="$_db"
fi

# ── 6. Wait for Postgres ─────────────────────────────────────────────────────
if command -v pg_isready &>/dev/null; then
    echo "$TAG Waiting for PostgreSQL at ${DB_HOST}:${DB_PORT}..."
    for i in $(seq 1 30); do
        if pg_isready -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" >/dev/null 2>&1; then
            echo "$TAG PostgreSQL is ready."
            break
        fi
        if [ "$i" -eq 30 ]; then
            echo "$TAG ERROR: PostgreSQL not reachable at ${DB_HOST}:${DB_PORT} after 60s"
            exit 1
        fi
        sleep 2
    done
else
    # pg_isready not available — fall back to a simple TCP check with /dev/tcp
    echo "$TAG pg_isready not found — falling back to TCP check on ${DB_HOST}:${DB_PORT}"
    for i in $(seq 1 30); do
        if (echo >/dev/tcp/"$DB_HOST"/"$DB_PORT") 2>/dev/null; then
            echo "$TAG PostgreSQL port ${DB_PORT} is open."
            break
        fi
        if [ "$i" -eq 30 ]; then
            echo "$TAG ERROR: TCP check on ${DB_HOST}:${DB_PORT} timed out after 60s"
            exit 1
        fi
        sleep 2
    done
fi

# ── 7. Install AI Agent Skill (optional, best-effort) ───────────────────────
SKILL_SRC="${INSTALL_DIR}/skills/social-forge-agent"
SKILL_DEST="${HOME}/.agents/skills/social-forge-agent"
if [ -d "$SKILL_SRC" ]; then
    mkdir -p "${SKILL_DEST}/references"
    cp "${SKILL_SRC}/SKILL.md" "${SKILL_DEST}/SKILL.md" 2>/dev/null || true
    cp "${SKILL_SRC}/references/providers.md" "${SKILL_DEST}/references/providers.md" 2>/dev/null || true
    echo "$TAG Skill installed to $SKILL_DEST"
fi

# ── 8. Launch binary ─────────────────────────────────────────────────────────
# Resolve binary location — checks common install locations in order.
# Change to INSTALL_DIR so relative paths (data/, etc.) resolve correctly.
# Note: migrations are embedded in the binary at compile time via
# sqlx::migrate!("./migrations") and applied automatically on startup.
# No manual psql or migration scripting is needed.
echo "$TAG Starting social-forge serve..."
cd "$INSTALL_DIR"

# Priority: ~/.local/bin → /usr/local/bin → PATH
if [ -x "$HOME/.local/bin/social-forge" ]; then
    BINARY="$HOME/.local/bin/social-forge"
elif [ -x "/usr/local/bin/social-forge" ]; then
    BINARY="/usr/local/bin/social-forge"
else
    BINARY=$(command -v social-forge 2>/dev/null || true)
fi

if [ -z "$BINARY" ] || [ ! -x "$BINARY" ]; then
    echo "$TAG ERROR: social-forge binary not found. Install it to ~/.local/bin or /usr/local/bin."
    exit 1
fi

exec "$BINARY" serve
