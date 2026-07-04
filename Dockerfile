# ─── Social Forge Docker Image ───────────────────────────────
# Multi-stage build: Rust binary from local source + frontend.
#
# Build: docker compose build
# Run:   docker compose up -d

# ─── Rust Build ─────────────────────────────────────────────
# Slim base keeps the build layer small while still providing
# the C toolchain (cmake, libclang) required by boring-sys (wreq).
FROM rust:1.94-slim-bookworm AS backend

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
      cmake \
      pkg-config \
      libssl-dev \
      libclang-dev \
      libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# ── Dependency-cache layer ──────────────────────────────────
# Stub main.rs first so `cargo build` populates the registry cache
# without recompiling on every source change. Borrowed from
# Dockerfile.build (the production deploy path) which already had
# this pattern; main Dockerfile was missing it.
COPY Cargo.toml Cargo.lock ./
COPY .sqlx/ ./.sqlx/
COPY migrations/ ./migrations/
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
ENV SQLX_OFFLINE=true
RUN cargo build --release --locked || true
# Remove the stub so the real source compiles next.
RUN rm -rf src

# ── Real source ─────────────────────────────────────────────
COPY src/ ./src/
RUN touch src/main.rs && cargo build --release --locked

# ─── Frontend Build ──────────────────────────────────────────
FROM node:20-slim AS frontend

WORKDIR /app/frontend
COPY frontend/package.json frontend/pnpm-lock.yaml* ./
RUN npm install -g pnpm && pnpm install --frozen-lockfile 2>/dev/null || pnpm install
COPY frontend/ ./
RUN pnpm build

# ─── Runtime ─────────────────────────────────────────────────
# Non-root: most container admission policies (k8s PSA restricted,
# Docker Bench, registry scanners) reject root containers.
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates curl && \
    rm -rf /var/lib/apt/lists/* && \
    # Create a non-root user with a fixed UID/GID for volume permission
    # stability. 10001 is in the dynamic UID range and unlikely to
    # collide with anything on the host.
    useradd --system --uid 10001 --gid 0 --home-dir /app --shell /usr/sbin/nologin sf

COPY --from=backend /app/target/release/social-forge /usr/local/bin/social-forge
# NOTE: frontend/build is already embedded in the binary via rust-embed
# (src/api/mod.rs:15-18). We no longer COPY it into the runtime image —
# saves ~5-10 MB and avoids any drift between the embedded and on-disk
# copies.
COPY migrations/ /app/migrations/

WORKDIR /app
RUN mkdir -p /app/uploads /app/data && \
    chown -R sf:0 /app/uploads /app/data

USER sf

EXPOSE 6543
HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -sf http://localhost:6543/health || exit 1

VOLUME ["/app/uploads", "/app/data"]

CMD ["social-forge", "serve"]
