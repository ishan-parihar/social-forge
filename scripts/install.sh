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
#   3. Creates a .env from the embedded template (SQLite DATABASE_URL)
#   4. Symlinks ~/.social-forge/.env at that file, so there is ONE config
#   5. Interactively prompts for APP_PASSWORD / APP_URL / TOKEN_ENCRYPTION_KEY
#      / JWT_SECRET (skipped when stdin is not a TTY or NONINTERACTIVE=true)
#   6. Onboards platforms through the CLI (config list, providers,
#      connect <platform>, setup) — never `doctor`, which makes live API calls
#   7. Downloads the startup script
#   8. Installs the systemd service (Linux only, unless SKIP_SERVICE=true)
#   9. Installs the AI agent skill (unless SKIP_SKILL=true)
#
# Environment variables:
#   INSTALL_DIR      Installation directory (default: $HOME/social-forge)
#   BIN_DIR          Binary install path   (default: /usr/local/bin)
#   SKIP_SERVICE     Skip systemd service  (default: false)
#   SKIP_SKILL       Skip AI agent skill   (default: false)
#   SKIP_ONBOARD     Skip the post-install CLI onboarding step (default: false)
#   NONINTERACTIVE   Never prompt (default: false; forced on when stdin is not
#                    a TTY, which is the case for `curl … | bash`)
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
  SKIP_ONBOARD    Skip the CLI onboarding step (default: false)
  NONINTERACTIVE  Never prompt for secrets   (default: false, auto-on when
                                               stdin is not a TTY)
  SERVE_FRONTEND  Disable embedded web UI    (default: true)
  VERSION         Specific version to install (default: latest)

Examples:
  # Install with custom directory
  INSTALL_DIR=/opt/social-forge bash install.sh

  # Install without systemd service (useful for testing)
  SKIP_SERVICE=true bash install.sh

  # Install API-only (no web dashboard)
  SERVE_FRONTEND=false bash install.sh

  # Fully unattended (CI / provisioning): no prompts, no onboarding
  NONINTERACTIVE=true SKIP_ONBOARD=true bash install.sh

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
SKIP_ONBOARD="${SKIP_ONBOARD:-false}"
SERVE_FRONTEND="${SERVE_FRONTEND:-true}"
VERSION="${VERSION:-latest}"

# Prompts need a terminal on stdin. `curl … | bash` hands us a pipe, so a piped
# install is non-interactive by construction; NONINTERACTIVE=true forces the
# same behaviour on a real terminal (CI, box provisioning, -y installs).
IS_TTY=false
if [ -t 0 ]; then IS_TTY=true; fi
NONINTERACTIVE="${NONINTERACTIVE:-false}"
if [ "$IS_TTY" != "true" ] || [ "$NONINTERACTIVE" = "true" ]; then
    NONINTERACTIVE=true
fi

ENV_FILE="${INSTALL_DIR}/.env"
SF_BIN="${BIN_DIR}/${APP_NAME}"

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
    # `head` is shadowed by the log helper above, so use sed to take line 1.
    VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' | sed -n 1p | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
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
    "${INSTALL_DIR}/data/whatsapp"
log "Created: ${INSTALL_DIR}"
# Absolute path from here on: the .env bakes it into DATABASE_URL and the
# onboarding step cd's into it, so a relative INSTALL_DIR would rot.
INSTALL_DIR="$(cd "${INSTALL_DIR}" && pwd)"
ENV_FILE="${INSTALL_DIR}/.env"
USER_ENV="${HOME}/.social-forge/.env"
# Note: migrations are embedded in the binary at compile time via
# sqlx::migrate!("./migrations"). No migration files need to be
# downloaded — the binary applies them automatically on startup.

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

# ── Create .env from template ─────────────────────────────────────────────────
head "Configuration..."
if [ ! -f "${INSTALL_DIR}/.env" ]; then
    SERVE_FRONTEND_LINE=""
    if [ "$SERVE_FRONTEND" = "false" ]; then
        SERVE_FRONTEND_LINE="SERVE_FRONTEND=false"
    fi

    cat > "${INSTALL_DIR}/.env" <<ENVEOF
# ─── Social Forge Configuration ────────────────────────────────────────────
# Generated by install.sh on $(date -u +"%Y-%m-%dT%H:%M:%SZ")
# Edit this file to add your platform API credentials.
#
# This is the ONLY config file: install.sh symlinks ~/.social-forge/.env here,
# and every `social-forge config set` write lands in this file too.

