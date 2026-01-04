# Lash: Rust Architecture Recommendations

**Version:** 0.1
**Date:** 2025-11-17
**Purpose:** Technical implementation guidance for Lash based on design document analysis

---

## Executive Summary

This document provides detailed architectural recommendations for implementing Lash in Rust, covering crate structure, dependency selection, core data structures, performance optimization strategies, error handling, testing approaches, and implementation sequencing.

**Key Recommendations:**
- Proposed crate structure is sound with minor refinements
- Use pulldown-cmark for parsing, clap for CLI, rusqlite for DB, ratatui for TUI
- Implement arena-based allocation for task trees to optimize memory and performance
- Focus on thiserror for library crates, anyhow for CLI error contexts
- Start with a "vertical slice" approach: lint → parse → index → query
- Performance-critical paths: parsing, indexing, and dependency resolution need careful optimization

---

## 1. Crate Structure Analysis

### 1.1 Proposed Structure Validation

The proposed five-crate structure is **fundamentally sound** and follows Rust best practices for separation of concerns:

```
lash/
├── lash-cli/          # Binary crate (application entry point)
├── lash-core/         # Library crate (core domain logic)
├── lash-db/           # Library crate (persistence layer)
├── lash-tui/          # Library crate (TUI implementation)
└── lash-agent/        # Library crate (agent utilities)
```

**Strengths:**
- Clear separation between domain logic (core), persistence (db), and presentation (cli/tui)
- Agent utilities isolated for focused token-minimization work
- Binary crate (cli) acts as thin integration layer

### 1.2 Recommended Refinements

**Add a `lash-types` crate:**

Create a foundational `lash-types` crate to house shared types, avoiding circular dependencies:

```
lash-types/            # Shared types, no dependencies on other lash crates
├── src/
│   ├── lib.rs
│   ├── task.rs        # Task status, ID types
│   ├── labels.rs      # Label types
│   ├── annotations.rs # Annotation key types
│   └── location.rs    # File path, line/column types
```

**Rationale:**
- `lash-core`, `lash-db`, and `lash-agent` all need access to fundamental types
- Prevents circular dependency issues
- Allows each crate to depend on a stable, minimal interface
- Easier to maintain API stability across the project

**Alternative approach (if keeping to 5 crates):**
- Define all shared types in `lash-core` and have other crates depend on it
- This works but creates a heavier dependency graph
- Acceptable if you want to minimize crate count

### 1.3 Recommended Crate Structure

```
lash/
├── Cargo.toml         # Workspace manifest
├── lash-types/        # NEW: Shared types and traits
├── lash-core/         # Parsing, linting, validation, dependency graph
├── lash-db/           # SQLite schema, indexing, queries
├── lash-agent/        # Prompt generation, token minimization
├── lash-tui/          # Terminal UI (depends on core + db)
└── lash-cli/          # CLI binary (depends on all)
```

**Dependency graph:**
```
lash-cli
  ├─→ lash-core
  ├─→ lash-db
  ├─→ lash-agent
  ├─→ lash-tui
  └─→ lash-types

lash-tui
  ├─→ lash-core
  ├─→ lash-db
  └─→ lash-types

lash-core
  ├─→ lash-types
  └─→ lash-db (optional, for cross-file dependency resolution)

lash-db
  └─→ lash-types

lash-agent
  ├─→ lash-core
  ├─→ lash-db
  └─→ lash-types
```

### 1.4 API Boundaries and Visibility

**lash-types:**
- Public API: Core types (TaskId, TaskStatus, Label, SourceLocation, etc.)
- All types are `pub` since this is a foundational crate
- No internal implementation details

**lash-core:**
- Public API:
  - `Parser` - parse Markdown to AST
  - `Linter` - validate files and return diagnostics
  - `Task`, `TaskFile` - domain models
  - `DependencyGraph` - dependency resolution
- Private:
  - Parsing internals, visitor patterns, validation rules

**lash-db:**
- Public API:
  - `Database` - main database handle
  - `Indexer` - walk files and build index
  - `Query` - search and filter interface
- Private:
  - Schema details, SQL generation, migration logic

**lash-agent:**
- Public API:
  - `PromptGenerator` - generate agent prompts
  - `ContextMinimizer` - token reduction utilities
  - `AgentSchema` - JSON schema definitions
- Private:
  - Template rendering, summarization logic

**lash-tui:**
- Public API:
  - `Tui::run()` - launch TUI
  - Possibly configuration types
- Private:
  - All UI components, event handling, rendering

**lash-cli:**
- No public API (binary crate)
- All code is internal to the CLI application

---

## 2. Key Dependencies and Libraries

### 2.1 Markdown Parsing

**Recommendation: pulldown-cmark**

```toml
[dependencies]
pulldown-cmark = "0.12"
```

**Rationale:**
- **Most widely used** Markdown parser in Rust ecosystem
- **CommonMark compliant** with GFM extensions available
- **Streaming, event-based API** - memory efficient for large files
- **Battle-tested** - used by mdBook, cargo-readme, and many others
- **Good performance** - zero-copy parsing where possible

**Alternative: comrak**
- Pros: Full GFM support, can modify AST
- Cons: Heavier, more complex API
- Use case: If you need to modify parsed Markdown (for auto-formatting)

**Recommendation:** Start with pulldown-cmark. If auto-formatting requires AST modification, evaluate comrak later.

