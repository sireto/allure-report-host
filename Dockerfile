# --- Build Stage ---
FROM rust:1.86-bookworm AS builder

WORKDIR /app

# Copy manifests first for better caching
COPY api/Cargo.toml api/Cargo.toml

# Create a dummy main to cache dependencies
RUN mkdir -p api/src && \
    echo "fn main() {}" > api/src/main.rs && \
    echo "" > api/src/lib.rs && \
    cd api && cargo build --release && \
    rm -rf api/src

# Copy actual source code
COPY api/src api/src

# Touch main.rs to force rebuild of our code (not deps)
RUN touch api/src/main.rs && cd api && cargo build --release

# --- Runtime Stage ---
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    nodejs \
    npm \
    && npm install -g allure-commandline \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/api/target/release/api /app/api
COPY scripts/ /app/scripts/

RUN chmod +x /app/scripts/*.sh

EXPOSE 8088

ENV DATA_DIR=/app/data
ENV API_KEY=""

CMD ["/app/api"]