# ── Database ─────────────────────────────────────────────────────────────────
# SQLite file, created and migrated automatically on first run.
DATABASE_URL=sqlite://${INSTALL_DIR}/data/social-forge.db?mode=rwc

# ── Server ───────────────────────────────────────────────────────────────────
# Your public-facing URL. Used for OAuth callback URIs.
# Meta platforms (Instagram, Threads) REQUIRE https://.
# The server auto-generates a self-signed TLS cert on first start.
APP_URL=https://localhost:6543

# Single-user WebUI password. Left unset, the binary generates one on first
# run and persists it here. Re-roll it with:
#   social-forge config reset-password
# APP_PASSWORD=

# 64 hex chars — encrypts OAuth tokens at rest (optional but recommended)
# TOKEN_ENCRYPTION_KEY=

# Optionally set a strong secret (auto-derived from APP_PASSWORD if missing)
# JWT_SECRET=

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
# Instagram (Graph API) reads the CLIENT_ID pair; it is normally the same
# Meta app id/secret as FACEBOOK_CLIENT_ID/SECRET.
# INSTAGRAM_CLIENT_ID=
# INSTAGRAM_CLIENT_SECRET=
# Instagram (Standalone) reads the APP_ID pair.
# INSTAGRAM_APP_ID=
# INSTAGRAM_APP_SECRET=
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
    chmod 600 "${INSTALL_DIR}/.env"
    warn "Edit ${INSTALL_DIR}/.env with your platform credentials before starting"
else
    warn "Skipped .env — already exists at ${INSTALL_DIR}/.env"
fi

# ── One config file ──────────────────────────────────────────────────────────
# The binary reads $CWD/.env and then ~/.social-forge/.env, while
# `social-forge config set` only ever writes ~/.social-forge/.env. Two real
# files would silently split your credentials, so ~/.social-forge/.env becomes
# a symlink to the install file: both readers see the same bytes and every CLI
# write lands in the file the systemd service already sources.
info "Linking ${USER_ENV} → ${ENV_FILE}"
mkdir -p "$(dirname "${USER_ENV}")"
if [ -L "${USER_ENV}" ]; then
    if [ "$(readlink "${USER_ENV}")" != "${ENV_FILE}" ]; then
        rm -f "${USER_ENV}"
        ln -s "${ENV_FILE}" "${USER_ENV}"
    fi
    log "Symlink in place"
elif [ -e "${USER_ENV}" ]; then
    warn "${USER_ENV} already exists as a regular file — left untouched"
    warn "Two configs would split credentials. Unify them with:"
    warn "  mv ${USER_ENV} ${USER_ENV}.bak && ln -s ${ENV_FILE} ${USER_ENV}"
else
    ln -s "${ENV_FILE}" "${USER_ENV}"
    log "Symlink created"
fi

# ── .env helpers ─────────────────────────────────────────────────────────────
# env_get: current value of KEY in the install .env ("" when unset/commented).
env_get() {
    [ -f "${ENV_FILE}" ] || return 0
    sed -n "s/^[[:space:]]*$1=//p" "${ENV_FILE}" | sed -n 1p
}

# env_write: replace-or-append KEY=VALUE. The value is a plain shell word that
# is never exec'd, so it stays out of `ps` and shell history; the result is
# piped into the file with `cat` (not `mv`) so an existing
# ~/.social-forge/.env symlink keeps pointing at the install file.
env_write() {
    local key="$1" val="$2" tmp
    tmp="$(mktemp)"
    if [ -f "${ENV_FILE}" ]; then
        awk -v k="$key" -v v="$val" '
            !done && $0 ~ "^[[:space:]]*" k "=" { print k "=" v; done = 1; next }
            { print }
            END { if (!done) print k "=" v }
        ' "${ENV_FILE}" > "$tmp"
    else
        printf '%s=%s\n' "$key" "$val" > "$tmp"
    fi
    cat "$tmp" > "${ENV_FILE}"
    rm -f "$tmp"
    chmod 600 "${ENV_FILE}"
}

# ask_yes_no PROMPT — defaults to yes on empty input.
ask_yes_no() {
    local reply=""
    printf "  %s [Y/n] " "$1"
    read -r reply || return 1
    case "$reply" in
        [nN]*) return 1 ;;
        *) return 0 ;;
    esac
}