**Implementation approach:**
```rust
use pulldown_cmark::{Parser, Event, Tag};

pub fn parse_task_file(markdown: &str) -> Result<TaskFile, ParseError> {
    let parser = Parser::new(markdown);
    let mut builder = TaskFileBuilder::new();

    for event in parser {
        match event {
            Event::Start(Tag::Item) => builder.start_task(),
            Event::Text(text) => builder.add_text(text.as_ref()),
            // ... handle other events
        }
    }

    builder.build()
}
```

### 2.2 CLI Parsing

**Recommendation: clap v4**

```toml
[dependencies]
clap = { version = "4.5", features = ["derive", "cargo", "env"] }
```

**Rationale:**
- **Industry standard** for Rust CLIs
- **Derive macros** reduce boilerplate significantly
- **Excellent error messages** out of the box
- **Supports subcommands** naturally (lash lint, lash index, etc.)
- **Shell completion generation** built-in
- **Environment variable support** for configuration

**Example structure:**
```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "lash")]
#[command(about = "Minimalist, Markdown-native task tracker", long_about = None)]
struct Cli {
    #[arg(long, global = true)]
    root: Option<PathBuf>,

    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Lint {
        #[arg(value_name = "PATH")]
        paths: Vec<PathBuf>,

        #[arg(long)]
        fix: bool,
    },
    Index,
    List {
        #[arg(long)]
        label: Option<String>,

        #[arg(long)]
        status: Option<String>,
    },
    // ... other commands
}
```

### 2.3 SQLite

**Recommendation: rusqlite**

```toml
[dependencies]
rusqlite = { version = "0.32", features = ["bundled", "blob", "functions"] }
```

**Rationale:**
- **Most mature** SQLite bindings for Rust
- **Bundled feature** includes SQLite library (no system dependency)
- **Type-safe** query construction with good ergonomics
- **Transaction support** with RAII guards
- **User-defined functions** for custom SQL operations
- **FTS5 support** for full-text search

**Configuration recommendations:**
```rust
use rusqlite::{Connection, OpenFlags};

fn open_database(path: &Path) -> Result<Connection> {
    let mut conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX, // Single-threaded for simplicity
    )?;

    // WAL mode for better concurrent read performance
    conn.pragma_update(None, "journal_mode", "WAL")?;

    // Reasonable synchronous setting (NORMAL is safe for application crashes)
    conn.pragma_update(None, "synchronous", "NORMAL")?;

    // Memory-mapped I/O for better performance
    conn.pragma_update(None, "mmap_size", 30000000000i64)?; // 30GB

    Ok(conn)
}
```

**Schema migration:** Use a simple version-based migration system:
```rust
const MIGRATIONS: &[&str] = &[
    include_str!("migrations/001_initial.sql"),
    include_str!("migrations/002_add_fts.sql"),
];

fn migrate(conn: &Connection) -> Result<()> {
    let version: i32 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))?;

    for (i, migration) in MIGRATIONS.iter().enumerate().skip(version as usize) {
        conn.execute_batch(migration)?;
        conn.pragma_update(None, "user_version", i + 1)?;
    }

    Ok(())
}
```

### 2.4 TUI Framework

**Recommendation: ratatui + crossterm**

```toml
[dependencies]
ratatui = "0.28"
crossterm = "0.28"
```

**Rationale:**
- **ratatui** is the actively maintained fork of tui-rs
- **Immediate-mode UI** - simple mental model, render on every frame
- **Flexible layout** system with constraints
- **Good widget library** - lists, tables, paragraphs, etc.
- **crossterm** provides portable terminal manipulation
- **Active community** with good documentation

**Alternative: Cursive**
- Pros: Higher-level, more batteries included
- Cons: Less flexible, retained-mode can be harder to reason about
- Use case: If you want faster initial TUI development

**Basic TUI structure:**
```rust
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{backend::CrosstermBackend, Terminal};

pub fn run_tui() -> Result<()> {
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
    terminal.clear()?;

    let mut app = App::new();

    loop {
        terminal.draw(|f| ui::render(f, &app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                // ... handle other keys
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    Ok(())
}
```

### 2.5 Fuzzy Search

**Recommendation: nucleo**

```toml
[dependencies]
nucleo = "0.5"
```

**Rationale:**
- **Fast** - optimized for interactive search
- **Flexible scoring** - configurable matching algorithms
- **Used by Helix editor** - proven in production
- **Simple API** - easier than skim or fuzzy-matcher

**Alternative: SQLite FTS5**
- Pros: Integrated with database, no additional dependencies
- Cons: Not truly "fuzzy", more full-text search
- Use case: If you want exact/prefix matching, not fuzzy

**Recommendation:** Use both:
- **nucleo** for interactive TUI fuzzy search (in-memory, fast)
- **FTS5** for CLI search with ranking (persistent, queryable)

**Implementation sketch:**
```rust
use nucleo::{Config, Nucleo, Utf32String};

pub fn fuzzy_search(items: &[String], query: &str) -> Vec<(usize, String)> {
    let mut matcher = Nucleo::new(
        Config::DEFAULT,
        Arc::new(parking_lot::Mutex::new(Vec::new())),
        None,
        1,
    );

    // Implementation details...
}
```

### 2.6 Other Essential Dependencies

**File walking:**
```toml
ignore = "0.4"  # Respects .gitignore, fast directory traversal
```

**Hashing:**
```toml
blake3 = "1.5"  # Fast, secure hashing for content detection
```

**Date/Time:**
```toml
chrono = "0.4"  # Date parsing and formatting
```

**Serialization:**
```toml
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

**Path utilities:**
```toml
camino = "1.1"  # UTF-8 paths (better than std::path for Markdown files)
```

---

## 3. Core Data Structures

### 3.1 Fundamental Types (lash-types)

```rust
// lash-types/src/lib.rs

