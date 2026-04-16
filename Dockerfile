# ═══════════════════════════════════════════════
# Stage 1: Build WASM frontend
# ═══════════════════════════════════════════════
FROM rust:1-bookworm AS wasm-builder

RUN curl -L https://github.com/wasm-bindgen/wasm-pack/releases/download/v0.13.1/wasm-pack-v0.13.1-x86_64-unknown-linux-musl.tar.gz -o /tmp/wasm-pack.tar.gz \
    && tar -xzf /tmp/wasm-pack.tar.gz -C /tmp \
    && mv /tmp/wasm-pack-v0.13.1-x86_64-unknown-linux-musl/wasm-pack /usr/local/bin/wasm-pack \
    && chmod +x /usr/local/bin/wasm-pack \
    && rm -rf /tmp/wasm-pack.tar.gz /tmp/wasm-pack-v0.13.1-x86_64-unknown-linux-musl

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates/engine/Cargo.toml crates/engine/Cargo.toml
COPY crates/frontend/Cargo.toml crates/frontend/Cargo.toml
COPY crates/backend/Cargo.toml crates/backend/Cargo.toml

COPY crates/engine/ crates/engine/
COPY crates/frontend/ crates/frontend/

RUN wasm-pack build --target web --out-dir /app/static/wasm --out-name karta --no-typescript crates/frontend/

# ═══════════════════════════════════════════════
# Stage 2: Build backend + seed binaries
# ═══════════════════════════════════════════════
FROM rust:1-bookworm AS backend-builder

RUN apt-get update && apt-get install -y pkg-config libsqlite3-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates/engine/Cargo.toml crates/engine/Cargo.toml
COPY crates/frontend/Cargo.toml crates/frontend/Cargo.toml
COPY crates/backend/Cargo.toml crates/backend/Cargo.toml
COPY crates/engine/ crates/engine/
COPY crates/backend/ crates/backend/
# Frontend source is not needed for backend build, but Cargo.toml must exist for workspace resolution
RUN mkdir -p crates/frontend/src && echo "" > crates/frontend/src/lib.rs
COPY .sqlx/ .sqlx/
COPY crates/backend/migrations/ crates/backend/migrations/

ENV SQLX_OFFLINE=true

RUN cargo build --release -p backend --bin backend --bin seed

# ═══════════════════════════════════════════════
# Stage 3: Runtime
# ═══════════════════════════════════════════════
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libsqlite3-0 curl && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=backend-builder /app/target/release/backend /app/backend
COPY --from=backend-builder /app/target/release/seed /app/seed
COPY static/ /app/static-reference/
COPY --from=wasm-builder /app/static/wasm/ /app/static-reference/wasm/

COPY crates/backend/migrations/ /app/migrations/
COPY data/template/ /app/template/
COPY entrypoint.sh /app/entrypoint.sh

RUN chmod +x /app/backend /app/seed /app/entrypoint.sh

ENV DATABASE_URL="sqlite:/app/data/rom.db"
ENV SEED_DATA_PATH="/app/template"
ENV RUST_LOG="info"
ENV PORT=3000
ENV SECURE_COOKIE=true

EXPOSE 3000

VOLUME ["/app/data", "/app/static"]

ENTRYPOINT ["/app/entrypoint.sh"]
CMD ["./backend"]
