<div align="center">
  <img src="assets/lash_logo.svg" alt="Lash Logo" width="200"/>

  # lash

  **Minimalist task tracker for devs and agents**

  [![CI](https://github.com/fixture-dev/lash/actions/workflows/ci.yml/badge.svg)](https://github.com/fixture-dev/lash/actions/workflows/ci.yml)
  [![Release](https://img.shields.io/github/v/release/fixture-dev/lash?sort=semver)](https://github.com/fixture-dev/lash/releases/latest)
  [![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
  [![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
  [![Built with Markdown](https://img.shields.io/badge/built%20with-markdown-000000.svg?logo=markdown)](https://commonmark.org/)
  [![SQLite](https://img.shields.io/badge/database-SQLite-003B57.svg?logo=sqlite)](https://www.sqlite.org/)

  **Status:** Active development
</div>

## Overview

Lash is a terminal-first task management system that uses Markdown as the single source of truth. It's designed for developers and AI agents who want:

- **Markdown-native**: Task files are just structured Markdown
- **Fast**: SQLite-backed indexing for instant queries
- **Strict format**: Linter-enforced structure for predictability
- **Agent-friendly**: Token-minimized output for LLM integration
- **Dependency-aware**: Cross-file task dependencies with cycle detection

## What's Implemented

All core functionality is production-ready and backed by an extensive test suite (3,000+ tests):

- **Markdown Parser** - Full task file parsing with contextual notes support
- **Linter & Formatter** - 28 validation rules with auto-formatting and interactive mode
- **SQLite Indexing** - Fast indexing engine exceeding performance targets by 8-12x
- **Dependency Resolution** - Complete graph analysis with cycle detection
- **Terminal UI (TUI)** - Interactive interface with 300+ Gogh color schemes and task creation
- **CLI Framework** - Configuration, logging, and command execution infrastructure
- **Query Commands** - List, search, show, and graph commands for exploring tasks
- **Task Creation** - CLI and TUI support for adding tasks with full annotation support
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

## Installation

### Prebuilt binaries (recommended)

Download a prebuilt binary for Linux, macOS, or Windows from the
[latest release](https://github.com/fixture-dev/lash/releases/latest), or use
the installer scripts published with each release:

```bash
# Linux / macOS
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/fixture-dev/lash/releases/latest/download/lash-installer.sh | sh
```

```powershell
# Windows
powershell -ExecutionPolicy Bypass -c "irm https://github.com/fixture-dev/lash/releases/latest/download/lash-installer.ps1 | iex"
```

### From source

Install Lash globally using the install script:

```bash
# Install Lash to ~/.cargo/bin/lash
./scripts/install.sh

# Force reinstall (useful for local development/testing)
./scripts/install.sh reinstall

# Check installation status
./scripts/install.sh status

# Uninstall
./scripts/install.sh uninstall
```

The script builds an optimized release binary and installs it to `~/.cargo/bin/lash`. Ensure `~/.cargo/bin` is in your PATH:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Alternatively, install directly with Cargo:

```bash
cargo install --path crates/lash-cli          # Install
cargo install --path crates/lash-cli --force  # Reinstall
cargo uninstall lash                          # Uninstall
```

## Try the Playground

Want to explore Lash's features without setting up your own project? Try the playground!

```bash
lash playground init
cd playground
lash list --label gameplay
```

The playground creates "PixelQuest" - a realistic game development demo project with:
- 25+ task files + index across features, systems, content, and milestones
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
- **Comprehensive tests**: 3,000+ tests across all crates (>80% coverage target)
- **Error taxonomy**: 50+ documented error codes in `docs/error-codes.md`
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

### Project Setup

```bash
# Initialize a new Lash project
lash init [--path DIR]

# Initialize demo project (PixelQuest)
lash playground init
```

### Linting & Formatting

```bash
# Lint task files
lash lint [PATH...] [--fix] [--interactive]

# Format task files (alias: fmt)
lash format [PATH...] [--check] [--diff]
```

### Indexing & Database

```bash
# Index files into database
lash index [--force] [--show-files]

# Verify database consistency
lash check-index [--diff]
```

### Querying Tasks

```bash
# List tasks (with filters)
lash list [--label backend] [--status open] [--owner name]
lash list [--tree] [--show-descriptions] [--show-notes]

# Search tasks (full-text)
lash search "authentication" [--limit 20]

# Show task details (agent note, dependency status, children; --short for terse)
lash show <task-id> [--deps] [--rdeps] [--short]
```

### Task Creation

```bash
# Add a new task
lash add "Task description" [--file path.md] [--parent task-id]
lash add "Task" --label backend --owner alice --estimate 2h
lash add --interactive  # Interactive mode
```

### Task Completion

```bash
# Mark tasks as complete
lash complete <task-id>              # Complete a single task
lash complete task1 task2 task3      # Complete multiple tasks
lash complete --dry-run <task-id>    # Preview without changing files
lash complete --json <task-id>       # Machine-readable output
```

### Task Waiving

```bash
# Mark a task as waived (not applicable) instead of completed
lash waive <task-id>
lash waive --reason "Superseded by task-2" <task-id>  # Record why
lash waive --cascade <task-id>       # Also waive unchecked plain-bullet children
lash waive --dry-run <task-id>       # Preview without changing files
```

Writes the `[-]` checkbox marker and re-indexes in the same run — no
separate `lash index` step needed. Already-waived tasks and completed
(`[x]`) tasks are refused (hand-edit the checkbox if completed work truly
needs to be waived).

### Task Editing

```bash
# Rewrite a task's title (pins the old title-derived @id first, so any
# @depends-on reference pointing at it keeps resolving)
lash update <task-id> --title "New title"

# Labels, owner, estimate
lash update <task-id> --add-label urgent --remove-label backend
lash update <task-id> --owner alice --estimate 2h
lash update <task-id> --owner ""   # empty string clears the annotation

# Agent notes
lash update <task-id> --agent-note "Replace the whole note"
lash update <task-id> --append-agent-note "Add a continuation line"

# Dependencies (validated against the project, like `lash add --depends-on`)
lash update <task-id> --add-depends-on other-task
lash update <task-id> --remove-depends-on other-task
lash update <task-id> --add-depends-on not-yet-created --allow-forward-ref

# Preview without writing
lash update <task-id> --title "New title" --dry-run
```

At least one mutation flag is required. Writes and re-indexes atomically,
same as `complete`/`waive`.

### Dependencies

```bash
# Export dependency graph
lash graph [--format ascii|dot|json|mermaid]
lash graph [--scope file.md] [--hide-completed]

# Validate cross-file links
lash check-links [--fix] [--dry-run]
```

### Heading-Slug Matching for `@doc:` Fragments

When a task references a documentation fragment with
`@doc: path/to/file.md#fragment`, Lash matches `fragment` against the headings
of the target document using **case-insensitive, punctuation-insensitive**
normalization. The fragment and each heading are both reduced to a canonical
form before comparison:

1. Lowercase the text.
2. Replace `-` with a space.
3. Drop every character that is not alphanumeric or whitespace
   (so `<`, `>`, `/`, `.`, `(`, `)`, backticks, and `_` are all stripped —
    **no word boundary is inserted**).
4. Collapse runs of whitespace into single spaces.

Examples:

| Heading                                                     | Matching fragment                                              |
|-------------------------------------------------------------|----------------------------------------------------------------|
| `## Section One`                                            | `section-one`                                                  |
| `## 1. Three-Runtime Separation`                            | `1-three-runtime-separation`                                   |
| `## Validation rules (must pass at index time)`             | `validation-rules-must-pass-at-index-time`                     |
| `` ### Pack manifest (`<pack>/SKILL.md`) ``                 | `pack-manifest-packskillmd` *(slashes/dots/angle brackets collapse to nothing)* |
| `` ### `allowed_tools` vocabulary (launch) ``               | `allowed_tools-vocabulary-launch` *(underscores match anything or nothing)* |

The matcher is symmetric: any fragment that normalizes to the same canonical
form as the heading will match. `lash lint` raises `W_SEM_DOC_FRAGMENT` when no
heading in the target file matches the fragment, and the warning message lists
the headings that do exist so you can pick the right one. Run
`lash explain W_SEM_DOC_FRAGMENT` for the long-form explanation.

### Agent Integration

```bash
# Generate live, project-specific prompt for LLMs (dynamic context on demand)
lash agent-prompt [--format plain|json|agents-md]
lash agent-prompt [--label backend] [--max-tokens 4000]
lash agent-prompt [--include-descriptions] [--include-notes]

# Install a static Lash skill into a coding agent's conventional directory
lash skill install --target claude|codex|cursor|agents-md [--scope project|user]
lash skill list                        # show installed skills
lash skill update --target claude      # refresh after upgrading lash
lash skill uninstall --target claude
```

### User Interface

```bash
# Launch TUI
lash tui [--color-scheme "Nord"]
```

### Configuration & Help

```bash
# Manage configuration
lash config get <key>
lash config set <key> <value>
lash config list [--changed]

# Generate shell completions
lash completion bash|zsh|fish|powershell|elvish

# Explain error codes
lash explain <CODE>
lash explain --list  # Show all error codes
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

### Description Sections

Task files can include an optional `## Description` section for providing detailed context about the file's purpose:

```markdown
# Authentication System

@id: auth
@labels: backend, security

## Description

This module handles all authentication flows including login, logout,
password reset, and session management. It integrates with our OAuth
providers and implements JWT-based token authentication.

## Tasks

- [ ] Implement login endpoint
- [ ] Add password reset flow
```

**Key points:**
- Description sections are full-text searchable via `lash search`
- Displayed in task detail views (`lash show`) and TUI
- Use `--show-descriptions` with `lash list` to include in output
- Great for providing context to both humans and AI agents

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

- [User Guide](./docs/user-guide.md) - Complete user documentation
- [Developer Guide](./docs/developer-guide.md) - Architecture and contribution guide
- [Agent Integration Guide](./docs/agent-guide.md) - Guide for AI agents using Lash
- [Design Document](./docs/design-doc.md) - Comprehensive specification
- [Error Codes](./docs/error-codes.md) - Complete error catalog
- [Testing Guide](./docs/TESTING.md) - Testing documentation
- [Examples](./examples/) - Tutorials and sample projects

## Contributing

We welcome contributions! See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

For the current development roadmap, see [`tasks/tasks.md`](./tasks/tasks.md) and [`devlog.md`](./devlog.md) for recent progress.

## License

Licensed under the [Apache License, Version 2.0](LICENSE).

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you shall be licensed as above, without any
additional terms or conditions.

### Third-party attributions

Bundled terminal color schemes in `crates/lash-tui/data/themes.json` are derived
from the [Gogh](https://github.com/Gogh-Co/Gogh) collection (MIT License). See
[`NOTICE`](NOTICE) for details.