/// Unique identifier for a task within a file
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskId(String);

impl TaskId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Globally unique task identifier (file path + local ID)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FullTaskId {
    pub file_path: Utf8PathBuf,
    pub local_id: TaskId,
}

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Open,
    Done,
    Waived,
    Blocked,
}

/// Label (cross-cutting tag)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Label(String);

/// Source location for error reporting
#[derive(Debug, Clone, Copy)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

/// Span of text in source
#[derive(Debug, Clone, Copy)]
pub struct Span {
    pub start: SourceLocation,
    pub end: SourceLocation,
}
```

### 3.2 Task Representation (lash-core)

**Challenge:** How to represent hierarchical tasks efficiently?

**Options considered:**

1. **Tree of Boxes** (simple, idiomatic)
   ```rust
   pub struct Task {
       pub id: Option<TaskId>,
       pub title: String,
       pub status: TaskStatus,
       pub children: Vec<Box<Task>>,
   }
   ```
   - Pros: Simple, idiomatic Rust
   - Cons: Lots of allocations, pointer chasing, cache-unfriendly

2. **Arena allocation** (performance-oriented)
   ```rust
   use typed_arena::Arena;

   pub struct Task<'a> {
       pub id: Option<TaskId>,
       pub title: String,
       pub status: TaskStatus,
       pub children: Vec<&'a Task<'a>>,
   }

   pub struct TaskFile<'a> {
       arena: Arena<Task<'a>>,
       pub root_tasks: Vec<&'a Task<'a>>,
   }
   ```
   - Pros: Single allocation, cache-friendly, fast traversal
   - Cons: Lifetime annotations, slightly more complex

3. **Indexed/Flat storage** (database-friendly)
   ```rust
   pub struct Task {
       pub id: TaskIndex,
       pub parent_id: Option<TaskIndex>,
       pub title: String,
       pub status: TaskStatus,
   }

   pub struct TaskFile {
       pub tasks: Vec<Task>,
   }
   ```
   - Pros: Easy to serialize to DB, simple iteration
   - Cons: Less ergonomic for tree operations, needs manual parent tracking

**Recommendation: Hybrid approach**

Use **arena allocation during parsing** for performance, then convert to **flat indexed form** for database storage:

```rust
// lash-core/src/task.rs

use typed_arena::Arena;

/// In-memory representation optimized for parsing and linting
pub struct TaskTree<'a> {
    arena: Arena<TaskNode<'a>>,
    root_tasks: Vec<&'a TaskNode<'a>>,
}

pub struct TaskNode<'a> {
    pub id: Option<TaskId>,
    pub title: String,
    pub status: TaskStatus,
    pub labels: Vec<Label>,
    pub children: Vec<&'a TaskNode<'a>>,
    pub depth: u8,
    pub span: Span,
}

impl<'a> TaskTree<'a> {
    /// Convert to flat representation for database storage
    pub fn flatten(&self) -> Vec<FlatTask> {
        let mut flat = Vec::new();
        let mut index = 0;

        for root in &self.root_tasks {
            self.flatten_node(root, None, &mut flat, &mut index);
        }

        flat
    }

    fn flatten_node(
        &self,
        node: &TaskNode<'a>,
        parent_index: Option<usize>,
        flat: &mut Vec<FlatTask>,
        index: &mut usize,
    ) {
        let current_index = *index;
        *index += 1;

        flat.push(FlatTask {
            index: current_index,
            parent_index,
            id: node.id.clone(),
            title: node.title.clone(),
            status: node.status,
            labels: node.labels.clone(),
            depth: node.depth,
        });

        for child in &node.children {
            self.flatten_node(child, Some(current_index), flat, index);
        }
    }
}

/// Flat representation for database and serialization
#[derive(Debug, Clone)]
pub struct FlatTask {
    pub index: usize,
    pub parent_index: Option<usize>,
    pub id: Option<TaskId>,
    pub title: String,
    pub status: TaskStatus,
    pub labels: Vec<Label>,
    pub depth: u8,
}
```

**Why this approach:**
- Parsing is fast (arena allocation, single memory region)
- Linting can traverse tree efficiently
- Database insertion is straightforward (flat structure)
- Best of both worlds

### 3.3 File Representation

```rust
// lash-core/src/file.rs

use camino::Utf8PathBuf;

pub struct TaskFile {
    pub path: Utf8PathBuf,
    pub metadata: FileMetadata,
    pub tasks: Vec<FlatTask>,
}

pub struct FileMetadata {
    pub id: Option<String>,
    pub labels: Vec<Label>,
    pub status: Option<TaskStatus>,
    pub owner: Option<String>,
    pub created: Option<chrono::NaiveDate>,
    pub depends_on: Vec<DependencyRef>,
}

pub enum DependencyRef {
    /// Reference to another file
    File(Utf8PathBuf),
    /// Reference to specific task in another file
    Task(FullTaskId),
}
```

### 3.4 Dependency Graph

```rust
// lash-core/src/dependency.rs

use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

pub struct DependencyGraph {
    graph: DiGraph<Node, Edge>,
    task_to_node: HashMap<FullTaskId, NodeIndex>,
}

pub enum Node {
    Task(FullTaskId),
    File(Utf8PathBuf),
}

