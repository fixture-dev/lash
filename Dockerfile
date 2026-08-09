# Dockerfile for flawd mutation testing - Rust
FROM rust:1.77-slim

RUN apt-get update && apt-get install -y git curl ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# flawd.toml's [coverage] command runs `cargo llvm-cov`, which shells out to
# llvm-cov and llvm-profdata. The llvm-tools component supplies both, so the
# command needs no LLVM_COV/LLVM_PROFDATA overrides. rust-toolchain.toml lands
# first so the component is added to the toolchain the build actually uses
# rather than the image's default.
COPY rust-toolchain.toml ./
RUN rustup component add llvm-tools-preview

# cargo-llvm-cov ships prebuilt binaries; installing from source here would add
# several minutes to every image build.
RUN set -eux; \
    case "$(uname -m)" in \
      x86_64)  target=x86_64-unknown-linux-gnu ;; \
      aarch64) target=aarch64-unknown-linux-gnu ;; \
      *) echo "no cargo-llvm-cov release for $(uname -m)" >&2; exit 1 ;; \
    esac; \
    curl -fsSL "https://github.com/taiki-e/cargo-llvm-cov/releases/latest/download/cargo-llvm-cov-${target}.tar.gz" \
      | tar xzf - -C "${CARGO_HOME}/bin"; \
    cargo llvm-cov --version

# Pre-fetch and build dependencies for layer caching (debug/test profile)
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
    cargo test --no-run 2>/dev/null || true
RUN find crates -name "*.rs" -delete

# Copy project source and build test artifacts
COPY . .
RUN cargo test --no-run 2>/dev/null || true

# Ensure the lash binary built by `cargo test` is on PATH for subprocess tests
ENV PATH="/app/target/debug:${PATH}"

# Keep container running for flawd to exec into
CMD ["sleep", "infinity"]
