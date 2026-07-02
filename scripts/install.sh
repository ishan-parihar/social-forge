#!/usr/bin/env bash
# ─── Social Forge Installer ────────────────────────────────────────────────────
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/ishan-parihar/social-forge/main/scripts/install.sh | bash
#   # or with options:
#   INSTALL_DIR=~/my-dir SKIP_SERVICE=true bash install.sh
#
# What this script does:
#   1. Detects OS/arch, downloads the pre-built musl binary from GitHub Releases
#   2. Creates the install directory structure
#   3. Creates a .env from the embedded template (DATABASE_URL uses correct port 5433)
#   4. Downloads docker-compose.yml and migrations
#   5. Installs the systemd service (Linux only, unless SKIP_SERVICE=true)
#   6. Installs the AI agent skill (unless SKIP_SKILL=true)
#   7. Installs postgresql-client for pg_isready (unless SKIP_PG_CLIENT=true)
#
# Environment variables:
#   INSTALL_DIR      Installation directory (default: $HOME/social-forge)
#   BIN_DIR          Binary install path   (default: /usr/local/bin)
#   SKIP_SERVICE     Skip systemd service  (default: false)
#   SKIP_SKILL       Skip AI agent skill   (default: false)
#   SKIP_PG_CLIENT   Skip postgresql-client install (default: false)
#   SERVE_FRONTEND   Set to false to disable the embedded web UI (default: true)
#   VERSION          Specific tag to install (default: latest)
# ───────────────────────────────────────────────────────────────────────────────
set -euo pipefail

REPO="ishan-parihar/social-forge"
APP_NAME="social-forge"
SCRIPTS_RAW="https://raw.githubusercontent.com/${REPO}/main/scripts"
REPO_RAW="https://raw.githubusercontent.com/${REPO}/main"

# ── Colors ──────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
CYAN='\033[0;36m'; BOLD='\033[1m'; NC='\033[0m'
log()  { echo -e "  ${GREEN}✓${NC} $1"; }
warn() { echo -e "  ${YELLOW}⚠${NC}  $1"; }
err()  { echo -e "  ${RED}✗${NC}  $1" >&2; exit 1; }
info() { echo -e "  ${CYAN}→${NC} $1"; }
head() { echo -e "\n  ${BOLD}$1${NC}"; }

# ── Help ─────────────────────────────────────────────────────────────────────
if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  cat <<HELP
Social Forge Installer

Usage:
  curl -fsSL https://raw.githubusercontent.com/${REPO}/main/scripts/install.sh | bash

Environment variables:
  INSTALL_DIR     Installation directory     (default: \$HOME/social-forge)
  BIN_DIR         Binary directory           (default: /usr/local/bin)
  SKIP_SERVICE    Skip systemd service       (default: false)
  SKIP_SKILL      Skip AI agent skill        (default: false)
  SKIP_PG_CLIENT  Skip postgresql-client     (default: false)
  SERVE_FRONTEND  Disable embedded web UI    (default: true)
  VERSION         Specific version to install (default: latest)

Examples:
  # Install with custom directory
  INSTALL_DIR=/opt/social-forge bash install.sh

  # Install without systemd service (useful for testing)
  SKIP_SERVICE=true bash install.sh

  # Install API-only (no web dashboard)
  SERVE_FRONTEND=false bash install.sh

  # Install specific version
  VERSION=v0.2.21 bash install.sh
HELP
  exit 0
fi

# ── Platform detection ───────────────────────────────────────────────────────
ARCH=$(uname -m)
OS=$(uname -s | tr '[:upper:]' '[:lower:]')

case "$ARCH" in
  x86_64|amd64)  ARCH_TAG="x64"   ;;
  aarch64|arm64) ARCH_TAG="arm64"  ;;
  *) err "Unsupported architecture: $ARCH. Expected x86_64 or aarch64." ;;
esac

case "$OS" in
  linux)  ARTIFACT="${APP_NAME}-linux-${ARCH_TAG}"  ;;
  darwin) ARTIFACT="${APP_NAME}-macos-${ARCH_TAG}"  ;;
  *) err "Unsupported OS: $OS. Expected linux or darwin." ;;
esac

# ── Configuration ────────────────────────────────────────────────────────────
CURRENT_USER="$(id -un)"
INSTALL_DIR="${INSTALL_DIR:-${HOME}/social-forge}"
BIN_DIR="${BIN_DIR:-/usr/local/bin}"
SKIP_SERVICE="${SKIP_SERVICE:-false}"
SKIP_SKILL="${SKIP_SKILL:-false}"
SKIP_PG_CLIENT="${SKIP_PG_CLIENT:-false}"
SERVE_FRONTEND="${SERVE_FRONTEND:-true}"
VERSION="${VERSION:-latest}"