pub enum Edge {
    /// Parent-child relationship within a file
    Hierarchy,
    /// Explicit cross-file dependency
    Explicit,
    /// Directory-level dependency
    Directory,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            task_to_node: HashMap::new(),
        }
    }

    pub fn add_task(&mut self, task_id: FullTaskId) -> NodeIndex {
        let node = self.graph.add_node(Node::Task(task_id.clone()));
        self.task_to_node.insert(task_id, node);
        node
    }

    pub fn add_dependency(&mut self, from: NodeIndex, to: NodeIndex, kind: Edge) {
        self.graph.add_edge(from, to, kind);
    }

    /// Check if task is blocked by incomplete dependencies
    pub fn is_blocked(&self, task_id: &FullTaskId) -> bool {
        // Implementation using graph traversal
        todo!()
    }

    /// Detect cycles in the dependency graph
    pub fn detect_cycles(&self) -> Vec<Vec<FullTaskId>> {
        use petgraph::algo::tarjan_scc;

        tarjan_scc(&self.graph)
            .into_iter()
            .filter(|component| component.len() > 1)
            .map(|component| {
                component
                    .into_iter()
                    .filter_map(|idx| match &self.graph[idx] {
                        Node::Task(id) => Some(id.clone()),
                        _ => None,
                    })
                    .collect()
            })
            .collect()
    }
}
```

**Dependency: petgraph**
```toml
petgraph = "0.6"  # Graph data structures and algorithms
```

---

## 4. Performance-Critical Paths

### 4.1 Parsing and Linting

**Critical for:** Pre-commit hooks must be fast (<100ms for typical files)

**Optimization strategies:**

1. **Streaming parsing** - Use pulldown-cmark's event-based API to avoid building full AST
2. **Early exit** - Stop parsing on first error in strict mode
3. **Parallel file processing** - Use rayon for multi-file linting
4. **Incremental linting** - Only lint changed files (track mtimes)

```rust
use rayon::prelude::*;

pub fn lint_files(paths: &[Utf8PathBuf]) -> Vec<LintResult> {
    paths
        .par_iter()
        .map(|path| lint_file(path))
        .collect()
}

fn lint_file(path: &Utf8Path) -> LintResult {
    let content = std::fs::read_to_string(path)?;
    let mut linter = Linter::new();

    // Stream events without building full AST
    for event in Parser::new(&content) {
        linter.process_event(event)?;
    }

    linter.finish()
}
```

**Dependencies:**
```toml
rayon = "1.8"  # Data parallelism
```

### 4.2 Indexing

**Critical for:** Large repositories (100s of files) need fast index rebuilds

**Optimization strategies:**

1. **Content hashing** - Skip unchanged files using BLAKE3 hashes
2. **Parallel walking** - Use `ignore` crate with parallel directory traversal
3. **Batched inserts** - Insert multiple tasks in single transaction
4. **Prepared statements** - Reuse compiled SQL queries

```rust
use ignore::WalkBuilder;

pub fn index_repository(root: &Utf8Path, db: &mut Database) -> Result<IndexStats> {
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .build_parallel();

    let (sender, receiver) = crossbeam_channel::unbounded();

    // Parallel file discovery and parsing
    walker.run(|| {
        let sender = sender.clone();
        Box::new(move |entry| {
            if let Ok(entry) = entry {
                if entry.path().extension() == Some("md") {
                    let file = parse_file(entry.path());
                    sender.send(file).unwrap();
                }
            }
            ignore::WalkState::Continue
        })
    });

    drop(sender);

    // Single-threaded database insertion with batched transactions
    let tx = db.transaction()?;
    let mut stmt = tx.prepare(INSERT_TASK_SQL)?;

    for file in receiver {
        for task in file.tasks {
            stmt.execute(/* params */)?;
        }
    }

    tx.commit()?;

    Ok(IndexStats { /* ... */ })
}
```

**Dependencies:**
```toml
crossbeam-channel = "0.5"  # Better channels for parallel work
```

### 4.3 Search and Filtering

**Critical for:** Interactive TUI responsiveness

**Optimization strategies:**

1. **FTS5 indexes** - SQLite full-text search for keyword queries
2. **Covering indexes** - Index all fields used in common queries
3. **In-memory caching** - Cache frequently accessed data in TUI
4. **Lazy loading** - Only load visible tasks in TUI

```rust
// Covering index example
const CREATE_INDEX_SQL: &str = r#"
    CREATE INDEX IF NOT EXISTS idx_tasks_status_labels
    ON tasks(status, labels);
"#;

// Query using covering index
pub fn query_tasks_by_status_and_label(
    db: &Database,
    status: TaskStatus,
    label: &str,
) -> Result<Vec<Task>> {
    let mut stmt = db.prepare(
        "SELECT id, title, status, labels
         FROM tasks
         WHERE status = ?1 AND labels LIKE ?2"
    )?;

    // This will use the covering index efficiently
    stmt.query_map([status, format!("%{}%", label)], |row| {
        Ok(Task { /* ... */ })
    })
}
```

### 4.4 Dependency Resolution

**Critical for:** Accurate task completion status

**Optimization strategies:**

1. **Graph caching** - Build dependency graph once, reuse
2. **Topological ordering** - Process tasks in dependency order
3. **Cycle detection** - Use Tarjan's SCC algorithm (O(V+E))
4. **Memoization** - Cache "is blocked" checks

```rust
use std::collections::HashMap;

pub struct DependencyResolver {
    graph: DependencyGraph,
    blocked_cache: HashMap<FullTaskId, bool>,
}

impl DependencyResolver {
    pub fn is_blocked(&mut self, task_id: &FullTaskId) -> bool {
        if let Some(&cached) = self.blocked_cache.get(task_id) {
            return cached;
        }

        let result = self.compute_blocked(task_id);
        self.blocked_cache.insert(task_id.clone(), result);
        result
    }

    fn compute_blocked(&self, task_id: &FullTaskId) -> bool {
        // Use BFS to check if any dependency is incomplete
        // ...
    }

