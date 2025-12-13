<div align="center">
  <img src="assets/lash_logo.svg" alt="Lash Logo" width="200"/>

  # lash

  **Minimalist task tracker for devs and agents**

  [![CI](https://github.com/fixture-dev/lash/workflows/CI/badge.svg)](https://github.com/fixture-dev/lash/actions)
  [![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
  [![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
  [![Built with Markdown](https://img.shields.io/badge/built%20with-markdown-000000.svg?logo=markdown)](https://commonmark.org/)
  [![SQLite](https://img.shields.io/badge/database-SQLite-003B57.svg?logo=sqlite)](https://www.sqlite.org/)

  **Status:** Active Development (Phase 5 - Core Complete, Documentation In Progress)
</div>

## Overview

Lash is a terminal-first task management system that uses Markdown as the single source of truth. It's designed for developers and AI agents who want:

- **Markdown-native**: Task files are just structured Markdown
- **Fast**: SQLite-backed indexing for instant queries
- **Strict format**: Linter-enforced structure for predictability
- **Agent-friendly**: Token-minimized output for LLM integration
- **Dependency-aware**: Cross-file task dependencies with cycle detection

## What's Implemented

All core functionality is production-ready:

- **Markdown Parser** - Full task file parsing with 390+ tests (67.7µs benchmark)
- **Linter & Formatter** - 20 validation rules with auto-formatting (607 tests)
- **SQLite Indexing** - Fast indexing engine exceeding performance targets by 8-12x
- **Dependency Resolution** - Complete graph analysis with cycle detection (495 tests)
- **Terminal UI (TUI)** - Interactive interface with 300+ Gogh color schemes
- **CLI Framework** - Configuration, logging, and command execution infrastructure
- **Query Commands** - List, search, show, and graph commands for exploring tasks
- **Agent Integration** - Token-minimized prompt generation for LLM workflows

## Project Structure

```
lash/
├── crates/
│   ├── lash-types/    # Shared types, errors, config
│   ├── lash-core/     # Markdown parsing & validation
│   ├── lash-db/       # SQLite indexing & queries
│   ├── lash-agent/    # Agent integration & prompt generation
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

## Try the Playground

Want to explore Lash's features without setting up your own project? Try the playground!

```bash
lash playground init
cd playground
lash list --label gameplay
```

The playground creates "PixelQuest" - a realistic game development demo project with:
- 23 task files + index across features, systems, content, and milestones
- Hundreds of tasks showing realistic project complexity
- Examples of dependencies, labels, statuses, and annotations
- A comprehensive `PLAYGROUND_GUIDE.md` with usage examples

Perfect for:
- Learning Lash's features
- Testing new commands
- Demos and presentations
- Understanding best practices

See `playground/PLAYGROUND_GUIDE.md` for detailed usage instructions.

## Development

This project follows strict quality standards:
- **Pre-commit hooks**: Auto-enforces formatting, linting, and tests
- **Zero warnings**: All clippy lints must pass with `clippy::pedantic`
- **Comprehensive tests**: 920+ tests across all crates (>80% coverage target)
- **Error taxonomy**: 25+ documented error codes in `docs/error-codes.md`
- **Doctests**: All public APIs include executable examples
- **CI/CD**: Automated testing on Linux, macOS, and Windows

```bash
# Run with formatting
cargo fmt --all

# Run comprehensive checks (enforced by pre-commit hook)
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --doc

# Install pre-commit hooks
./scripts/install-pre-commit-hook.sh

# Generate coverage report
cargo install cargo-llvm-cov
cargo llvm-cov --workspace --html
open target/llvm-cov/html/index.html
```

See [docs/TESTING.md](docs/TESTING.md) for detailed testing documentation.

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

```bash
# Lint task files
lash lint [PATH...]

# Format task files
lash format [PATH...]

# Index files into database
lash index

# Verify database consistency
lash check-index

# List tasks (with filters)
lash list [--label backend] [--status open] [--tree]

# Search tasks
lash search "authentication"

# Show task details
lash show task-id

# Export dependency graph
lash graph [--format dot|json|mermaid]

# Validate cross-file links
lash check-links [--fix]

# Generate agent prompt for LLMs
lash agent-prompt [OPTIONS]

# Launch TUI
lash tui [--color-scheme "Nord"]

# Manage configuration
lash config get <key>
lash config set <key> <value>
lash config list

# Generate shell completions
lash completion bash|zsh|fish|powershell

# Explain error codes
lash explain <CODE>

# Initialize demo project
lash playground init
```

### Contextual Notes

Lash supports **contextual notes** - plain bullet points (without checkboxes) nested under tasks that provide inline context, requirements, or acceptance criteria:

```markdown
- [ ] Implement payment gateway
  - Use Stripe API v3 for transactions
  - Support credit card and ACH payments
  - Must handle refunds and partial captures
  - [ ] Set up Stripe account
  - [ ] Implement checkout flow
  - [ ] Add webhook handling
```

**Key points:**
- **Plain bullets** (`- Text`) provide context and are *not* tracked for completion
- **Checkbox bullets** (`- [ ] Text`) are actionable tasks tracked for completion
- Notes should appear before child tasks (convention)
- Notes cannot have children (enforced by linter)
- Notes are searchable via `lash search`

This distinction helps separate "what needs to be done" from "how to do it" or "acceptance criteria", making task files more readable and providing better context for both humans and AI agents.

See [`examples/contextual-notes.md`](examples/contextual-notes.md) for comprehensive examples.

### Color Schemes

Lash supports 300+ color schemes from the [Gogh](https://gogh-co.github.io/Gogh/) collection. You can:

- **Set globally** via `~/.lash/config.toml`:
  ```toml
  color_scheme = "Nord"
  ```

- **Override per-command** with `--color-scheme`:
  ```bash
  lash tui --color-scheme "Dracula"
  ```

- **Change in TUI** by pressing `t` to open the theme selector

Popular schemes include: Nord, Dracula, Solarized Dark, Solarized Light, Monokai, Tokyo Night, Catppuccin, and Base2Tone Desert (default).

## Documentation

- [Design Document](./docs/design-doc.md) - Comprehensive specification
- [Error Codes](./docs/error-codes.md) - Complete error catalog
- [Testing Guide](./docs/TESTING.md) - Testing documentation
- [Development Tasks](./tasks/tasks.md) - Implementation roadmap

## Contributing

Lash is in active development. See [`tasks/tasks.md`](./tasks/tasks.md) for the current roadmap and [`devlog.md`](./devlog.md) for recent progress.

## License

MIT OR Apache-2.0 (dual-licensed)
