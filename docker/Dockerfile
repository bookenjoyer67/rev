FROM rust:1.82-slim AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY migrations/ migrations/

ENV SQLX_OFFLINE=true
RUN cargo build --release --bin komun-server

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates wget && rm -rf /var/lib/apt/lists/*

RUN groupadd -r komun && useradd -r -g komun -d /tmp komun

WORKDIR /app
COPY --from=builder --chown=komun:komun /app/target/release/komun-server ./
COPY --chown=komun:komun migrations/ migrations/

RUN mkdir -p /app/data/relay && chown komun:komun /app/data/relay

EXPOSE 3000 9001
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:3000/api/health || exit 1

USER komun
CMD ["./komun-server"]