    pub fn invalidate_cache(&mut self, task_id: &FullTaskId) {
        self.blocked_cache.remove(task_id);
        // Also invalidate dependents
    }
}
```

### 4.5 Benchmarking Strategy

Use `criterion` for performance benchmarks:

```toml
[dev-dependencies]
criterion = "0.5"
```

**Key benchmarks to track:**
- Parse single file (100 tasks)
- Lint single file (100 tasks)
- Index repository (100 files, 10,000 tasks)
- Fuzzy search (1,000 results)
- Dependency resolution (deep tree)

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_parse_file(c: &mut Criterion) {
    let content = include_str!("../fixtures/large_file.md");

    c.bench_function("parse_file_100_tasks", |b| {
        b.iter(|| parse_file(black_box(content)))
    });
}

criterion_group!(benches, bench_parse_file);
criterion_main!(benches);
```

---

## 5. Error Handling Strategy

### 5.1 Error Types Architecture

**Recommendation:**
- Use **thiserror** for library crates (lash-core, lash-db, lash-agent, lash-tui)
- Use **anyhow** for the binary crate (lash-cli)

```toml
# Library crates
[dependencies]
thiserror = "1.0"

# Binary crate
[dependencies]
anyhow = "1.0"
```

**Rationale:**
- Libraries should have **strongly-typed errors** that callers can match on
- CLI can use **anyhow::Error** for ergonomic error propagation and context
- This is the recommended pattern from the Rust error handling working group

### 5.2 Error Types Design

**lash-core errors:**
```rust
// lash-core/src/error.rs

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Invalid task status '{status}' at {location}")]
    InvalidStatus {
        status: String,
        location: SourceLocation,
    },

    #[error("Task depth ({depth}) exceeds maximum ({max}) at {location}")]
    DepthLimitExceeded {
        depth: u8,
        max: u8,
        location: SourceLocation,
    },

    #[error("Duplicate task ID '{id}' at {location}")]
    DuplicateId {
        id: String,
        location: SourceLocation,
    },

    #[error("I/O error reading file: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum LintError {
    #[error("Linter diagnostic: {diagnostic}")]
    Diagnostic { diagnostic: Diagnostic },

    #[error("Parse error: {0}")]
    Parse(#[from] ParseError),
}

/// Structured diagnostic for machine-readable output
#[derive(Debug, Clone, serde::Serialize)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub severity: Severity,
    pub message: String,
    pub location: SourceLocation,
    pub file: Utf8PathBuf,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub enum DiagnosticCode {
    E001_InvalidStatus,
    E002_DepthLimitExceeded,
    E003_DuplicateId,
    E004_UnknownAnnotation,
    E005_BrokenDependency,
    // ... stable codes for all error types
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub enum Severity {
    Error,
    Warning,
    Info,
}
```

**lash-db errors:**
```rust
// lash-db/src/error.rs

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Database version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: i32, found: i32 },

    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),

    #[error("Task not found: {0}")]
    TaskNotFound(FullTaskId),
}
```

**lash-cli error handling:**
```rust
// lash-cli/src/main.rs

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Lint { paths, .. } => {
            lint_command(&paths)
                .context("Failed to lint files")?;
        }
        Commands::Index => {
            index_command()
                .context("Failed to build index")?;
        }
        // ...
    }

    Ok(())
}

fn lint_command(paths: &[PathBuf]) -> Result<()> {
    for path in paths {
        let diagnostics = lash_core::lint_file(path)
            .with_context(|| format!("Failed to lint {}", path.display()))?;

        if !diagnostics.is_empty() {
            print_diagnostics(&diagnostics);
            anyhow::bail!("Linting failed with {} errors", diagnostics.len());
        }
    }

    Ok(())
}
```

### 5.3 Human and Machine-Readable Output

Support both formats in the CLI:

```rust
fn print_diagnostics(diagnostics: &[Diagnostic], json_output: bool) {
    if json_output {
        println!("{}", serde_json::to_string_pretty(diagnostics).unwrap());
    } else {
        for diag in diagnostics {
            eprintln!(
                "{}:{}:{}: {}: {}",
                diag.file,
                diag.location.line,
                diag.location.column,
                match diag.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    Severity::Info => "info",
                },
                diag.message
            );

            if let Some(suggestion) = &diag.suggestion {
                eprintln!("  help: {}", suggestion);
            }
        }
    }
}
```

**JSON output example:**
```json
[
  {
    "code": "E005_BrokenDependency",
    "severity": "Error",
    "message": "Unknown dependency '@depends-on: core/parser#task:xyz' (target not found)",
    "location": { "line": 42, "column": 5 },
    "file": "core/cli.md",
    "suggestion": "Check that the target file exists and contains a task with this id."
  }
]
```

---

## 6. Testing Strategy

### 6.1 Test Layers

**Unit tests** (inline in each module):
```rust
// lash-core/src/parser.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_task() {
        let markdown = "- [ ] Simple task";
        let result = parse_tasks(markdown).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status, TaskStatus::Open);
    }

    #[test]
    fn test_parse_nested_tasks() {
        let markdown = r#"
- [ ] Parent
  - [ ] Child 1
  - [ ] Child 2
"#;
        let result = parse_tasks(markdown).unwrap();
        assert_eq!(result[0].children.len(), 2);
    }
}
```

