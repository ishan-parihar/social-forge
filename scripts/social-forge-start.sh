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

# ── 3. Install AI Agent Skill (optional, best-effort) ──────────────────────
SKILL_SRC="${INSTALL_DIR}/skills/social-forge-agent"
SKILL_DEST="${HOME}/.agents/skills/social-forge-agent"
if [ -d "$SKILL_SRC" ]; then
    mkdir -p "${SKILL_DEST}/references"
    cp "${SKILL_SRC}/SKILL.md" "${SKILL_DEST}/SKILL.md" 2>/dev/null || true
    cp "${SKILL_SRC}/references/providers.md" "${SKILL_DEST}/references/providers.md" 2>/dev/null || true
    echo "$TAG Skill installed to $SKILL_DEST"
fi

# ── 4. Launch binary ─────────────────────────────────────────────────────────
# Resolve binary location — checks common install locations in order.
# Change to INSTALL_DIR so relative paths (data/, etc.) resolve correctly.
# Note: migrations are embedded in the binary at compile time via
# sqlx::migrate!("./migrations") and applied automatically on startup.
# No manual SQL tooling or migration scripting is needed.
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
