# syntax=docker/dockerfile:1

FROM rust:1.94-slim-bookworm AS chef
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    clang \
    llvm \
    cmake \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --version "^0.1" --locked
RUN cargo install sccache --version "^0.8" --locked
ENV RUSTC_WRAPPER=sccache
ENV SCCACHE_DIR=/sccache
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
    cargo build --release --package phantom-mcp --bin phantom && \
    cp /app/target/release/phantom /tmp/phantom
RUN --mount=type=cache,target=$SCCACHE_DIR,sharing=locked sccache --show-stats || true

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*
RUN groupadd --gid 1001 phantom && \
    useradd --uid 1001 --gid phantom --no-create-home --shell /usr/sbin/nologin phantom
RUN mkdir -p /data/storage && \
    chown -R phantom:phantom /data && \
    chmod 700 /data/storage
COPY --from=builder /tmp/phantom /usr/local/bin/phantom
USER phantom
EXPOSE 8080
VOLUME ["/data"]
ENV PHANTOM_BIND_ADDR=0.0.0.0:8080
ENV PHANTOM_STORAGE_DIR=/data/storage
ENV PHANTOM_LOG_FORMAT=json
ENV RUST_LOG=phantom=info,tower_http=warn
HEALTHCHECK --interval=15s --timeout=5s --start-period=30s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1
ENTRYPOINT ["/usr/local/bin/phantom"]