**Integration tests** (tests/ directory):
```rust
// tests/integration_test.rs

use lash_core::Linter;
use lash_db::Database;

#[test]
fn test_lint_and_index_workflow() {
    let temp_dir = tempdir::TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.md");

    std::fs::write(&file_path, include_str!("fixtures/valid_task_file.md")).unwrap();

    // Lint the file
    let diagnostics = Linter::lint_file(&file_path).unwrap();
    assert!(diagnostics.is_empty());

    // Index into database
    let db = Database::open_in_memory().unwrap();
    db.index_file(&file_path).unwrap();

    // Query back
    let tasks = db.query_tasks_by_status(TaskStatus::Open).unwrap();
    assert_eq!(tasks.len(), 3);
}
```

**Fixture-based testing** for the linter:
```rust
// tests/linter_fixtures.rs

use std::fs;
use glob::glob;

#[test]
fn test_linter_on_all_fixtures() {
    for entry in glob("tests/fixtures/valid/*.md").unwrap() {
        let path = entry.unwrap();
        let diagnostics = Linter::lint_file(&path).unwrap();
        assert!(
            diagnostics.is_empty(),
            "Valid fixture {} should not have errors: {:?}",
            path.display(),
            diagnostics
        );
    }

    for entry in glob("tests/fixtures/invalid/*.md").unwrap() {
        let path = entry.unwrap();
        let diagnostics = Linter::lint_file(&path).unwrap();
        assert!(
            !diagnostics.is_empty(),
            "Invalid fixture {} should have errors",
            path.display()
        );
    }
}
```

**Fixture directory structure:**
```
tests/
├── fixtures/
│   ├── valid/
│   │   ├── simple_tasks.md
│   │   ├── nested_tasks.md
│   │   ├── with_annotations.md
│   │   └── with_dependencies.md
│   ├── invalid/
│   │   ├── duplicate_ids.md
│   │   ├── too_deep.md
│   │   ├── invalid_status.md
│   │   └── broken_dependency.md
│   └── edge_cases/
│       ├── empty_file.md
│       ├── no_tasks.md
│       └── unicode_titles.md
```

### 6.2 Property-Based Testing

Use `proptest` for complex logic like dependency resolution:

```toml
[dev-dependencies]
proptest = "1.4"
```

```rust
// lash-core/src/dependency.rs

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_no_cycles_in_tree_dependencies(
            depth in 1..5usize,
            breadth in 1..5usize,
        ) {
            let graph = generate_tree_graph(depth, breadth);
            let cycles = graph.detect_cycles();
            assert!(cycles.is_empty(), "Tree graphs should never have cycles");
        }

        #[test]
        fn test_transitive_blocking(
            task_count in 1..20usize,
        ) {
            // Property: If A depends on B and B is incomplete, A must be blocked
            let (graph, tasks) = generate_chain_graph(task_count);

            for i in 1..tasks.len() {
                if tasks[i - 1].status != TaskStatus::Done {
                    assert!(
                        graph.is_blocked(&tasks[i].id),
                        "Task {} should be blocked by incomplete dependency",
                        i
                    );
                }
            }
        }
    }
}
```

### 6.3 Snapshot Testing

For complex outputs like formatted Markdown or dependency graphs:

```toml
[dev-dependencies]
insta = "1.34"
```

```rust
use insta::assert_snapshot;

#[test]
fn test_format_output() {
    let input = include_str!("fixtures/messy_format.md");
    let formatted = format_task_file(input).unwrap();
    assert_snapshot!(formatted);
}

#[test]
fn test_dependency_graph_dot_output() {
    let graph = build_test_graph();
    let dot = graph.to_dot();
    assert_snapshot!(dot);
}
```

### 6.4 Test Coverage Goals

**Priorities:**
1. **100% coverage of error paths** - Every error variant should have a test
2. **High coverage of parsing** - Complex state machine, needs thorough testing
3. **High coverage of dependency resolution** - Critical correctness
4. **Reasonable coverage of DB layer** - Focus on complex queries
5. **Lower coverage of CLI/TUI** - Harder to test, less critical

**Tools:**
```bash
# Install tarpaulin for coverage
cargo install cargo-tarpaulin

# Run coverage
cargo tarpaulin --out Html --output-dir coverage
```

### 6.5 Testing Best Practices

**Do:**
- Test behavior, not implementation
- Use descriptive test names: `test_parse_rejects_duplicate_ids`
- Test edge cases: empty files, max depth, unicode, etc.
- Write tests that fail for the right reason
- Use fixtures for complex inputs

**Don't:**
- Test private implementation details
- Write tests that depend on execution order
- Add special test-only code paths to production code
- Aim for 100% coverage just for the metric
- Mock extensively in unit tests (prefer real implementations)

---

## 7. Implementation Order Recommendations

### 7.1 Vertical Slice Approach

Build a minimal "vertical slice" through all layers to prove the architecture works:

**Phase 0: Project Setup (1-2 days)**
- [ ] Create workspace Cargo.toml
- [ ] Set up crate structure
- [ ] Configure CI (GitHub Actions)
- [ ] Add pre-commit hooks (clippy, rustfmt)
- [ ] Set up test fixtures directory

**Phase 1: Core Parsing (3-5 days)**
- [ ] Implement basic Markdown parsing (pulldown-cmark integration)
- [ ] Parse task checkboxes and status
- [ ] Parse nested tasks (up to max depth)
- [ ] Parse annotations (@id, @labels)
- [ ] Build arena-based TaskTree
- [ ] Write comprehensive parsing tests

**Phase 2: Basic Linting (3-5 days)**
- [ ] Implement syntax validation
- [ ] Check depth limits
- [ ] Validate task status values
- [ ] Check for duplicate IDs within file
- [ ] Generate structured diagnostics
- [ ] Add fixture-based linter tests

