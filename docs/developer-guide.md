# Lash Developer Guide

This guide provides everything you need to understand, build, and contribute to Lash.

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [Development Setup](#development-setup)
- [Code Organization](#code-organization)
- [Core Components](#core-components)
- [Database Schema](#database-schema)
- [Error Handling](#error-handling)
- [Testing Strategy](#testing-strategy)
- [Contributing](#contributing)
- [Release Process](#release-process)

---

## Architecture Overview

Lash is built as a modular Rust workspace with clear separation of concerns. The architecture follows these key principles:

- **Markdown as Source of Truth**: All task data lives in Markdown files
- **SQLite as Acceleration Layer**: Database is fully reconstructible from Markdown
- **Layered Design**: Each crate has a single, well-defined responsibility
- **Interface-Based**: Components program to interfaces, not implementations

### Crate Structure

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

### Crate Responsibilities

#### lash-types

**Purpose**: Foundation types shared across all crates

**Key Components**:
- `Task`, `TaskFile` - Core data structures
- `LashError` - Unified error taxonomy with 25+ error codes
- `LashConfig` - Project and user configuration
- `Status`, `Label`, `Dependency` - Domain types
- `TaskCreationBuilder` - Builder pattern for task creation
- `ErrorFormatter` - Rich error output with miette integration

**Dependencies**: None (foundational crate)

**When to use**: Define new types that need to be shared across multiple crates

#### lash-core

**Purpose**: Markdown parsing, linting, validation, and dependency resolution

**Key Components**:
- `Parser` - Markdown parsing using pulldown-cmark
- `Linter` - 20+ validation rules with auto-fix capabilities
- `Formatter` - Markdown normalization and pretty-printing
- `DependencyGraph` - Task dependency resolution with cycle detection
- `ContextualNotesParser` - Plain bullet note parsing
- `FuzzyMatcher` - String similarity matching for search

**Dependencies**: `lash-types`

**Performance**:
- File parsing: ~67.7µs (benchmark target met)
- Linting: >500 tasks/sec

**When to use**: Any Markdown format changes, new lint rules, parser features

#### lash-db

**Purpose**: SQLite indexing, querying, and incremental updates

**Key Components**:
- `Indexer` - Fast indexing engine with profiling support
- `SearchEngine` - Full-text search with FTS5
- `DependencyUpdater` - Incremental dependency graph updates
- `GraphBuilder` - Dependency closure computation
- `Verifier` - Database consistency checking
- `ProjectRoot` - Project root detection

**Dependencies**: `lash-types`, `lash-core`

**Performance**:
- Small projects (10 files, ~50 tasks): 10.5ms (target: <50ms) ✅
- Medium projects (100 files, ~500 tasks): 61ms (target: <500ms) ✅
- Large projects (1000 files, ~5000 tasks): 425ms (target: <5s) ✅

**When to use**: Database schema changes, query optimization, indexing improvements

#### lash-agent

**Purpose**: LLM agent integration and token minimization

**Key Components**:
- `PromptGenerator` - Token-minimized context generation
- `SchemaGenerator` - JSON schema for agent tooling
- `TokenCounter` - Token estimation utilities
- `ContextBuilder` - Sparse context for specific agent actions

**Dependencies**: `lash-types`, `lash-core`, `lash-db`

**When to use**: New agent features, prompt templates, token optimization

#### lash-tui

**Purpose**: Terminal user interface

**Key Components**:
- `App` - Main TUI application state machine
- `EventHandler` - Keyboard and mouse event processing
- `ThemeManager` - 300+ Gogh color scheme support
- `TaskCreator` - Interactive task creation UI
- `Terminal` - Crossterm/ratatui integration

**Dependencies**: `lash-types`, `lash-core`, `lash-db`

**When to use**: New TUI features, UI improvements, theme additions

#### lash-cli

**Purpose**: Command-line interface and user-facing binary

**Key Components**:
- `Cli` - Clap-based CLI parser
- `CommandExecutor` - Command dispatch and execution
- `ErrorReporter` - Rich terminal error output
- `TreeFormatter` - Hierarchical task tree rendering
- `ProgressIndicator` - Operation progress UI
- `ConfigManager` - Configuration file handling
- `CliTheme` - Centralized colored output

**Dependencies**: All crates

**When to use**: New CLI commands, output formatting, user experience

### Data Flow

```
┌─────────────────────────────────────────────────────────────┐
│                       User Interaction                       │
├─────────────────┬───────────────────────────────────────────┤
│   CLI Commands  │              TUI                          │
│  (lash-cli)     │          (lash-tui)                       │
└────────┬────────┴───────────────┬──────────────────────────┘
         │                         │
         ├─────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│                    Command Processing                        │
│                      (lash-cli)                             │
└─────────┬──────────────────────────────┬────────────────────┘
          │                              │
          ▼                              ▼
┌──────────────────────┐      ┌──────────────────────┐
│   Markdown Layer     │      │   Database Layer     │
│    (lash-core)       │◄────►│     (lash-db)        │
└──────────────────────┘      └──────────────────────┘
          │                              │
          │                              │
          ├──────────────┬───────────────┤
          │              │               │
          ▼              ▼               ▼
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│   Parser    │  │   Linter    │  │   Indexer   │
└─────────────┘  └─────────────┘  └─────────────┘
```

**Typical Operation Flow**:

1. **Parse**: User runs `lash list --label backend`
2. **CLI Layer**: `lash-cli` parses arguments, validates project root
3. **Database Query**: `lash-db` queries SQLite for matching tasks
4. **Markdown Read**: If needed, `lash-core` parses files for details
5. **Format Output**: `lash-cli` formats results for terminal display
6. **Display**: Rendered output shown to user

### Module Boundaries

Clear boundaries between crates prevent coupling:

- **Types flow downward**: `lash-types` → `lash-core` → `lash-db`
- **No circular dependencies**: Enforced by Cargo
- **Interface segregation**: Each crate exposes minimal public API
- **Data ownership**: Markdown owns state, SQLite caches it

---

## Development Setup

### Prerequisites

- **Rust**: Version 1.75+ (see `rust-toolchain.toml`)
- **Git**: For version control
- **SQLite**: Bundled via rusqlite (no external install needed)
- **Optional**: `cargo-llvm-cov` for coverage reports

### Install Rust

```bash
# Via rustup (recommended)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify installation
rustc --version
cargo --version
```

### Clone and Build

```bash
# Clone repository
git clone https://github.com/fixture-dev/lash.git
cd lash

# Build all crates
cargo build --workspace

# Build in release mode (optimized)
cargo build --workspace --release

# Install locally
cargo install --path crates/lash-cli
```

### Running Tests

See [TESTING.md](./TESTING.md) for comprehensive testing documentation.

**Quick start**:

```bash
# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p lash-core

# Run with output
cargo test --workspace -- --nocapture

# Run doc tests
cargo test --doc
```

### Running Benchmarks

Lash includes comprehensive benchmarks for performance-critical code:

```bash
# Run all benchmarks
cargo bench --workspace

# Run specific benchmark suite
cargo bench -p lash-db --bench indexing
cargo bench -p lash-core --bench parser_bench

# Quick benchmarks (faster, less accurate)
cargo bench -- --quick

# View HTML reports
open target/criterion/report/index.html
```

**Benchmark suites**:
- `parser_bench` - Markdown parsing performance
- `graph_bench` - Dependency graph operations
- `linter_bench` - Linting speed
- `indexing` - Database indexing throughput
- `search_bench` - Full-text search latency
- `notes_parser_bench` - Contextual notes parsing
- `notes_indexing_bench` - Notes indexing performance

### Code Quality Checks

```bash
# Format code
cargo fmt --all

# Run clippy (strict mode)
cargo clippy --workspace --all-targets -- -D warnings

# Check format without modifying
cargo fmt --check

# Run all pre-commit checks
./scripts/pre-commit
```

### Installing Pre-commit Hooks

Automate quality checks before every commit:

```bash
./scripts/install-pre-commit-hook.sh
```

The hook runs:
- `cargo fmt --check` - Code formatting
- `cargo clippy` - Lint checks
- `cargo test --workspace --lib` - Unit tests
- `cargo test --doc` - Doc tests

To bypass (not recommended): `git commit --no-verify`

### Development Tools

**Recommended tools**:

```bash
# Coverage reporting
cargo install cargo-llvm-cov

# Generate coverage report
cargo llvm-cov --workspace --html
open target/llvm-cov/html/index.html

# Benchmarking (included with criterion)
cargo bench

# Watch mode (auto-rebuild on changes)
cargo install cargo-watch
cargo watch -x test

# Dependency graph visualization
cargo install cargo-depgraph
cargo depgraph | dot -Tpng > deps.png
```

---

## Code Organization

### Directory Structure

```
lash/
├── .github/
│   └── workflows/
│       └── ci.yml                  # CI/CD pipeline
├── crates/
│   ├── lash-types/
│   │   ├── src/
│   │   │   ├── lib.rs              # Public API
│   │   │   ├── error.rs            # Error types
│   │   │   ├── task.rs             # Task model
│   │   │   ├── file.rs             # TaskFile model
│   │   │   ├── config.rs           # Configuration
│   │   │   └── ...
│   │   └── Cargo.toml
│   ├── lash-core/
│   │   ├── src/
│   │   │   ├── lib.rs              # Parser & linter
│   │   │   ├── parser/             # Markdown parser
│   │   │   ├── linter/             # Validation rules
│   │   │   ├── formatter/          # Code formatting
│   │   │   └── ...
│   │   ├── benches/                # Benchmarks
│   │   └── Cargo.toml
│   ├── lash-db/
│   │   ├── src/
│   │   │   ├── lib.rs              # Database API
│   │   │   ├── indexer.rs          # Indexing engine
│   │   │   ├── search.rs           # Search queries
│   │   │   ├── migrations.rs       # Schema migrations
│   │   │   └── ...
│   │   └── Cargo.toml
│   ├── lash-agent/
│   │   └── src/
│   │       ├── lib.rs              # Agent API
│   │       ├── prompt.rs           # Prompt generation
│   │       └── ...
│   ├── lash-tui/
│   │   └── src/
│   │       ├── lib.rs              # TUI library
│   │       ├── app.rs              # Application state
│   │       ├── state.rs            # UI state machine
│   │       └── ...
│   └── lash-cli/
│       ├── src/
│       │   ├── main.rs             # Binary entry point
│       │   ├── cli.rs              # CLI argument parsing
│       │   ├── command.rs          # Command implementations
│       │   └── ...
│       ├── tests/
│       │   ├── common/             # Test utilities
│       │   ├── fixtures/           # Test data
│       │   └── e2e_cli_tests.rs    # End-to-end tests
│       └── Cargo.toml
├── docs/
│   ├── design-doc.md               # Technical specification
│   ├── error-codes.md              # Error catalog
│   ├── TESTING.md                  # Testing guide
│   └── developer-guide.md          # This file
├── tasks/                          # Development tracking
│   ├── tasks.md                    # Task index
│   └── tasks.*.md                  # Feature-specific tasks
├── scripts/
│   ├── install-pre-commit-hook.sh  # Hook installer
│   └── pre-commit                  # Pre-commit checks
└── Cargo.toml                      # Workspace manifest
```

### Naming Conventions

**Files**:
- `lib.rs` - Crate root, public API
- `mod.rs` - Module index (avoid when possible, prefer named files)
- `{feature}.rs` - Single-responsibility modules
- Test files match source: `parser.rs` → `parser_tests.rs` or `#[cfg(test)] mod tests`

**Functions**:
- `snake_case` for functions and methods
- `new()` for constructors
- `try_*()` or `*_checked()` for fallible operations
- `to_*()` for conversions (consumes self)
- `as_*()` for cheap references
- `into_*()` for consuming conversions

**Types**:
- `PascalCase` for structs, enums, traits
- `SCREAMING_SNAKE_CASE` for constants
- Prefer descriptive names: `TaskFile` over `TF`
- Error types: `*Error` suffix
- Result types: `Result<T, Error>` or type alias `type Result<T> = std::result::Result<T, Error>`

**Modules**:
- One concept per module
- Public items explicitly marked `pub`
- Re-export commonly used types in `lib.rs`

### Module Boundaries

**Public API Surface**:

```rust
// lash-types/src/lib.rs - Everything needed by consumers
pub use error::{LashError, LashResult};
pub use task::{Task, TaskStatus};
pub use file::TaskFile;
pub use config::LashConfig;

// Internal-only modules
mod internal_utils; // Not re-exported
```

**Dependency rules**:
1. **Never** import from `lash-cli` (it's the integration layer)
2. `lash-types` imports nothing internal (foundation)
3. `lash-core` can import `lash-types` only
4. `lash-db` can import `lash-types`, `lash-core`
5. `lash-agent` can import `lash-types`, `lash-core`, `lash-db`
6. `lash-tui` can import `lash-types`, `lash-core`, `lash-db`

---

## Core Components

### Markdown Parser

**Location**: `crates/lash-core/src/parser/`

**Purpose**: Parse Lash Markdown files into structured `TaskFile` objects

**Key Files**:
- `parser.rs` - Main parsing logic
- `annotations.rs` - Metadata parsing (`@id`, `@labels`, etc.)
- `notes.rs` - Contextual notes parsing

**Example Usage**:

```rust
use lash_core::parser::parse_file;
use lash_types::LashConfig;
use std::path::PathBuf;

let path = PathBuf::from("tasks.md");
let config = LashConfig::default();
let task_file = parse_file(&path, &config)?;

println!("Found {} tasks", task_file.tasks.len());
```

**Key Concepts**:

1. **Two-pass parsing**:
   - First pass: Extract metadata block
   - Second pass: Parse task tree

2. **Contextual notes**: Plain bullets (`- Text`) vs checkboxes (`- [ ] Task`)
   - Notes provide context without completion tracking
   - Indexed in database for search
   - Cannot have children (linter enforced)

3. **Annotations**: Structured metadata
   ```markdown
   @id: unique.identifier
   @labels: backend, api
   @owner: alice
   @estimate: 2h
   ```

**Performance**: ~67.7µs per file (benchmark)

### Linter

**Location**: `crates/lash-core/src/linter/`

**Purpose**: Validate Markdown format and enforce rules

**Validation Rules** (20+ total):
- `E_LINT_MISSING_ID` - Files must have `@id`
- `E_LINT_DUPLICATE_ID` - IDs must be unique
- `E_LINT_DEPTH_EXCEEDED` - Respect max nesting (default: 3)
- `E_LINT_INVALID_STATUS` - Status values must be valid
- `E_LINT_UNKNOWN_ANNOTATION` - No unknown annotations
- `E_LINT_BAD_INDENTATION` - Consistent indentation (2 spaces)
- `E_LINT_NOTE_HAS_CHILDREN` - Notes cannot have sub-items
- `E_LINT_DESCRIPTION_TOO_LONG` - Description length limits

**Auto-fix capabilities**:
- Normalize indentation
- Sort annotations
- Fix spacing around headings
- Add missing header boilerplate

**Example Usage**:

```rust
use lash_core::linter::{lint_file, LintOptions};
use lash_types::LashConfig;

let options = LintOptions {
    fix: true,
    interactive: false,
};
let config = LashConfig::default();

let diagnostics = lint_file(&path, &config, &options)?;

for diag in diagnostics.errors {
    println!("Error: {} at line {}", diag.message, diag.line);
}
```

### Formatter

**Location**: `crates/lash-core/src/formatter/`

**Purpose**: Normalize Markdown formatting for consistency

**Features**:
- Consistent indentation (configurable, default 2 spaces)
- Annotation ordering (alphabetical)
- Heading spacing
- Task tree structure preservation
- Diff mode (`--diff`) shows changes without applying

**Example**:

```bash
# Format file in place
lash format tasks.md

# Show diff without applying
lash format tasks.md --diff

# Check if formatting needed
lash format tasks.md --check
```

### Dependency Resolver

**Location**: `crates/lash-core/src/graph/`

**Purpose**: Build and analyze task dependency graphs

**Dependency Types**:

1. **Implicit hierarchy**: Parent tasks depend on children
   ```markdown
   - [ ] Parent
     - [ ] Child A
     - [ ] Child B
   ```

2. **Explicit cross-file**: Via `@depends-on`
   ```markdown
   @depends-on: path/to/file.md#task:id
   ```

3. **Directory-level**: File dependencies on subdirectories

**Key Operations**:
- `build_graph()` - Construct dependency graph
- `detect_cycles()` - Find circular dependencies
- `compute_closure()` - Transitive dependency closure
- `topological_sort()` - Valid execution order

**Example**:

```rust
use lash_core::graph::DependencyGraph;

let graph = DependencyGraph::from_files(&files)?;

// Check for cycles
if let Some(cycle) = graph.detect_cycles() {
    eprintln!("Cycle detected: {:?}", cycle);
}

// Get all dependencies of a task
let deps = graph.get_all_dependencies("task:foo");
```

### Indexing Engine

**Location**: `crates/lash-db/src/indexer.rs`

**Purpose**: Fast incremental indexing of task files into SQLite

**Features**:
- **Incremental**: Only re-parse changed files (via hash comparison)
- **Parallel**: Multi-threaded parsing with rayon
- **Profiled**: Built-in performance profiling
- **Transactional**: Atomic database updates

**Indexing Pipeline**:

```
1. Discovery    - Walk filesystem, enumerate .md files
2. Diff         - Compare hashes, identify changed files
3. Parse        - Parse changed files in parallel
4. Database     - Insert/update SQLite rows
5. Closure      - Rebuild dependency closure table
```

**Performance Profiling**:

```rust
use lash_db::{Indexer, IndexerConfig};

let config = IndexerConfig::new(root)
    .with_profiling(true);

let mut indexer = Indexer::new(&conn, config, &lash_config);
let report = indexer.index_project()?;

// Print performance breakdown
if let Some(profile) = report.profile {
    profile.print_summary();
    // Outputs:
    // Discovery: 5.2ms
    // Diff: 1.3ms
    // Parsing: 15.8ms (10 files, 1.58ms avg)
    // Database: 8.1ms (150 rows inserted)
    // Closure rebuild: 3.2ms
    // Total: 33.6ms
}
```

**Performance Targets** (all exceeded 8-12x):
- Small (10 files): <50ms (achieved: 10.5ms)
- Medium (100 files): <500ms (achieved: 61ms)
- Large (1000 files): <5s (achieved: 425ms)

### Query Engine

**Location**: `crates/lash-db/src/search.rs`, `crates/lash-db/src/lib.rs`

**Purpose**: Fast querying of indexed tasks

**Query Types**:

1. **Filtered list**: Query by label, status, owner, path
   ```rust
   let tasks = conn.query_tasks(QueryOptions {
       labels: Some(vec!["backend".into()]),
       status: Some(TaskStatus::Open),
       limit: Some(50),
       ..Default::default()
   })?;
   ```

2. **Full-text search**: FTS5-powered fuzzy search
   ```rust
   let results = search_tasks(&conn, "authentication", 20)?;
   for result in results {
       println!("{}: {}", result.rank, result.task.title);
   }
   ```

3. **Dependency queries**: Get dependencies/dependents
   ```rust
   let deps = get_task_dependencies(&conn, task_id)?;
   let rdeps = get_task_dependents(&conn, task_id)?;
   ```

**Search Features**:
- FTS5 full-text indexing
- Rank-based result ordering
- Search across: titles, bodies, notes, descriptions, file paths
- Configurable result limits

### CLI Framework

**Location**: `crates/lash-cli/src/`

**Purpose**: User-facing command-line interface

**Key Components**:

1. **Argument parsing** (`cli.rs`): Clap-based CLI definition
2. **Command dispatch** (`command.rs`): Route commands to handlers
3. **Error reporting** (`error_reporter.rs`): Rich terminal errors
4. **Configuration** (`config.rs`): Config file management
5. **Logging** (`logging.rs`): Tracing integration
6. **Progress** (`progress.rs`): Indicatif progress bars
7. **Theme** (`theme.rs`): Centralized colored output

**Adding a new command**:

```rust
// 1. Add to CLI definition (cli.rs)
#[derive(Subcommand)]
enum Command {
    #[command(about = "My new command")]
    MyCommand {
        #[arg(short, long)]
        option: String,
    },
}

// 2. Add handler (command.rs)
impl Command {
    pub fn execute(&self, ctx: &Context) -> Result<()> {
        match self {
            Command::MyCommand { option } => {
                execute_my_command(ctx, option)
            }
        }
    }
}

// 3. Implement logic
fn execute_my_command(ctx: &Context, option: &str) -> Result<()> {
    // Your implementation
    Ok(())
}
```

### TUI

**Location**: `crates/lash-tui/src/`

**Purpose**: Interactive terminal UI with ratatui

**Architecture**:

```
┌─────────────────────────────────────────────────────────────┐
│                         TUI App                             │
├──────────────────┬──────────────────────────────────────────┤
│   Event Loop     │           State Machine                  │
│  (Keyboard/      │         (Navigation,                     │
│   Mouse Input)   │          Selection)                      │
└────────┬─────────┴─────────────────┬────────────────────────┘
         │                           │
         ▼                           ▼
    Event Handler              Render Pipeline
         │                           │
         └───────────┬───────────────┘
                     ▼
              Terminal Output
             (crossterm/ratatui)
```

**Key Features**:
- 300+ color schemes (Gogh collection)
- Interactive task creation
- Keyboard-driven navigation
- Fuzzy search panel
- Dependency graph view

**State management** (`state.rs`):
- Separation of UI state from data state
- State transitions via actions
- Immutable state updates

### Agent Integration

**Location**: `crates/lash-agent/src/`

**Purpose**: Generate token-minimized prompts for LLM agents

**Key Features**:

1. **Schema generation**: JSON schema for agent tooling
2. **Prompt templates**: Pre-built prompts for common operations
3. **Context minimization**: Sparse, relevant-only context
4. **Token counting**: Estimate prompt token usage
5. **Format variations**: Plain text, JSON, Claude Code skills, Agents.md

**Example**:

```rust
use lash_agent::PromptGenerator;

let generator = PromptGenerator::new(&conn, &config);

// Generate minimal agent prompt
let prompt = generator.generate_prompt(PromptOptions {
    format: PromptFormat::Plain,
    label_filter: Some("backend"),
    include_examples: false,
    max_tokens: Some(4000),
})?;

println!("{}", prompt);
```

**Token optimization strategies**:
- ID-based references instead of full descriptions
- Summarized file headers
- Only include changed/relevant files
- Schema-first approach (minimal examples)

---

## Database Schema

### Overview

SQLite database (`<project>/.lash/index.db`) stores parsed task data for fast queries. Schema is fully versioned and migrated.

### Tables

#### `files`

Stores task file metadata.

```sql
CREATE TABLE files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,         -- Relative path from project root
    hash TEXT NOT NULL,                -- BLAKE3 content hash
    mtime INTEGER NOT NULL,            -- Last modified timestamp
    file_id TEXT,                      -- @id annotation
    title TEXT,                        -- File title (heading)
    description TEXT,                  -- ## Description section
    owner TEXT,                        -- @owner annotation
    created TEXT,                      -- @created annotation (ISO 8601)
    estimate TEXT,                     -- @estimate annotation
    agent_note TEXT,                   -- @agent-note annotation
    indexed_at INTEGER NOT NULL        -- When indexed
);

CREATE INDEX idx_files_path ON files(path);
CREATE INDEX idx_files_hash ON files(hash);
CREATE INDEX idx_files_status ON files(status);
```

#### `tasks`

Stores individual tasks within files.

```sql
CREATE TABLE tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id INTEGER NOT NULL,          -- FK to files.id
    task_id TEXT,                      -- @id annotation (unique within file)
    full_id TEXT NOT NULL UNIQUE,      -- path#task:id (globally unique)
    title TEXT NOT NULL,               -- Task title
    status TEXT NOT NULL,              -- open, done, waived, blocked
    depth INTEGER NOT NULL,            -- Nesting depth (0 = top-level)
    parent_id INTEGER,                 -- FK to tasks.id (NULL for top-level)
    order_index INTEGER NOT NULL,      -- Position within parent
    owner TEXT,                        -- @owner annotation
    estimate TEXT,                     -- @estimate annotation
    body TEXT,                         -- Additional task description
    line_number INTEGER,               -- Line in source file
    FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_id) REFERENCES tasks.id ON DELETE CASCADE
);

CREATE INDEX idx_tasks_file_id ON tasks(file_id);
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_parent_id ON tasks(parent_id);
CREATE INDEX idx_tasks_full_id ON tasks(full_id);
```

#### `contextual_notes`

Stores plain bullet points nested under tasks.

```sql
CREATE TABLE contextual_notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL,          -- FK to tasks.id
    content TEXT NOT NULL,             -- Note text
    order_index INTEGER NOT NULL,      -- Position within task
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

CREATE INDEX idx_notes_task_id ON contextual_notes(task_id);
```

#### `labels`

Normalized label storage.

```sql
CREATE TABLE labels (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE          -- Label name (e.g., "backend")
);

CREATE INDEX idx_labels_name ON labels(name);
```

#### `task_labels`

Many-to-many: tasks to labels.

```sql
CREATE TABLE task_labels (
    task_id INTEGER NOT NULL,
    label_id INTEGER NOT NULL,
    PRIMARY KEY (task_id, label_id),
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (label_id) REFERENCES labels(id) ON DELETE CASCADE
);

CREATE INDEX idx_task_labels_label_id ON task_labels(label_id);
```

#### `file_labels`

Many-to-many: files to labels.

```sql
CREATE TABLE file_labels (
    file_id INTEGER NOT NULL,
    label_id INTEGER NOT NULL,
    PRIMARY KEY (file_id, label_id),
    FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE,
    FOREIGN KEY (label_id) REFERENCES labels(id) ON DELETE CASCADE
);

CREATE INDEX idx_file_labels_label_id ON file_labels(label_id);
```

#### `dependencies`

Tracks task dependencies.

```sql
CREATE TABLE dependencies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_task_id INTEGER NOT NULL,     -- Dependent task
    to_task_id INTEGER NOT NULL,       -- Dependency target
    kind TEXT NOT NULL,                -- 'parent-child', 'explicit', 'file'
    FOREIGN KEY (from_task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (to_task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

CREATE INDEX idx_deps_from ON dependencies(from_task_id);
CREATE INDEX idx_deps_to ON dependencies(to_task_id);
CREATE INDEX idx_deps_kind ON dependencies(kind);
```

#### `dependency_closure`

Transitive closure of dependencies (for fast "all dependencies" queries).

```sql
CREATE TABLE dependency_closure (
    ancestor_id INTEGER NOT NULL,      -- Transitively depends on...
    descendant_id INTEGER NOT NULL,    -- ...this task
    depth INTEGER NOT NULL,            -- Path length
    PRIMARY KEY (ancestor_id, descendant_id),
    FOREIGN KEY (ancestor_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (descendant_id) REFERENCES tasks(id) ON DELETE CASCADE
);

CREATE INDEX idx_closure_ancestor ON dependency_closure(ancestor_id);
CREATE INDEX idx_closure_descendant ON dependency_closure(descendant_id);
```

#### Full-Text Search

FTS5 virtual table for fast text search.

```sql
CREATE VIRTUAL TABLE tasks_fts USING fts5(
    full_id UNINDEXED,                 -- Task identifier (not searchable)
    title,                             -- Task title (searchable)
    body,                              -- Task body (searchable)
    file_path,                         -- File path (searchable)
    notes,                             -- Contextual notes (searchable)
    description,                       -- File description (searchable)
    content=''                         -- External content table
);

-- Triggers to keep FTS in sync
CREATE TRIGGER tasks_fts_insert AFTER INSERT ON tasks ...
CREATE TRIGGER tasks_fts_update AFTER UPDATE ON tasks ...
CREATE TRIGGER tasks_fts_delete AFTER DELETE ON tasks ...
```

### Relationships

```
files (1) ──────── (many) tasks
  │                        │
  │                        ├── (many) contextual_notes
  │                        │
  │                        └── (many) task_labels ── labels
  │
  └── (many) file_labels ── labels

tasks (many) ───── (many) dependencies
  │                        │
  │                        └── kind: 'parent-child' | 'explicit' | 'file'
  │
  └── (recursive) parent_id → tasks.id

dependency_closure (computed from dependencies)
  ancestor_id → tasks.id
  descendant_id → tasks.id
```

### Schema Migrations

**Location**: `crates/lash-db/src/migrations.rs`

**Version tracking**:

```sql
CREATE TABLE schema_version (
    version INTEGER PRIMARY KEY,
    applied_at INTEGER NOT NULL
);
```

**Migration process**:
1. Check current version
2. Apply pending migrations in order
3. Update version table
4. Rebuild indexes and closure

**Adding a migration**:

```rust
// migrations.rs
pub fn run_migrations(conn: &Connection) -> Result<()> {
    let version = get_schema_version(conn)?;

    if version < 1 {
        migrate_v1(conn)?;
    }
    if version < 2 {
        migrate_v2(conn)?; // Your new migration
    }

    Ok(())
}

fn migrate_v2(conn: &Connection) -> Result<()> {
    conn.execute("ALTER TABLE tasks ADD COLUMN new_field TEXT", [])?;
    set_schema_version(conn, 2)?;
    Ok(())
}
```

### Query Patterns

**Get all tasks in a file**:

```sql
SELECT * FROM tasks
WHERE file_id = (SELECT id FROM files WHERE path = ?)
ORDER BY order_index;
```

**Search tasks by label**:

```sql
SELECT t.* FROM tasks t
JOIN task_labels tl ON t.id = tl.task_id
JOIN labels l ON tl.label_id = l.id
WHERE l.name = ?;
```

**Full-text search**:

```sql
SELECT t.*, rank FROM tasks t
JOIN tasks_fts fts ON t.full_id = fts.full_id
WHERE tasks_fts MATCH ?
ORDER BY rank
LIMIT ?;
```

**Get all dependencies (transitive)**:

```sql
SELECT t.* FROM tasks t
JOIN dependency_closure dc ON t.id = dc.descendant_id
WHERE dc.ancestor_id = ?;
```

---

## Error Handling

### Error Taxonomy

Lash uses a comprehensive error taxonomy with 25+ documented error codes. See [error-codes.md](./error-codes.md) for the complete catalog.

**Error categories**:
- `E_PARSE_*` - Parsing errors (10 codes)
- `E_LINT_*` - Linting errors (7 codes)
- `E_DEP_*` - Dependency errors (3 codes)
- `E_IO_*` - I/O errors (4 codes)
- `E_DB_*` - Database errors (4 codes)
- `E_CFG_*` - Configuration errors (4 codes)
- `E_CREATE_*` - Task creation errors (13 codes)

### Error Types

**Location**: `crates/lash-types/src/error.rs`

```rust
#[derive(Debug, thiserror::Error)]
pub enum LashError {
    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Lint error: {0}")]
    Lint(String),

    #[error("Dependency error: {0}")]
    Dependency(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Task creation error: {0}")]
    TaskCreation(String),
}

pub type LashResult<T> = Result<T, LashError>;
```

### Error Formatting

Lash uses `miette` for rich error reporting with context and suggestions.

**Example error output**:

```
Error: E_LINT_DEPTH_EXCEEDED

  × Task nesting exceeds maximum allowed depth
   ╭─[tasks.md:42:1]
 42 │         - [ ] Too deeply nested task
    │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ exceeds max depth (3)
   ╰────
  help: Reduce nesting depth or adjust max_depth in config

  Caused by:
      Maximum depth is 3 levels, but found 4
```

**Formatting strategies**:

1. **Terminal output** (default): Colored, formatted with miette
2. **JSON output** (`--json`): Machine-readable for scripts/agents
3. **Plain text**: No colors, for piping/logs

**Error reporter** (`crates/lash-cli/src/error_reporter.rs`):

```rust
pub fn report_error(err: &LashError, format: OutputFormat) {
    match format {
        OutputFormat::Human => {
            // Rich terminal output with miette
            eprintln!("{:?}", miette::Report::new(err));
        }
        OutputFormat::Json => {
            // Machine-readable JSON
            let json = serde_json::json!({
                "error": {
                    "code": err.code(),
                    "message": err.to_string(),
                    "details": err.details(),
                }
            });
            println!("{}", json);
        }
        OutputFormat::Plain => {
            // Simple text
            eprintln!("Error: {}", err);
        }
    }
}
```

### Error Codes

Each error has a stable code for programmatic handling:

```rust
impl LashError {
    pub fn code(&self) -> &'static str {
        match self {
            LashError::Parse(_) => "E_PARSE",
            LashError::Lint(msg) if msg.contains("depth") => "E_LINT_DEPTH_EXCEEDED",
            LashError::Lint(msg) if msg.contains("duplicate") => "E_LINT_DUPLICATE_ID",
            // ... more specific codes
            _ => "E_UNKNOWN",
        }
    }
}
```

**CLI error explanation**:

```bash
# Explain a specific error code
lash explain E_LINT_DEPTH_EXCEEDED

# List all error codes
lash explain --list
```

### Exit Codes

```rust
pub enum ExitCode {
    Success = 0,
    GeneralError = 1,
    LintError = 2,
    DependencyError = 3,
    IoError = 4,
    DatabaseError = 5,
    ConfigError = 6,
}
```

**Usage in CLI**:

```rust
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::Success,
        Err(e @ LashError::Lint(_)) => {
            report_error(&e);
            ExitCode::LintError
        }
        Err(e) => {
            report_error(&e);
            ExitCode::GeneralError
        }
    }
}
```

---

## Testing Strategy

See [TESTING.md](./TESTING.md) for comprehensive testing documentation.

### Test Layers

Lash employs a multi-layered testing strategy:

```
┌─────────────────────────────────────────────────────────────┐
│              End-to-End Tests (E2E)                         │
│  Full CLI workflows, user scenarios                         │
│  Location: crates/lash-cli/tests/e2e_*.rs                  │
│  Tool: assert_cmd                                           │
└─────────────────────────────────────────────────────────────┘
                            ▲
                            │
┌─────────────────────────────────────────────────────────────┐
│           Integration Tests                                 │
│  Multi-component interactions                               │
│  Location: crates/*/tests/*.rs                             │
│  Tool: tempfile, rstest                                     │
└─────────────────────────────────────────────────────────────┘
                            ▲
                            │
┌─────────────────────────────────────────────────────────────┐
│              Unit Tests                                     │
│  Single function/module behavior                            │
│  Location: #[cfg(test)] mod tests                          │
│  Tool: built-in Rust test framework                         │
└─────────────────────────────────────────────────────────────┘
                            ▲
                            │
┌─────────────────────────────────────────────────────────────┐
│              Doc Tests                                      │
│  API examples as tests                                      │
│  Location: /// code blocks in docs                         │
│  Tool: cargo test --doc                                     │
└─────────────────────────────────────────────────────────────┘
```

### Unit Tests

**Characteristics**:
- Fast (<1ms each)
- Isolated (no I/O, network, database)
- Test single functions/methods
- Colocated with source code

**Example**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_checkbox_open() {
        let input = "- [ ] Task";
        let result = parse_checkbox(input);
        assert_eq!(result.unwrap().status, TaskStatus::Open);
    }

    #[test]
    fn parse_checkbox_invalid() {
        let input = "- [?] Invalid";
        assert!(parse_checkbox(input).is_err());
    }
}
```

**Best practices**:
- One assertion per test (or closely related assertions)
- Descriptive test names: `test_parse_checkbox_with_labels`
- Use `#[should_panic]` sparingly (prefer `Result` assertions)
- Test edge cases and error paths

### Integration Tests

**Characteristics**:
- Slower (I/O, database setup)
- Test component interactions
- Use temporary files/databases
- Located in `tests/` directory

**Example**:

```rust
// crates/lash-db/tests/indexing_tests.rs
use lash_db::{init_database, Indexer};
use tempfile::TempDir;

#[test]
fn test_incremental_indexing() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");

    // Initial index
    let conn = init_database(&db_path).unwrap();
    let mut indexer = Indexer::new(&conn, config, lash_config);
    let report1 = indexer.index_project().unwrap();
    assert_eq!(report1.files_indexed, 5);

    // No changes - should be fast
    let report2 = indexer.index_project().unwrap();
    assert_eq!(report2.files_indexed, 0); // All cached
}
```

**Test fixtures**: `crates/lash-cli/tests/fixtures/`
- `valid/` - Valid task files
- `invalid/` - Files with specific errors
- `repos/` - Complete project structures

### End-to-End Tests

**Characteristics**:
- Test full CLI workflows
- Use actual binary (`lash`)
- Validate user experience
- Snapshot testing with `insta`

**Example**:

```rust
// crates/lash-cli/tests/e2e_cli_tests.rs
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_lint_command() {
    let mut cmd = Command::cargo_bin("lash").unwrap();

    cmd.arg("lint")
       .arg("tests/fixtures/valid/simple.md")
       .assert()
       .success()
       .stdout(predicate::str::contains("No errors"));
}

#[test]
fn test_list_with_label_filter() {
    let mut cmd = Command::cargo_bin("lash").unwrap();

    cmd.current_dir("tests/fixtures/repos/complete")
       .arg("list")
       .arg("--label")
       .arg("backend")
       .assert()
       .success();

    // Snapshot test the output
    insta::assert_snapshot!(cmd.output().unwrap().stdout);
}
```

### Doctests

**Purpose**: API examples that double as tests

**Example**:

```rust
/// Parse a task file from a string
///
/// # Example
///
/// ```
/// use lash_core::parser::parse_file_from_string;
/// use lash_types::LashConfig;
///
/// let content = "# Test\n\n## Tasks\n\n- [ ] Task 1\n";
/// let config = LashConfig::default();
/// let result = parse_file_from_string(content, &config);
///
/// assert!(result.is_ok());
/// let task_file = result.unwrap();
/// assert_eq!(task_file.tasks.len(), 1);
/// ```
pub fn parse_file_from_string(content: &str, config: &LashConfig) -> Result<TaskFile> {
    // Implementation
}
```

**Guidelines**:
- All public APIs should have doctests
- Use `#` prefix to hide boilerplate
- Use `no_run` for examples requiring I/O
- Avoid `ignore` unless absolutely necessary

### Benchmarks

**Purpose**: Track performance over time

**Tools**: Criterion (statistical benchmarking)

**Example**:

```rust
// crates/lash-core/benches/parser_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use lash_core::parser::parse_file;

fn bench_parse_small_file(c: &mut Criterion) {
    let content = include_str!("../tests/fixtures/small.md");

    c.bench_function("parse_small_file", |b| {
        b.iter(|| {
            parse_file_from_string(black_box(content), &config)
        });
    });
}

criterion_group!(benches, bench_parse_small_file);
criterion_main!(benches);
```

**Running benchmarks**:

```bash
cargo bench                          # All benchmarks
cargo bench -- parse                 # Filter by name
cargo bench -- --save-baseline main  # Save baseline
cargo bench -- --baseline main       # Compare to baseline
```

### Coverage Targets

- **Overall**: >80% line coverage
- **Critical modules** (parser, linter, graph): >90%
- **Less critical** (TUI, agent): >70%

**Generate coverage**:

```bash
cargo llvm-cov --workspace --html
open target/llvm-cov/html/index.html
```

### Test Organization

```
crates/lash-cli/
├── src/
│   └── command.rs                 # Implementation
├── tests/
│   ├── common/
│   │   └── mod.rs                 # Shared test utilities
│   ├── fixtures/
│   │   ├── valid/                 # Valid test data
│   │   ├── invalid/               # Error test cases
│   │   └── repos/                 # Full project structures
│   ├── e2e_cli_tests.rs           # End-to-end tests
│   └── integration_tests.rs       # Integration tests
└── benches/
    └── cli_bench.rs               # Benchmarks
```

---

## Contributing

We welcome contributions! Please read this guide before submitting PRs.

### Getting Started

1. **Fork and clone**:
   ```bash
   git clone https://github.com/YOUR_USERNAME/lash.git
   cd lash
   ```

2. **Install pre-commit hooks**:
   ```bash
   ./scripts/install-pre-commit-hook.sh
   ```

3. **Create a feature branch**:
   ```bash
   git checkout -b feature/my-feature
   ```

4. **Make changes and test**:
   ```bash
   cargo test --workspace
   cargo clippy --workspace -- -D warnings
   cargo fmt --check
   ```

5. **Commit and push**:
   ```bash
   git add .
   git commit -m "Add feature: description"
   git push origin feature/my-feature
   ```

6. **Open a pull request** on GitHub

### Code Style

Lash enforces strict code quality standards:

#### Rust Style

**Formatting**: `rustfmt` with default settings

```bash
cargo fmt --all
```

**Linting**: `clippy` in pedantic mode, zero warnings allowed

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

**Common clippy rules**:
- `clippy::all` - All standard lints
- `clippy::pedantic` - Extra lints for code quality
- Allowed exceptions (rare):
  ```rust
  #![allow(clippy::module_name_repetitions)] // When justified
  ```

#### Naming Conventions

- **Variables**: `snake_case`
- **Functions**: `snake_case`
- **Types**: `PascalCase`
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Modules**: `snake_case`

#### Documentation

All public APIs must have:
- Doc comments (`///` not `//`)
- Examples (preferably as doctests)
- Description of parameters and return values
- Links to related items

```rust
/// Parse a task file from disk
///
/// This function reads the file at `path`, parses it according to the
/// Lash Markdown format, and returns a structured `TaskFile`.
///
/// # Arguments
///
/// * `path` - Path to the Markdown file
/// * `config` - Configuration for parsing behavior
///
/// # Returns
///
/// Returns `Ok(TaskFile)` on success, or `Err(LashError)` if parsing fails.
///
/// # Errors
///
/// - `E_IO_FILE_NOT_FOUND` - File does not exist
/// - `E_PARSE_BAD_CHECKBOX` - Invalid checkbox syntax
///
/// # Example
///
/// ```no_run
/// use lash_core::parser::parse_file;
/// use lash_types::LashConfig;
/// use std::path::PathBuf;
///
/// let path = PathBuf::from("tasks.md");
/// let config = LashConfig::default();
/// let task_file = parse_file(&path, &config)?;
/// # Ok::<(), lash_types::LashError>(())
/// ```
pub fn parse_file(path: &Path, config: &LashConfig) -> Result<TaskFile> {
    // Implementation
}
```

#### Error Handling

- Use `Result<T, LashError>` for fallible operations
- Provide context with error messages
- Use appropriate error codes from taxonomy
- Never `unwrap()` or `expect()` in production code
  - Exception: In tests, use `unwrap()` freely

```rust
// Good
fn read_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .map_err(|e| LashError::Io(format!("Failed to read {}: {}", path.display(), e)))
}

// Bad - loses context
fn read_file(path: &Path) -> Result<String> {
    Ok(std::fs::read_to_string(path).unwrap()) // Don't do this
}
```

### PR Process

1. **Title**: Use conventional commit format
   - `feat: Add new feature`
   - `fix: Resolve bug`
   - `docs: Update documentation`
   - `refactor: Improve code structure`
   - `test: Add tests`
   - `perf: Performance improvement`

2. **Description**: Explain what and why
   - What does this PR do?
   - Why is this change needed?
   - How does it work?
   - Any breaking changes?

3. **Checklist**:
   - [ ] Tests pass (`cargo test --workspace`)
   - [ ] Clippy passes (`cargo clippy --workspace -- -D warnings`)
   - [ ] Code is formatted (`cargo fmt --check`)
   - [ ] Doctests added for new public APIs
   - [ ] Error codes documented (if new errors)
   - [ ] Benchmarks run (if performance-critical)
   - [ ] Documentation updated

4. **Review**: Maintainers will review and provide feedback

5. **Merge**: Squash-merge to main after approval

### Review Criteria

PRs are evaluated on:

1. **Correctness**: Does it work? Are tests comprehensive?
2. **Code quality**: Follows style guide, no clippy warnings
3. **Performance**: No regressions, benchmarks for hot paths
4. **Documentation**: Public APIs documented, examples provided
5. **Maintainability**: Clear code, appropriate abstractions
6. **Scope**: Focused changes, single responsibility

### What to Contribute

**Good first issues**:
- Documentation improvements
- Additional test coverage
- Bug fixes with test cases
- CLI output improvements
- New color schemes for TUI

**Larger contributions**:
- New lint rules
- Performance optimizations
- New CLI commands
- TUI features
- Agent prompt improvements

**Before starting large work**:
- Open an issue to discuss the approach
- Get feedback from maintainers
- Break into smaller PRs if possible

---

## Release Process

### Versioning

Lash follows [Semantic Versioning 2.0.0](https://semver.org/):

- **Major** (x.0.0): Breaking changes
- **Minor** (0.x.0): New features, backwards compatible
- **Patch** (0.0.x): Bug fixes, backwards compatible

**Examples**:
- `0.1.0` → `0.2.0`: Added `lash add` command (new feature)
- `0.2.0` → `0.2.1`: Fixed crash in `lash list` (bug fix)
- `0.9.0` → `1.0.0`: Changed annotation format (breaking)

### Changelog

Location: `CHANGELOG.md`

Format: [Keep a Changelog](https://keepachangelog.com/)

```markdown
# Changelog

## [Unreleased]
### Added
- New feature X

### Changed
- Improved Y

### Fixed
- Bug Z

## [0.2.0] - 2024-01-15
### Added
- Task creation with `lash add`
- Interactive mode for linting

### Changed
- Improved error messages

### Fixed
- Crash when parsing empty files
```

**Categories**:
- **Added**: New features
- **Changed**: Changes in existing functionality
- **Deprecated**: Soon-to-be removed features
- **Removed**: Removed features
- **Fixed**: Bug fixes
- **Security**: Security fixes

### Release Steps

1. **Update version** in `Cargo.toml`:
   ```toml
   [workspace.package]
   version = "0.2.0"
   ```

2. **Update `CHANGELOG.md`**:
   - Move `[Unreleased]` changes to new version section
   - Add release date
   - Create new empty `[Unreleased]` section

3. **Run full test suite**:
   ```bash
   cargo test --workspace --all-targets
   cargo test --doc
   cargo clippy --workspace -- -D warnings
   cargo fmt --check
   ```

4. **Run benchmarks** (verify no regressions):
   ```bash
   cargo bench --workspace -- --save-baseline release-0.2.0
   ```

5. **Build release binaries**:
   ```bash
   cargo build --release
   ```

6. **Tag release**:
   ```bash
   git tag -a v0.2.0 -m "Release v0.2.0"
   git push origin v0.2.0
   ```

7. **Publish to crates.io**:
   ```bash
   # Publish in dependency order
   cargo publish -p lash-types
   cargo publish -p lash-core
   cargo publish -p lash-db
   cargo publish -p lash-agent
   cargo publish -p lash-tui
   cargo publish -p lash-cli
   ```

8. **Create GitHub release**:
   - Go to GitHub Releases
   - Create release from tag
   - Copy changelog section
   - Upload release binaries (optional)

### Publishing

**Prerequisites**:
- crates.io account
- `cargo login` with API token
- Maintainer permissions

**Package preparation**:
- Update `Cargo.toml` metadata
- Include `README.md`, `LICENSE`, `CHANGELOG.md`
- Exclude test fixtures: `exclude = ["tests/fixtures/*"]`

**Verify before publishing**:
```bash
cargo package --list  # Check included files
cargo package --allow-dirty  # Create .crate file
tar -tzf target/package/lash-cli-0.2.0.crate  # Inspect contents
```

### Post-Release

1. **Update documentation** site (if applicable)
2. **Announce** release:
   - GitHub Discussions
   - Twitter/social media
   - Relevant forums
3. **Monitor** for issues in first 24-48 hours
4. **Plan** next release cycle

---

## Additional Resources

- [Design Document](./design-doc.md) - Complete technical specification
- [Error Codes](./error-codes.md) - Comprehensive error catalog
- [Testing Guide](./TESTING.md) - Detailed testing documentation
- [Task Tracking](../tasks/tasks.md) - Current development roadmap
- [Development Log](../devlog.md) - Progress and decisions

### External Documentation

- [Rust Book](https://doc.rust-lang.org/book/) - Rust language guide
- [Cargo Book](https://doc.rust-lang.org/cargo/) - Cargo package manager
- [clap](https://docs.rs/clap/) - CLI argument parsing
- [rusqlite](https://docs.rs/rusqlite/) - SQLite bindings
- [ratatui](https://docs.rs/ratatui/) - Terminal UI framework
- [criterion](https://docs.rs/criterion/) - Benchmarking
- [miette](https://docs.rs/miette/) - Error reporting

---

## FAQ

### How do I add a new CLI command?

1. Define in `crates/lash-cli/src/cli.rs` (add to `Command` enum)
2. Add handler in `crates/lash-cli/src/command.rs`
3. Implement logic (may involve other crates)
4. Add E2E tests in `crates/lash-cli/tests/`
5. Update `README.md` usage section

### How do I add a new lint rule?

1. Add error code to `crates/lash-types/src/error.rs`
2. Implement check in `crates/lash-core/src/linter/`
3. Add test cases (valid and invalid fixtures)
4. Document in `docs/error-codes.md`
5. Add to linter test suite

### How do I optimize database queries?

1. Add benchmark in `crates/lash-db/benches/`
2. Profile with `cargo bench`
3. Check EXPLAIN QUERY PLAN in SQLite
4. Add indexes if needed
5. Re-run benchmark to verify improvement

### How do I debug indexing performance?

Enable profiling:

```rust
let config = IndexerConfig::new(root).with_profiling(true);
let report = indexer.index_project()?;
report.profile.unwrap().print_summary();
```

### Where do I add new task file features?

1. Update `docs/design-doc.md` with spec
2. Modify parser in `lash-core`
3. Update database schema in `lash-db` (add migration)
4. Add linter rules as needed
5. Update formatter for pretty-printing
6. Add tests at all layers

---

**Happy contributing!** If you have questions, open a GitHub Discussion or reach out to maintainers.
