# Entropa — multi-stage build. Final image is just the binary + embedded Scryon/assets
# (compiled in via include_str!/include_bytes!), so it stays minimal and fast to cold-start
# on Cloud Run's scale-to-zero.

FROM rust:1-slim-bookworm AS builder
WORKDIR /build
COPY . .
RUN cargo build --release -p entropa-api

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/entropa-api /usr/local/bin/entropa-api
EXPOSE 8080
CMD ["entropa-api"]
