# ─── Build Stage ─────────────────────────────────────────────
FROM rust:1.85-slim-bookworm AS builder

RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache dependency downloads by building a minimal dummy first
COPY Cargo.toml Cargo.lock* ./
RUN mkdir -p src/ && echo "fn main() {}" > src/main.rs && \
    mkdir -p migrations/ && touch migrations/init.sql && \
    cargo build --release 2>/dev/null || true
RUN rm -rf src/ migrations/

# Now build the real application
COPY Cargo.toml Cargo.lock* ./
COPY src/ ./src/
COPY migrations/ ./migrations/

RUN cargo build --release && \
    strip target/release/postiz-rust

# ─── Runtime Stage ──────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates libssl3 wget && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/postiz-rust /usr/local/bin/postiz-rust

RUN mkdir -p /data/uploads

EXPOSE 3000
HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:3000/health || exit 1

VOLUME ["/data/uploads"]

CMD ["postiz-rust"]
