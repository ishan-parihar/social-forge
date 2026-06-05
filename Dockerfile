# ─── Social Forge Docker Image ───────────────────────────────
# Multi-stage build: Rust binary from local source + frontend.
#
# Build: docker compose build
# Run:   docker compose up -d

# ─── Rust Build ─────────────────────────────────────────────
FROM rust:1.94-bookworm AS backend

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
      cmake \
      pkg-config \
      libssl-dev \
      libclang-dev \
      libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY .sqlx/ ./.sqlx/
COPY migrations/ ./migrations/
COPY src/ ./src/

ENV SQLX_OFFLINE=true
RUN cargo build --release --locked

# ─── Frontend Build ──────────────────────────────────────────
FROM node:20-slim AS frontend

WORKDIR /app/frontend
COPY frontend/package.json frontend/pnpm-lock.yaml* ./
RUN npm install -g pnpm && pnpm install --frozen-lockfile 2>/dev/null || pnpm install
COPY frontend/ ./
RUN pnpm build

# ─── Runtime ─────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

COPY --from=backend /app/target/release/social-forge /usr/local/bin/social-forge
COPY --from=frontend /app/frontend/build /app/frontend/build
COPY migrations/ /app/migrations/

WORKDIR /app
RUN mkdir -p /app/uploads

EXPOSE 6543
HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -sf http://localhost:6543/health || exit 1

VOLUME ["/app/uploads"]

CMD ["social-forge", "serve"]