echo ""
echo -e "  ${BOLD}Social Forge Installer${NC}"
echo -e "  ${CYAN}──────────────────────────────────────${NC}"
info "Platform:         ${OS}/${ARCH_TAG}"
info "Install dir:      ${INSTALL_DIR}"
info "Binary dir:       ${BIN_DIR}"
info "User:             ${CURRENT_USER}"
info "Serve frontend:   ${SERVE_FRONTEND}"

# ── Pre-flight ───────────────────────────────────────────────────────────────
command -v curl &>/dev/null || err "curl is required. Install: apt install curl / brew install curl"

# Check if we need sudo for BIN_DIR
SUDO=""
if [ ! -w "${BIN_DIR}" ] 2>/dev/null || [ ! -d "${BIN_DIR}" ]; then
    if command -v sudo &>/dev/null; then
        SUDO="sudo"
    fi
fi

# ── Resolve version ──────────────────────────────────────────────────────────
head "Resolving version..."
if [ "$VERSION" = "latest" ]; then
    VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
    if [ -z "$VERSION" ]; then
        err "Could not resolve latest release tag. Check network or set VERSION=v0.x.y"
    fi
fi
log "Version: ${VERSION}"

DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARTIFACT}"

# ── Create directory structure ───────────────────────────────────────────────
head "Creating directories..."
mkdir -p \
    "${INSTALL_DIR}" \
    "${INSTALL_DIR}/data/media" \
    "${INSTALL_DIR}/data/telegram" \
    "${INSTALL_DIR}/data/whatsapp" \
    "${INSTALL_DIR}/migrations"
log "Created: ${INSTALL_DIR}"

# ── Download binary ───────────────────────────────────────────────────────────
head "Downloading binary..."
info "URL: ${DOWNLOAD_URL}"
TMP_BIN="$(mktemp)"
trap 'rm -f "${TMP_BIN}"' EXIT

HTTP_CODE=$(curl -fsSL -w '%{http_code}' -o "${TMP_BIN}" "${DOWNLOAD_URL}" 2>/dev/null || echo "000")
if [ "$HTTP_CODE" != "200" ]; then
    rm -f "${TMP_BIN}"
    err "Download failed (HTTP ${HTTP_CODE}). Check that release ${VERSION} exists: https://github.com/${REPO}/releases"
fi
chmod +x "${TMP_BIN}"
log "Downloaded ${ARTIFACT} ($(du -sh "${TMP_BIN}" | cut -f1))"

# ── Install binary ────────────────────────────────────────────────────────────
$SUDO mkdir -p "${BIN_DIR}"
$SUDO install -m 755 "${TMP_BIN}" "${BIN_DIR}/${APP_NAME}"
log "Installed: ${BIN_DIR}/${APP_NAME}"

# Quick sanity check
if ! "${BIN_DIR}/${APP_NAME}" --version &>/dev/null && \
   ! "${BIN_DIR}/${APP_NAME}" --help &>/dev/null; then
    warn "Binary installed but couldn't run --version (may be a cross-arch issue or expected)"
fi

# ── Install postgresql-client (for pg_isready in start script) ───────────────
if [ "$OS" = "linux" ] && [ "$SKIP_PG_CLIENT" != "true" ]; then
    if ! command -v pg_isready &>/dev/null; then
        head "Installing postgresql-client (for pg_isready)..."
        if command -v apt-get &>/dev/null; then
            $SUDO apt-get update -qq && $SUDO apt-get install -y -qq postgresql-client 2>/dev/null && \
                log "postgresql-client installed" || \
                warn "Could not install postgresql-client — TCP fallback will be used"
        elif command -v yum &>/dev/null; then
            $SUDO yum install -y -q postgresql &>/dev/null && \
                log "postgresql installed" || \
                warn "Could not install postgresql — TCP fallback will be used"
        else
            warn "Could not detect package manager — install postgresql-client manually for pg_isready"
        fi
    else
        log "pg_isready already available"
    fi
fi