# prompt_secret KEY PROMPT [MIN_LEN] — hidden input, written via env_write.
prompt_secret() {
    local key="$1" prompt="$2" min="${3:-1}" val=""
    while :; do
        printf "  %s " "$prompt"
        read -rs val || return 1
        printf "\n"
        if [ "${#val}" -ge "$min" ]; then
            env_write "$key" "$val"
            log "$key saved to ${ENV_FILE}"
            return 0
        fi
        warn "Must be at least ${min} characters."
    done
}

# 32 random bytes rendered as 64 hex chars.
gen_hex32() {
    if command -v openssl &>/dev/null; then
        openssl rand -hex 32
    else
        od -An -tx1 -N32 /dev/urandom | tr -d ' \n'
    fi
}

# ── Interactive configuration ────────────────────────────────────────────────
# Only reached on a TTY, and never overwrites a value that is already set —
# pre-exported env vars, an existing .env, or a re-run all win.
if [ "$NONINTERACTIVE" = "true" ]; then
    head "Configuration"
    info "Non-interactive install — skipping prompts"
    info "Set values later with: ${SF_BIN} config set KEY VALUE"
else
    head "Configuration prompts"

    # ── APP_PASSWORD ─────────────────────────────────────────
    if [ -n "$(env_get APP_PASSWORD)" ]; then
        log "APP_PASSWORD already set — keeping"
    elif ask_yes_no "Generate a random WebUI password now? (${SF_BIN} config reset-password)"; then
        "${SF_BIN}" config reset-password 2>&1 | sed 's/^/    /' || \
            warn "config reset-password failed — set APP_PASSWORD in ${ENV_FILE} by hand"
    else
        prompt_secret APP_PASSWORD "WebUI password (min 8 chars, hidden):" 8
    fi

    # ── APP_URL ──────────────────────────────────────────────
    # Validated because every OAuth redirect URI and the /setup link is built
    # from it — a typo here silently breaks authorization.
    current_url="$(env_get APP_URL)"
    [ -n "$current_url" ] || current_url="https://localhost:6543"
    reply="$current_url"
    attempt=0
    while [ "$attempt" -lt 3 ]; do
        printf "  Public URL for OAuth redirects [%s]: " "$current_url"
        read -r reply || reply=""
        [ -n "$reply" ] || reply="$current_url"
        if printf '%s' "$reply" | grep -qE '^https?://[^[:space:]]+$'; then
            break
        fi
        warn "Must be a URL starting with http:// or https://"
        attempt=$((attempt + 1))
    done
    if [ "$attempt" -ge 3 ]; then
        reply="$current_url"
        warn "Keeping the previous value: ${current_url}"
    fi
    env_write APP_URL "$reply"
    log "APP_URL=${reply}"
    case "$reply" in
        http://*) warn "http:// — Instagram-Standalone and Threads require https://" ;;
    esac

    # ── TOKEN_ENCRYPTION_KEY ────────────────────────────────
    if [ -n "$(env_get TOKEN_ENCRYPTION_KEY)" ]; then
        log "TOKEN_ENCRYPTION_KEY already set — keeping"
    elif ask_yes_no "Generate a 64-hex TOKEN_ENCRYPTION_KEY (encrypts tokens at rest)?"; then
        env_write TOKEN_ENCRYPTION_KEY "$(gen_hex32)"
        log "TOKEN_ENCRYPTION_KEY generated"
    else
        attempt=0
        while [ "$attempt" -lt 3 ]; do
            printf "  TOKEN_ENCRYPTION_KEY (64 hex chars, hidden): "
            read -rs val || break
            printf "\n"
            if printf '%s' "$val" | grep -qE '^[0-9a-fA-F]{64}$'; then
                env_write TOKEN_ENCRYPTION_KEY "$val"
                log "TOKEN_ENCRYPTION_KEY saved"
                break
            fi
            warn "Must be exactly 64 hex characters (32 bytes)."
            attempt=$((attempt + 1))
        done
        if [ "$attempt" -ge 3 ]; then
            warn "Skipped — provider tokens will be stored unencrypted."
        fi
    fi

    # ── JWT_SECRET (optional) ───────────────────────────────
    printf "  JWT_SECRET (optional — blank derives it from APP_PASSWORD, hidden): "
    read -rs val || val=""
    printf "\n"
    if [ -n "$val" ]; then
        env_write JWT_SECRET "$val"
        log "JWT_SECRET saved"
    else
        log "JWT_SECRET left unset — derived from APP_PASSWORD at startup"
    fi

    # ── Summary (secrets reported as set/unset, never printed) ──
    for k in APP_PASSWORD TOKEN_ENCRYPTION_KEY JWT_SECRET; do
        if [ -n "$(env_get "$k")" ]; then
            info "$k = (set)"
        else
            info "$k = (not set)"
        fi
    done
    info "Wrote: ${ENV_FILE}"
