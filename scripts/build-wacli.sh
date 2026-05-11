#!/bin/bash
set -e
cd "$(dirname "$0")/../wacli"
echo "Building wacli (WhatsApp CLI) with server mode..."
CGO_ENABLED=1 CGO_CFLAGS="-Wno-error=missing-braces" \
  go build -tags sqlite_fts5 -o "$PWD/dist/wacli" ./cmd/wacli
echo "Built wacli at $PWD/dist/wacli"
echo "  -> server mode: wacli server (JSON-RPC over stdin/stdout)"
