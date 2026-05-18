#!/usr/bin/env bash
#
# build-tg.sh — Build vysheng/telegram-cli for social-forge-rust
# Produces binary at: tg/bin/telegram-cli
#

set -euo pipefail

TG_DIR="tg"
TG_BINARY="${TG_DIR}/bin/telegram-cli"

log() { echo "[build-tg] $*"; }
die() { echo "[build-tg] ERROR: $*" >&2; exit 1; }

# --- Pre-flight checks ---
check_cmd() {
    command -v "$1" >/dev/null 2>&1 || die "Required command not found: $1"
}

log "Checking required build tools..."
for cmd in make gcc autoconf automake libtool pkg-config; do
    check_cmd "$cmd"
done
log "All required tools found."

# --- Check tg directory ---
if [[ ! -d "$TG_DIR" ]]; then
    die "Directory '$TG_DIR' does not exist. Please clone vysheng/telegram-cli into ./tg/"
fi

# --- Init submodules ---
log "Initializing git submodules..."
cd "$TG_DIR"
git submodule update --init --recursive

# --- Configure ---
log "Running configure with flags..."
./configure --disable-openssl --disable-libconfig CFLAGS="-Wno-error"

# --- Patch Makefile ---
log "Patching Makefile (remove -Werror, add -fcommon)..."
MAKEFILE="Makefile"
if [[ ! -f "$MAKEFILE" ]]; then
    die "Makefile not found after configure. Build failed."
fi

# Remove -Werror from COMPILE_FLAGS
sed -i 's/-Werror//g' "$MAKEFILE"

# Add -fcommon to COMPILE_FLAGS if not already present
if ! grep -q '\-fcommon' "$MAKEFILE"; then
    sed -i 's/COMPILE_FLAGS =/COMPILE_FLAGS = -fcommon/g' "$MAKEFILE"
fi

# --- Build ---
log "Building with make -j$(nproc)..."
make -j"$(nproc)"

# --- Verify binary ---
if [[ ! -f "$TG_BINARY" ]]; then
    die "Build completed but binary not found at '$TG_BINARY'"
fi

log "Build successful!"
log "Binary: $(realpath "$TG_BINARY")"