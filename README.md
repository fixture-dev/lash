# lash

Minimalist, ultra-fast, Markdown-native task tracker for devs and agents.

**Status:** Early development (Phase 1 - Foundation complete)

## Overview

Lash is a terminal-first task management system that uses Markdown as the single source of truth. It's designed for developers and AI agents who want:

- **Markdown-native**: Task files are just structured Markdown
- **Fast**: SQLite-backed indexing for instant queries
- **Strict format**: Linter-enforced structure for predictability
- **Agent-friendly**: Token-minimized output for LLM integration
- **Dependency-aware**: Cross-file task dependencies with cycle detection

## Project Structure

```
lash/
├── crates/
│   ├── lash-types/    # Shared types, errors, config
│   ├── lash-core/     # Markdown parsing & validation
│   ├── lash-db/       # SQLite indexing & queries
│   ├── lash-agent/    # Agent integration
│   ├── lash-tui/      # Terminal UI
│   └── lash-cli/      # CLI binary
├── docs/              # Design docs & error codes
└── tasks/             # Development task tracking
```

## Building

Requirements: Rust stable toolchain (see `rust-toolchain.toml`)

```bash
# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Check formatting and lints
cargo fmt --check
cargo clippy --workspace -- -D warnings
```

## Development

This project uses:
- **Pre-commit hook**: Enforces formatting, linting, and tests before commits
- **Strict linting**: Zero warnings policy with `clippy::pedantic`
- **Test fixtures**: Located in `crates/lash-cli/tests/fixtures/`
- **Error taxonomy**: 25+ documented error codes in `docs/error-codes.md`

```bash
# Run with formatting
cargo fmt --all

# Run comprehensive checks
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### Performance Benchmarking

Lash includes comprehensive benchmarks for indexing performance. Benchmarks measure performance across different project sizes and scenarios.

```bash
# Run all benchmarks
cargo bench --package lash-db --bench indexing

# Run specific benchmark
cargo bench --package lash-db --bench indexing -- full_indexing

# Quick benchmarks (faster, less accurate)
cargo bench --package lash-db --bench indexing -- --quick

# View HTML reports
open target/criterion/report/index.html
```

**Performance targets:**
- Small projects (10 files, ~50 tasks): <50ms
- Medium projects (100 files, ~500 tasks): <500ms
- Large projects (1000 files, ~5000 tasks): <5s

### Performance Profiling

Enable profiling to measure time spent in each indexing phase:

```rust
use lash_db::{Indexer, IndexerConfig, init_database};
use lash_types::LashConfig;

let conn = init_database(&db_path)?;
let config = IndexerConfig::new(project_root)
    .with_profiling(true);  // Enable profiling
let parser_config = LashConfig::default();
let mut indexer = Indexer::new(&conn, config, &parser_config);

let report = indexer.index_project()?;

// Print performance summary
if let Some(profile) = report.profile {
    profile.print_summary();
    // Or export to JSON for analysis
    println!("{}", profile.to_json_pretty());
}
```

The profiler tracks:
- **Phase times**: discovery, diff, parsing, database, closure_rebuild
- **Per-file parse times**: Individual file parsing performance
- **Database operations**: Query and insert times with row counts
- **Total duration**: End-to-end indexing time

## Usage (Planned)

```bash
# Lint task files
lash lint [PATH...]

# Index files into database
lash index

# List tasks
lash list [--label backend] [--status open]

# Search tasks
lash search "authentication"

# Show task details
lash show task-id

# Generate agent prompt
lash agent-prompt

# Launch TUI
lash tui
```

## Documentation

- [Design Document](./docs/design-doc.md) - Comprehensive specification
- [Error Codes](./docs/error-codes.md) - Complete error catalog
- [Development Tasks](./tasks/tasks.md) - Implementation roadmap

## License

TBD

