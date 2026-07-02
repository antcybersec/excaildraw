# Multi-stage production Dockerfile
# Combines WASM client and Axum server on a single port.

# 1. Build the WASM frontend
FROM rust:1.89-bookworm AS wasm-builder
RUN rustup target add wasm32-unknown-unknown
RUN cargo install trunk --locked
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY apps ./apps
RUN cd crates/client && trunk build --release

# 2. Build the Axum backend server
FROM rust:1.89-bookworm AS server-builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY apps ./apps
COPY --from=wasm-builder /app/dist ./dist
RUN cargo build --release -p excaildraw-server

# 3. Create the final lightweight runtime image
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=server-builder /app/target/release/excaildraw-server /usr/local/bin/
COPY --from=wasm-builder /app/dist /app/dist

# Default to 7860 for compatibility with Hugging Face Spaces.
# This environment variable can be overridden at runtime.
ENV PORT=7860
ENV FRONTEND_DIST=/app/dist
EXPOSE 7860

CMD ["excaildraw-server"]