fi

# ── Onboarding ───────────────────────────────────────────────────────────────
# Only non-interactive-safe commands run here: config list, providers,
# connect <platform>, setup. `doctor` is deliberately excluded — it makes live
# API calls to every connected platform, which has no place inside an installer.
if [ "$SKIP_ONBOARD" = "true" ]; then
    info "Skipping onboarding (SKIP_ONBOARD=true)"
elif ! "${SF_BIN}" --version &>/dev/null; then
    warn "Binary will not run on this host — skipping onboarding"
else
    head "Onboarding"

    # Run from the install dir so $CWD/.env — the single config file — is read.
    cd "${INSTALL_DIR}"

    # 1. config list — proves which file the CLI actually reads.
    info "\$ ${SF_BIN} config list"
    "${SF_BIN}" config list 2>&1 | sed 's/^/    /' || warn "config list failed"

    # 2. providers. A fresh install has no users row yet (it is created by the
    #    first `serve`), and every connect/provider command needs one — report
    #    that instead of treating it as a failure.
    providers_out="$("${SF_BIN}" providers 2>&1 || true)"
    printf '%s\n' "$providers_out" | sed 's/^/    /'
    db_ready=true
    if printf '%s' "$providers_out" | grep -q "No user registered"; then
        db_ready=false
    fi

    if [ "$db_ready" = "false" ]; then
        warn "No user row yet — platform probes need one server start first."
        info "After starting the service once, run:"
        info "  ${SF_BIN} connect-all   # imports X + Reddit browser cookies"
        info "  ${SF_BIN} setup         # guided status of every platform"
    else
        # 3. Cookie-import providers (auto-reads Chrome/Brave/Firefox/Zen).
        for p in x reddit; do
            info "\$ ${SF_BIN} connect $p"
            connect_out="$("${SF_BIN}" connect "$p" 2>&1 || true)"
            printf '%s\n' "$connect_out" | sed 's/^/    /'
        done

        # 4. Env-var platforms — a status probe, never a connection. Each
        #    `not_configured` answer carries the `requires` array we surface.
        for p in bluesky github telegram-bot discord slack pinterest tiktok \
                 mastodon youtube medium devto hashnode threads; do
            connect_out="$("${SF_BIN}" connect "$p" 2>&1 || true)"
            requires="$(printf '%s\n' "$connect_out" | grep -m1 'requires\[' | sed 's/^[[:space:]]*//' || true)"
            status="$(printf '%s\n' "$connect_out" | grep -m1 '^status:' | sed 's/^status:[[:space:]]*//' || true)"
            if [ -n "$requires" ]; then
                warn "$p — $requires"
            elif [ -n "$status" ]; then
                log "$p — $status"
            else
                warn "$p — probe failed: $(printf '%s\n' "$connect_out" | sed -n 1p)"
            fi
        done
    fi

    # 5. setup — non-interactive status report for the whole instance.
    info "\$ ${SF_BIN} setup"
    setup_out="$("${SF_BIN}" setup 2>&1 || true)"
    printf '%s\n' "$setup_out" \
        | grep -E '^(status:|next_actions:)|^  (connect_all|config_set):' \
        | sed 's/^/    /' || true

    # OAuth/QR providers cannot be completed from a terminal — they need a
    # browser, so they stay a manual step.
    setup_url="$(env_get APP_URL)"
    [ -n "$setup_url" ] || setup_url="https://localhost:6543"
    echo ""
    info "OAuth / QR providers (LinkedIn, Facebook, Instagram, WhatsApp, …)"
    info "  finish them in a browser:  ${setup_url}/setup"
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
echo "  Config:     ${INSTALL_DIR}/.env  (symlinked as ${HOME}/.social-forge/.env)"
echo "  Data dir:   ${INSTALL_DIR}/data/"
echo ""
echo -e "  ${BOLD}Next steps:${NC}"
echo ""
echo "  1. Edit your config (this is the only .env — \`config set\` writes here too):"
echo "       nano ${INSTALL_DIR}/.env"
echo ""
if [ "$OS" = "linux" ] && [ "$SKIP_SERVICE" != "true" ]; then
  echo "  2. Start the service:"
  echo "       sudo systemctl enable ${APP_NAME} --now"
  echo ""
  echo "  3. Open the dashboard:"
  echo "       https://localhost:6543"
