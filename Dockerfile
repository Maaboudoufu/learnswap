# syntax=docker/dockerfile:1

# Everything is compiled inside the image, so this builds unchanged on x86-64
# and on the ARM cores of an Oracle Cloud Ampere instance -- no cross-compiling
# and no architecture-specific tags.

# ----------------------------------------------------------------- build stage
FROM rust:1.98-slim-bookworm AS builder

# curl fetches the Tailwind standalone CLI. The compilers are for the C sources
# vendored by libsqlite3-sys and ring (rustls).
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential pkg-config curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .

# The same script the team runs locally; it picks the CLI build matching this
# machine's architecture.
RUN ./scripts/tailwind.sh

# --locked so a deploy builds the exact dependency versions in Cargo.lock.
RUN cargo build --release --locked

# --------------------------------------------------------------- runtime stage
FROM debian:bookworm-slim AS runtime

# ca-certificates for TLS to a remote Postgres; curl only for the healthcheck.
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 learnswap

WORKDIR /app

# Templates and static assets are read from disk relative to the working
# directory. The SQL migrations are compiled into the binary by sqlx::migrate!,
# so migrations/ is deliberately not copied here.
COPY --from=builder /build/target/release/learnswap /usr/local/bin/learnswap
COPY --from=builder /build/templates ./templates
COPY --from=builder /build/assets ./assets

# The SQLite file lives on a volume so the database survives image rebuilds and
# `docker run --rm`. Point DATABASE_URL at Postgres to use that instead.
RUN mkdir -p /data && chown learnswap:learnswap /data
VOLUME ["/data"]

USER learnswap

ENV HOST=0.0.0.0 \
    PORT=3000 \
    DATABASE_URL="sqlite:///data/learnswap.db?mode=rwc" \
    RUST_LOG="learnswap=info,tower_http=info,warn"

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=3s --start-period=20s --retries=3 \
    CMD curl -fsS http://127.0.0.1:3000/health || exit 1

CMD ["learnswap"]
