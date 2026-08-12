# Entropa — multi-stage build. Final image is just the binary + embedded Scryon/assets
# (compiled in via include_str!/include_bytes!), so it stays minimal and fast to cold-start
# on Cloud Run's scale-to-zero.

FROM rust:1-slim-bookworm AS builder
WORKDIR /build

# Cache dependency compilation separately from source changes.
COPY Cargo.toml Cargo.lock* ./
COPY crates/core/Cargo.toml crates/core/Cargo.toml
COPY crates/node/Cargo.toml crates/node/Cargo.toml
COPY crates/api/Cargo.toml crates/api/Cargo.toml
RUN mkdir -p crates/core/src crates/node/src crates/api/src crates/api/scryon/assets \
    && echo "fn main() {}" > crates/api/src/main.rs \
    && echo "" > crates/core/src/lib.rs \
    && echo "" > crates/node/src/lib.rs \
    && echo "" > crates/api/src/lib.rs \
    && touch crates/api/scryon/assets/favicon.ico \
    && cargo build --release -p entropa-api 2>/dev/null || true

# Now bring in the real source and build for real.
COPY . .
RUN cargo build --release -p entropa-api

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/entropa-api /usr/local/bin/entropa-api
EXPOSE 8080
CMD ["entropa-api"]