else
  echo "  2. Start the server:"
  echo "       ${BIN_DIR}/${APP_NAME} serve"
  echo ""
  echo "  3. Open the dashboard:"
  echo "       https://localhost:6543"
fi
echo ""
echo "  Docs: https://github.com/${REPO}"
echo ""

# ── Install Session Hooks (AXI §7) ─────────────────────────────────────────
if [ "$SKIP_SKILL" != "true" ]; then
    head "Installing AI agent session hooks..."

    # Claude Code session hook
    CLAUDE_SETTINGS="$HOME/.claude/settings.json"
    if command -v jq &>/dev/null && [ -f "$CLAUDE_SETTINGS" ]; then
        if jq -e '.hooks.SessionStart[]?.hooks[]?.command == "social-forge"' "$CLAUDE_SETTINGS" &>/dev/null; then
            log "Claude Code session hook already installed"
        else
            cp "$CLAUDE_SETTINGS" "${CLAUDE_SETTINGS}.bak.$(date +%s)"
            jq '.hooks.SessionStart += [{"matcher":"","hooks":[{"type":"command","command":"social-forge"}]}]' \
                "$CLAUDE_SETTINGS" > "${CLAUDE_SETTINGS}.tmp" && mv "${CLAUDE_SETTINGS}.tmp" "$CLAUDE_SETTINGS"
            log "Claude Code session hook installed"
        fi
    else
        info "Claude Code: Add to ~/.claude/settings.json:"
        echo '    {"hooks":{"SessionStart":[{"matcher":"","hooks":[{"type":"command","command":"social-forge"}]}]}}'
    fi

    # Codex session hook
    CODEX_DIR="$HOME/.codex"
    if [ -d "$CODEX_DIR" ]; then
        CODEX_HOOKS="$CODEX_DIR/hooks.json"
        if [ -f "$CODEX_HOOKS" ] && jq -e '.SessionStart == "social-forge"' "$CODEX_HOOKS" &>/dev/null; then
            log "Codex session hook already installed"
        else
            if [ -f "$CODEX_HOOKS" ]; then
                cp "$CODEX_HOOKS" "${CODEX_HOOKS}.bak.$(date +%s)"
                jq '.SessionStart = "social-forge"' "$CODEX_HOOKS" > "${CODEX_HOOKS}.tmp" && mv "${CODEX_HOOKS}.tmp" "$CODEX_HOOKS"
            else
                echo '{"SessionStart":"social-forge"}' > "$CODEX_HOOKS"
            fi
            log "Codex session hook installed"
            CODEX_CONFIG="$CODEX_DIR/config.toml"
            if [ -f "$CODEX_CONFIG" ] && ! grep -q 'hooks = true' "$CODEX_CONFIG"; then
                echo -e '\n[features]\nhooks = true' >> "$CODEX_CONFIG"
                info "Enabled hooks in $CODEX_CONFIG"
            fi
        fi
    else
        info "Codex: Create ~/.codex/hooks.json with {"SessionStart":"social-forge"}"
    fi

    # OpenCode session hook
    OPENCODE_DIR="$HOME/.config/opencode/plugins"
    if [ -d "$HOME/.config/opencode" ]; then
        mkdir -p "$OPENCODE_DIR"
        if [ -f "$OPENCODE_DIR/social-forge.ts" ]; then
            log "OpenCode session hook already installed"
        else
            cat > "$OPENCODE_DIR/social-forge.ts" << 'OPENCODE_PLUGIN'
export default {
  name: "social-forge",
  onSessionStart: async () => {
    const { execSync } = require("child_process");
    return execSync("social-forge").toString();
  },
};
OPENCODE_PLUGIN
            log "OpenCode session hook installed"
        fi
    else
        info "OpenCode: Create ~/.config/opencode/plugins/social-forge.ts (see README)"
    fi
fi