# ── Create .env from template ─────────────────────────────────────────────────
head "Configuration..."
if [ ! -f "${INSTALL_DIR}/.env" ]; then
    # Note: DATABASE_URL uses port 5433 — the host-mapped port from docker-compose.yml
    # (Docker maps host:5433 → container:5432)
    SERVE_FRONTEND_LINE=""
    if [ "$SERVE_FRONTEND" = "false" ]; then
        SERVE_FRONTEND_LINE="SERVE_FRONTEND=false"
    fi

    cat > "${INSTALL_DIR}/.env" <<ENVEOF
# ─── Social Forge Configuration ────────────────────────────────────────────
# Generated by install.sh on $(date -u +"%Y-%m-%dT%H:%M:%SZ")
# Edit this file to add your platform API credentials.

# ── Database ─────────────────────────────────────────────────────────────────
# Port 5433 = host-side Docker port mapping (container runs on 5432 internally)
DATABASE_URL=postgres://social_forge:social_forge@localhost:5433/social_forge

# ── Server ───────────────────────────────────────────────────────────────────
# Your public-facing URL. Used for OAuth callback URIs.
# Meta platforms (Instagram, Threads) REQUIRE https://.
# The server auto-generates a self-signed TLS cert on first start.
APP_URL=https://localhost:6543

# Optionally set a strong secret (auto-generated in dev if missing)
# JWT_SECRET=

# 64 hex chars — encrypts OAuth tokens at rest (optional but recommended)
# TOKEN_ENCRYPTION_KEY=

