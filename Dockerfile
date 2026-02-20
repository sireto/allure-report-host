FROM rust:1.93.0-bookworm AS builder

WORKDIR /app

COPY api/Cargo.toml api/Cargo.toml
COPY api/Cargo.lock api/Cargo.lock

# Copy the source and build
COPY api/src api/src
RUN cd api && cargo build --release

# Runtime Stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    nodejs \
    npm \
    && npm install -g allure \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/api/target/release/api /app/api
COPY scripts/ /app/scripts/
COPY data/ /app/data/

RUN chmod +x /app/scripts/*.sh

EXPOSE 8080

CMD ["/app/api"]