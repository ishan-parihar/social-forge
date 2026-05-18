# ─── Build Stage ─────────────────────────────────────────────
FROM rust:1.85-slim-bookworm AS builder

RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache dependencies
COPY Cargo.toml Cargo.lock* ./
RUN mkdir -p src && echo "fn main() {}" > src/main.rs && \
    echo "" > src/lib.rs && \
    cargo build --release 2>/dev/null || true && \
    rm -rf src

# Build the real application
COPY src/ ./src/
COPY migrations/ ./migrations/
COPY .sqlx/ ./.sqlx/

ENV SQLX_OFFLINE=true
RUN cargo build --release && strip target/release/social-forge

# ─── Frontend Build Stage ────────────────────────────────────
FROM node:20-slim AS frontend

WORKDIR /app/frontend
COPY frontend/package.json frontend/pnpm-lock.yaml ./
RUN npm install -g pnpm && pnpm install --frozen-lockfile
COPY frontend/ ./
RUN pnpm build

# ─── Runtime Stage ───────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/social-forge /usr/local/bin/social-forge
COPY --from=frontend /app/frontend/build /app/frontend/build
COPY migrations/ /app/migrations/

WORKDIR /app
RUN mkdir -p /app/uploads

EXPOSE 3444
HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:3444/health || exit 1

VOLUME ["/app/uploads"]

CMD ["social-forge", "serve"]