# ── Frontend (embedded web UI) ────────────────────────────────────────────────
# Set to false to run in API-only mode (no web dashboard). Saves ~5 MB memory.
${SERVE_FRONTEND_LINE:-# SERVE_FRONTEND=true}

# ── Platform Credentials ─────────────────────────────────────────────────────
# Uncomment and fill in as needed. See README for full docs.

# X / Twitter (cookie auth recommended — enables full GraphQL API)
# X_CT0=
# X_CLIENT_ID=

# Reddit (cookie auth recommended — enables voting, moderation)
# REDDIT_CLIENT_ID=
# REDDIT_USERNAME=

# LinkedIn
# LINKEDIN_CLIENT_ID=

# Facebook / Instagram / Threads (Meta)
# FACEBOOK_CLIENT_ID=
# INSTAGRAM_APP_ID=
# THREADS_APP_ID=

# YouTube / Google
# YOUTUBE_CLIENT_ID=

# TikTok
# TIKTOK_CLIENT_ID=

# Pinterest
# PINTEREST_CLIENT_ID=

# Discord
# DISCORD_CLIENT_ID=

# Slack
# SLACK_CLIENT_ID=

# Telegram (Bot — comma-separated tokens for multi-bot)
# TELEGRAM_BOT_TOKENS=

# Telegram (User client — Grammers MTProto)
# TELEGRAM_API_ID=
# TELEGRAM_API_HASH=
# TELEGRAM_SESSION_DIR=./data/telegram

# WhatsApp Web
# WHATSAPP_STORE_DIR=./data/whatsapp

# Bluesky
# BLUESKY_HANDLE=
# BLUESKY_APP_PASSWORD=

# Mastodon
# MASTODON_CLIENT_ID=
# MASTODON_INSTANCE_URL=

# Medium / Dev.to / Hashnode / GitHub (API key providers)
# MEDIUM_ACCESS_TOKEN=
# DEVTO_API_KEY=
# HASHNODE_API_KEY=
# GITHUB_TOKEN=
ENVEOF
    log "Created: ${INSTALL_DIR}/.env"
    warn "Edit ${INSTALL_DIR}/.env with your platform credentials before starting"
else
    warn "Skipped .env — already exists at ${INSTALL_DIR}/.env"
fi

# ── Download docker-compose.yml ───────────────────────────────────────────────
if [ ! -f "${INSTALL_DIR}/docker-compose.yml" ]; then
    curl -fsSL -o "${INSTALL_DIR}/docker-compose.yml" \
        "${REPO_RAW}/docker-compose.yml" 2>/dev/null && \
        log "Downloaded docker-compose.yml" || \
        warn "Failed to download docker-compose.yml — download it manually from https://github.com/${REPO}"
fi

# ── Download startup script ───────────────────────────────────────────────────
if [ "$OS" = "linux" ]; then
    curl -fsSL -o "${INSTALL_DIR}/social-forge-start.sh" \
        "${SCRIPTS_RAW}/social-forge-start.sh" 2>/dev/null && \
        chmod +x "${INSTALL_DIR}/social-forge-start.sh" && \
        log "Downloaded social-forge-start.sh" || \
        warn "Failed to download start script"
fi

# ── Install systemd service (Linux only) ──────────────────────────────────────
if [ "$OS" = "linux" ] && [ "$SKIP_SERVICE" != "true" ]; then
    head "Installing systemd service..."

    SERVICE_SRC="${SCRIPTS_RAW}/social-forge.service"
    SERVICE_DST="/etc/systemd/system/${APP_NAME}.service"
    START_DST="/usr/local/bin/social-forge-start.sh"

    # Install the startup script to /usr/local/bin
    if [ -f "${INSTALL_DIR}/social-forge-start.sh" ]; then
        $SUDO install -m 755 "${INSTALL_DIR}/social-forge-start.sh" "$START_DST" && \
            log "Installed start script to ${START_DST}"
    fi

    # Download the service template
    TMP_SVC="$(mktemp)"
    trap 'rm -f "${TMP_BIN}" "${TMP_SVC}"' EXIT
    curl -fsSL -o "$TMP_SVC" "$SERVICE_SRC" 2>/dev/null || {
        warn "Failed to download service template — skipping systemd setup"
        TMP_SVC=""
    }

    if [ -n "$TMP_SVC" ] && [ -s "$TMP_SVC" ]; then
        # Fill in template placeholders
        sed -i \
            -e "s|%%USER%%|${CURRENT_USER}|g" \
            -e "s|%%GROUP%%|${CURRENT_USER}|g" \
            -e "s|%%INSTALL_DIR%%|${INSTALL_DIR}|g" \
            "$TMP_SVC"

        if [ -f "$SERVICE_DST" ]; then
            warn "Service already exists at ${SERVICE_DST} — backing up and replacing"
            $SUDO cp "$SERVICE_DST" "${SERVICE_DST}.bak.$(date +%s)"
        fi

        $SUDO install -m 644 "$TMP_SVC" "$SERVICE_DST"
        $SUDO systemctl daemon-reload 2>/dev/null || true
        log "Service installed: ${SERVICE_DST}"

        echo ""
        echo -e "  ${CYAN}To enable and start the service:${NC}"
        echo "    sudo systemctl enable ${APP_NAME} --now"
        echo ""
        echo -e "  ${CYAN}To view logs:${NC}"
        echo "    sudo journalctl -u ${APP_NAME} -f"
    fi
fi

# ── Install AI Agent skill ─────────────────────────────────────────────────────
if [ "$SKIP_SKILL" != "true" ]; then
    head "Installing AI Agent skill..."
    SKILL_DIR="${HOME}/.agents/skills/social-forge-agent"
    mkdir -p "${SKILL_DIR}/references"

    SKILL_OK=false
    curl -fsSL -o "${SKILL_DIR}/SKILL.md" \
        "${REPO_RAW}/skills/social-forge-agent/SKILL.md" 2>/dev/null && SKILL_OK=true
    curl -fsSL -o "${SKILL_DIR}/references/providers.md" \
        "${REPO_RAW}/skills/social-forge-agent/references/providers.md" 2>/dev/null || true

    $SKILL_OK && log "AI skill installed: ${SKILL_DIR}" || \
        warn "Failed to download AI agent skill (non-critical)"
fi

# ── Done ──────────────────────────────────────────────────────────────────────
echo ""
echo -e "  ${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "  ${GREEN}  ✓  Social Forge ${VERSION} installed!${NC}"
echo -e "  ${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "  Binary:     ${BIN_DIR}/${APP_NAME}"
echo "  Config:     ${INSTALL_DIR}/.env"
echo "  Data dir:   ${INSTALL_DIR}/data/"
echo ""
echo -e "  ${BOLD}Next steps:${NC}"
echo ""
echo "  1. Edit your config:"
echo "       nano ${INSTALL_DIR}/.env"
echo ""
echo "  2. Start PostgreSQL (Docker):"
echo "       cd ${INSTALL_DIR} && docker compose up -d postgres"
echo ""
if [ "$OS" = "linux" ] && [ "$SKIP_SERVICE" != "true" ]; then
  echo "  3. Start the service:"
  echo "       sudo systemctl enable ${APP_NAME} --now"
  echo ""
  echo "  4. Open the dashboard:"
  echo "       https://localhost:6543"
else
  echo "  3. Start the server:"
  echo "       ${BIN_DIR}/${APP_NAME} serve"
  echo ""
  echo "  4. Open the dashboard:"
  echo "       https://localhost:6543"
fi
echo ""
echo "  Docs: https://github.com/${REPO}"
echo ""