**Phase 3: Database Layer (4-6 days)**
- [ ] Design SQLite schema
- [ ] Implement migration system
- [ ] Implement file indexing (insert parsed tasks)
- [ ] Implement basic queries (by status, by label)
- [ ] Add database tests
- [ ] Benchmark indexing performance

**Phase 4: CLI Foundation (2-3 days)**
- [ ] Set up clap CLI structure
- [ ] Implement `lash lint` command
- [ ] Implement `lash index` command
- [ ] Implement `lash list` command
- [ ] Add --json output support
- [ ] Write CLI integration tests

**Phase 5: Dependency Resolution (5-7 days)**
- [ ] Implement dependency graph construction
- [ ] Add parent-child dependencies (within file)
- [ ] Parse explicit dependencies (@depends-on)
- [ ] Implement cycle detection
- [ ] Implement "is blocked" checks
- [ ] Add comprehensive dependency tests

**Phase 6: Advanced Linting (3-4 days)**
- [ ] Add cross-file validation (broken links)
- [ ] Validate dependency references
- [ ] Add auto-fix capabilities (--fix flag)
- [ ] Improve error messages

**Phase 7: Search (3-4 days)**
- [ ] Implement FTS5 integration
- [ ] Implement `lash search` command
- [ ] Add fuzzy matching (nucleo)
- [ ] Benchmark search performance

**Phase 8: TUI (7-10 days)**
- [ ] Set up ratatui + crossterm
- [ ] Implement file tree navigation
- [ ] Implement task list view
- [ ] Add keyboard shortcuts
- [ ] Implement fuzzy search in TUI
- [ ] Add task status toggling

**Phase 9: Agent Integration (4-5 days)**
- [ ] Design prompt templates
- [ ] Implement `lash agent-prompt` command
- [ ] Add token minimization logic
- [ ] Generate JSON schemas
- [ ] Write agent integration guide

**Phase 10: Polish & Documentation (3-5 days)**
- [ ] Write comprehensive README
- [ ] Add command documentation (--help)
- [ ] Create example task files
- [ ] Performance tuning based on benchmarks
- [ ] Release preparation

**Total estimated time: 40-60 days of focused work**

### 7.2 What Can Be Stubbed Initially

To move faster, you can stub these components initially:

