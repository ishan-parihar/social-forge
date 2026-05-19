# ─── Social Forge Docker Image ───────────────────────────────
# Fetches pre-built static binary from GitHub releases.
# No Rust compilation needed — deploys in seconds on any VPS.
#
# Build: docker build --build-arg VERSION=v0.1.0 -t social-forge .
# Run:   docker compose up -d

ARG VERSION=latest

# ─── Frontend Build ──────────────────────────────────────────
FROM node:20-slim AS frontend

WORKDIR /app/frontend
COPY frontend/package.json frontend/pnpm-lock.yaml* ./
RUN npm install -g pnpm && pnpm install --frozen-lockfile 2>/dev/null || pnpm install
COPY frontend/ ./
RUN pnpm build

# ─── Runtime ─────────────────────────────────────────────────
FROM debian:bookworm-slim

ARG VERSION
ARG TARGETARCH

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

# Download pre-built static binary from GitHub releases
RUN set -eux; \
    ARCH=$(case "${TARGETARCH}" in \
      amd64) echo "x64" ;; \
      arm64) echo "arm64" ;; \
      *) echo "x64" ;; \
    esac); \
    if [ "${VERSION}" = "latest" ]; then \
      URL="https://github.com/ishan-parihar/social-forge/releases/latest/download/social-forge-linux-${ARCH}"; \
    else \
      URL="https://github.com/ishan-parihar/social-forge/releases/download/${VERSION}/social-forge-linux-${ARCH}"; \
    fi; \
    curl -fsSL "${URL}" -o /usr/local/bin/social-forge && \
    chmod +x /usr/local/bin/social-forge

COPY --from=frontend /app/frontend/build /app/frontend/build
COPY migrations/ /app/migrations/

WORKDIR /app
RUN mkdir -p /app/uploads

EXPOSE 3444
HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -sf http://localhost:3444/health || exit 1

VOLUME ["/app/uploads"]

CMD ["social-forge", "serve"]
