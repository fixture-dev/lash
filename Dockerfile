# Dockerfile for flawd mutation testing - Rust
FROM rust:1.77-slim

WORKDIR /app

# Pre-fetch dependencies for better layer caching
COPY Cargo.toml Cargo.lock ./
COPY crates/lash-types/Cargo.toml crates/lash-types/Cargo.toml
COPY crates/lash-core/Cargo.toml crates/lash-core/Cargo.toml
COPY crates/lash-db/Cargo.toml crates/lash-db/Cargo.toml
COPY crates/lash-agent/Cargo.toml crates/lash-agent/Cargo.toml
COPY crates/lash-tui/Cargo.toml crates/lash-tui/Cargo.toml
COPY crates/lash-cli/Cargo.toml crates/lash-cli/Cargo.toml
RUN for dir in crates/lash-types crates/lash-core crates/lash-db crates/lash-agent crates/lash-tui crates/lash-cli; do \
      mkdir -p "$dir/src" && echo "fn main() {}" > "$dir/src/main.rs" && echo "" > "$dir/src/lib.rs"; \
    done && \
    cargo build --release 2>/dev/null || true
RUN find crates -name "*.rs" -delete

# Copy project source
COPY . .

# Keep container running for flawd to exec into
CMD ["sleep", "infinity"]