**Can stub:**
- Advanced formatting (just parse, don't rewrite)
- TUI (build CLI-first, add TUI later)
- Agent prompt generation (simple templates initially)
- Archive command
- Fix-links command
- Graph visualization (just detect cycles initially)

**Must implement early:**
- Parsing (core foundation)
- Linting (needed for validation)
- Database indexing (needed for queries)
- Basic CLI commands (lint, index, list)

### 7.3 Incremental Complexity

Build features incrementally:

**Version 0.1 (MVP):**
- Parse task files
- Lint for basic errors
- Index into SQLite
- List tasks by filters
- Basic CLI

**Version 0.2:**
- Add cross-file dependencies
- Dependency graph
- TUI implementation
- Search functionality

**Version 0.3:**
- Agent integration
- Auto-formatting
- Advanced linting
- Performance optimization

---

## 8. Additional Recommendations

### 8.1 Development Workflow

**Pre-commit hook:**
```bash
#!/bin/bash
# .git/hooks/pre-commit

set -e

# Run formatter
cargo fmt -- --check

# Run clippy
cargo clippy -- -D warnings

# Run tests
cargo test --all

# Run linter on test fixtures
cargo run -- lint tests/fixtures/valid/
```

**CI/CD (GitHub Actions):**
```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Run tests
        run: cargo test --all
      - name: Run clippy
        run: cargo clippy -- -D warnings
      - name: Check formatting
        run: cargo fmt -- --check

  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Run benchmarks
        run: cargo bench
```

### 8.2 Documentation

Use cargo-doc extensively:

```rust
//! # lash-core
//!
//! Core parsing, linting, and task model for Lash.
//!
//! ## Overview
//!
//! This crate provides the fundamental types and operations for working with
//! Lash task files. It includes:
//!
//! - Markdown parsing using pulldown-cmark
//! - Task validation and linting
//! - Dependency graph construction
//!
//! ## Example
//!
//! ```rust
//! use lash_core::{parse_file, Linter};
//!
//! let content = std::fs::read_to_string("tasks.md")?;
//! let task_file = parse_file(&content)?;
//!
//! let linter = Linter::new();
//! let diagnostics = linter.lint(&task_file)?;
//!
//! if !diagnostics.is_empty() {
//!     eprintln!("Found {} issues", diagnostics.len());
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

/// Parse a Lash task file from Markdown.
///
/// # Arguments
///
/// * `content` - The raw Markdown content
///
/// # Returns
///
/// A `TaskFile` containing the parsed tasks and metadata.
///
/// # Errors
///
/// Returns `ParseError` if the file is malformed or contains invalid syntax.
///
/// # Example
///
/// ```
/// # use lash_core::parse_file;
/// let content = "- [ ] My task";
/// let file = parse_file(content).unwrap();
/// assert_eq!(file.tasks.len(), 1);
/// ```
pub fn parse_file(content: &str) -> Result<TaskFile, ParseError> {
    // ...
}
```

### 8.3 Configuration

Consider a configuration file for user preferences:

```toml
# .lash/config.toml

[linter]
max_depth = 3
enforce_ids = true
enforce_labels = false

[index]
ignore_patterns = ["archive/", "*.bak.md"]

[tui]
theme = "default"
```

Load with:
```toml
[dependencies]
config = "0.14"
```

### 8.4 Cross-Platform Considerations

**Path handling:**
- Use `camino::Utf8PathBuf` for all paths (Markdown files are UTF-8)
- Normalize separators when comparing paths
- Be case-sensitive aware on macOS

**SQLite:**
- Use bundled feature to avoid system dependencies
- Test on Windows, macOS, Linux

**Terminal:**
- crossterm handles platform differences for TUI
- Test on different terminal emulators

### 8.5 Observability

Add structured logging:

```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

```rust
use tracing::{info, warn, debug, instrument};

#[instrument(skip(content))]
pub fn parse_file(path: &Utf8Path, content: &str) -> Result<TaskFile> {
    info!("Parsing file: {}", path);

    let start = std::time::Instant::now();
    let result = do_parse(content)?;

    debug!(
        "Parsed {} tasks in {:?}",
        result.tasks.len(),
        start.elapsed()
    );

    Ok(result)
}
```

Enable with:
```bash
RUST_LOG=lash=debug lash index
```

---

## 9. Summary and Next Steps

### 9.1 Key Decisions

**Crate structure:**
- Consider adding `lash-types` for shared types
- Keep proposed 5-crate structure otherwise

**Dependencies:**
- pulldown-cmark for Markdown parsing
- clap for CLI
- rusqlite for database
- ratatui + crossterm for TUI
- nucleo for fuzzy search

**Data structures:**
- Arena allocation for parsing performance
- Flat representation for database storage
- petgraph for dependency graph

**Error handling:**
- thiserror for libraries
- anyhow for CLI
- Structured diagnostics with stable codes

**Testing:**
- Fixture-based for linter
- Property-based for dependency logic
- Unit + integration + benchmarks

**Implementation order:**
- Start with vertical slice: parse → lint → index → query
- Build CLI before TUI
- Add agent integration after core is stable

### 9.2 Open Questions

These should be decided before/during implementation:

1. **Exact annotation syntax:** `@id: foo` vs YAML frontmatter?
2. **Maximum task depth:** 3 or 4 levels?
3. **Directory naming:** Flat with dots or nested?
4. **Fuzzy search:** FTS5 only, nucleo only, or both?
5. **Configuration:** TOML file, environment variables, or both?

### 9.3 Immediate Next Steps

1. **Set up project structure** (Phase 0)
   - Create workspace
   - Add all crates
   - Set up CI/CD

2. **Implement core parsing** (Phase 1)
   - Start with simplest case: flat task list
   - Add nesting incrementally
   - Build comprehensive test suite

3. **Validate architecture** early
   - Build vertical slice through all layers
   - Benchmark critical paths
   - Adjust if needed before going deep

### 9.4 Risk Mitigation

**Performance risk:**
- Benchmark early and often
- Have fallback strategies (e.g., incremental indexing)

**Complexity risk:**
- Keep scope minimal for v1
- Resist feature creep
- Focus on correctness over cleverness

**Dependency risk:**
- Avoid esoteric dependencies
- Stick to well-maintained crates
- Have migration path if a dependency is abandoned

---

## Appendix A: Workspace Cargo.toml

```toml
[workspace]
members = [
    "lash-types",
    "lash-core",
    "lash-db",
    "lash-agent",
    "lash-tui",
    "lash-cli",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
license = "MIT OR Apache-2.0"
repository = "https://github.com/yourusername/lash"
authors = ["Your Name <your.email@example.com>"]

[workspace.dependencies]
# Shared dependencies across all crates
lash-types = { path = "lash-types" }
lash-core = { path = "lash-core" }
lash-db = { path = "lash-db" }
lash-agent = { path = "lash-agent" }
lash-tui = { path = "lash-tui" }

# External dependencies
anyhow = "1.0"
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
camino = "1.1"
chrono = "0.4"

[profile.release]
lto = true
codegen-units = 1
strip = true
opt-level = 3

[profile.dev]
opt-level = 0
debug = true

[profile.bench]
inherits = "release"
debug = true
```

## Appendix B: Example Crate Structure

```
lash-core/
├── Cargo.toml
├── src/
│   ├── lib.rs           # Public API exports
│   ├── parser/
│   │   ├── mod.rs
│   │   ├── markdown.rs  # Markdown event processing
│   │   ├── task.rs      # Task parsing logic
│   │   └── metadata.rs  # Annotation parsing
│   ├── linter/
│   │   ├── mod.rs
│   │   ├── rules.rs     # Validation rules
│   │   └── diagnostic.rs
│   ├── model/
│   │   ├── mod.rs
│   │   ├── task.rs      # Task types
│   │   ├── file.rs      # File types
│   │   └── graph.rs     # Dependency graph
│   └── error.rs         # Error types
├── tests/
│   ├── parser_tests.rs
│   ├── linter_tests.rs
│   └── fixtures/
│       ├── valid/
│       └── invalid/
└── benches/
    └── parse_bench.rs
```

---

## Appendix C: Recommended Reading

**Rust patterns:**
- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)

**Error handling:**
- [Error Handling in Rust](https://blog.burntsushi.net/rust-error-handling/)
- [thiserror vs anyhow](https://www.lpalmieri.com/posts/error-handling-rust/)

**Testing:**
- [Property-based testing in Rust](https://www.jakobmeier.ch/proptest-intro/)
- [Effective Rust testing](https://matklad.github.io/2021/05/31/how-to-test.html)

**Architecture:**
- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/)
- [Clean Architecture in Rust](https://www.ncameron.org/blog/abstraction-in-rust/)

---

**Document End**

This architecture analysis should provide a solid foundation for implementing Lash. The recommendations prioritize simplicity, performance, and maintainability while staying true to the project's goals. Good luck with the implementation!
