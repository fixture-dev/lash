<div align="center">
  <img src="assets/lash_logo.svg" alt="Lash Logo" width="200"/>

  # lash

  **Minimalist task tracker for devs and agents**

  [![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
  [![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
  [![Built with Markdown](https://img.shields.io/badge/built%20with-markdown-000000.svg?logo=markdown)](https://commonmark.org/)
  [![SQLite](https://img.shields.io/badge/database-SQLite-003B57.svg?logo=sqlite)](https://www.sqlite.org/)

  **Status:** Active Development (Phase 4 - Dependencies & Queries)
</div>

## Overview

Lash is a terminal-first task management system that uses Markdown as the single source of truth. It's designed for developers and AI agents who want:

- **Markdown-native**: Task files are just structured Markdown
- **Fast**: SQLite-backed indexing for instant queries
- **Strict format**: Linter-enforced structure for predictability
- **Agent-friendly**: Token-minimized output for LLM integration
- **Dependency-aware**: Cross-file task dependencies with cycle detection

## What's Implemented

Core functionality is production-ready:

- **Markdown Parser** - Full task file parsing with 390+ tests (67.7µs benchmark)
- **Linter & Formatter** - 20 validation rules with auto-formatting (607 tests)
- **SQLite Indexing** - Fast indexing engine exceeding performance targets by 8-12x
- **Dependency Resolution** - Complete graph analysis with cycle detection (495 tests)
- **CLI Framework** - Configuration, logging, and command execution infrastructure

Next up: Query commands (`list`, `show`, `search`, `graph`)

## Project Structure

```
lash/
├── crates/
│   ├── lash-types/    # Shared types, errors, config ✅
│   ├── lash-core/     # Markdown parsing & validation ✅
│   ├── lash-db/       # SQLite indexing & queries ✅
│   ├── lash-agent/    # Agent integration (planned)
│   ├── lash-tui/      # Terminal UI (planned)
│   └── lash-cli/      # CLI binary (in progress)
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

This project follows strict quality standards:
- **Pre-commit hooks**: Auto-enforces formatting, linting, and tests
- **Zero warnings**: All clippy lints must pass with `clippy::pedantic`
- **Comprehensive tests**: 1300+ tests across all crates (>80% coverage)
- **Error taxonomy**: 25+ documented error codes in `docs/error-codes.md`
- **Doctests**: All public APIs include executable examples

```bash
# Run with formatting
cargo fmt --all

# Run comprehensive checks (enforced by pre-commit hook)
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --doc
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

**Performance achieved (exceeds targets by 8-12x):**
- Small projects (10 files, ~50 tasks): 10.5ms (target: <50ms)
- Medium projects (100 files, ~500 tasks): 61ms (target: <500ms)
- Large projects (1000 files, ~5000 tasks): 425ms (target: <5s)

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

## Usage

**Implemented commands:**

```bash
# Lint task files
lash lint [PATH...]

# Format task files
lash format [PATH...]

# Index files into database
lash index  # Coming soon
```

**Planned commands:**

```bash
# List tasks
lash list [--label backend] [--status open]

# Search tasks
lash search "authentication"

# Show task details
lash show task-id

# Export dependency graph
lash graph [--format dot|json]

# Generate agent prompt
lash agent-prompt

# Launch TUI
lash tui
```

## Documentation

- [Design Document](./docs/design-doc.md) - Comprehensive specification
- [Error Codes](./docs/error-codes.md) - Complete error catalog
- [Development Tasks](./tasks/tasks.md) - Implementation roadmap

## Contributing

Lash is in active development. See [`tasks/tasks.md`](./tasks/tasks.md) for the current roadmap and [`devlog.md`](./devlog.md) for recent progress.

## License

MIT OR Apache-2.0 (dual-licensed)

