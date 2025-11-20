# Lash Indexing Architecture - Quick Reference

This document provides a quick reference for the indexing engine architecture. See `indexing-architecture.md` for the full design.

## Module Overview

```
lash-db/src/indexing/
├── mod.rs          - Public API (index_project, verify_index)
├── walker.rs       - File discovery (FileWalker)
├── diff.rs         - Change detection (IndexDiff)
├── executor.rs     - Main orchestration (IndexExecutor)
├── verifier.rs     - Consistency checking (IndexVerifier)
├── progress.rs     - Progress reporting (ProgressReporter)
└── error.rs        - Error types (IndexError)
```

## Public API

### Main Entry Points

```rust
// Index a project
pub fn index_project(
    root: &Path,
    db_conn: &Connection,
    config: &IndexConfig,
) -> Result<IndexResult, IndexError>;

// Verify index consistency
pub fn verify_index(
    root: &Path,
    db_conn: &Connection,
) -> Result<VerificationReport, IndexError>;
```

### Configuration

```rust
pub struct IndexConfig {
    pub parallelism: usize,              // Default: num_cpus::get()
    pub file_extensions: Vec<String>,    // Default: [".md"]
    pub exclude_patterns: Vec<String>,   // Default: [".git/", "node_modules/"]
    pub follow_symlinks: bool,           // Default: false
    pub progress_callback: Option<ProgressCallback>,
    pub parser_config: LashConfig,
    pub continue_on_error: bool,         // Default: true
}
```

### Results

```rust
pub struct IndexResult {
    pub stats: IndexStats,
    pub errors: Vec<ParseError>,
    pub warnings: Vec<String>,
}

pub struct IndexStats {
    pub files_discovered: usize,
    pub files_changed: usize,
    pub files_unchanged: usize,
    pub files_deleted: usize,
    pub files_indexed: usize,
    pub files_failed: usize,
    pub tasks_updated: usize,
    pub tasks_deleted: usize,
    pub duration: Duration,
}
```

## Architecture Flow

```
┌─────────────────────────────────────────────────────────────┐
│                      index_project()                        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
         ┌────────────────────────────────────────┐
         │  Phase 1: Discover Files               │
         │  (FileWalker)                          │
         │  - Walk directory tree                 │
         │  - Apply exclusion patterns            │
         │  - Collect file metadata               │
         └────────────────────────────────────────┘
                              │
                              ▼
         ┌────────────────────────────────────────┐
         │  Phase 2: Compute Diff                 │
         │  (IndexDiff)                           │
         │  - Query existing DB records           │
         │  - Compare hashes                      │
         │  - Classify: new/modified/deleted      │
         └────────────────────────────────────────┘
                              │
                              ▼
         ┌────────────────────────────────────────┐
         │  Phase 3: Parse Files (Parallel)       │
         │  (IndexExecutor + Rayon)               │
         │  - Parse new/modified files            │
         │  - Collect successes and errors        │
         └────────────────────────────────────────┘
                              │
                              ▼
         ┌────────────────────────────────────────┐
         │  Phase 4: Update Database              │
         │  (Transaction)                         │
         │  - Delete removed files                │
         │  - Insert/update files & tasks         │
         │  - Commit atomically                   │
         └────────────────────────────────────────┘
                              │
                              ▼
         ┌────────────────────────────────────────┐
         │  Return IndexResult                    │
         │  - Statistics                          │
         │  - Errors (if any)                     │
         │  - Warnings                            │
         └────────────────────────────────────────┘
```

## Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Parallelism** | Rayon | Data parallelism, simpler than async for CPU-bound work |
| **Hashing** | Blake3 | Fast, cryptographically secure, built-in parallelism |
| **File Walking** | `ignore` crate | Respects .gitignore, efficient, battle-tested |
| **Transactions** | Single transaction | Atomic updates, simpler than batching |
| **Error Handling** | Aggregate errors | Better UX than fail-fast |
| **Progress** | Callbacks | Simple, flexible, sync-friendly |

## Performance Targets

| Project Size | Target | Strategy |
|--------------|--------|----------|
| 10 files     | <50ms  | Fast path: unchanged detection |
| 100 files    | <500ms | Parallel parsing (4-8 cores) |
| 1000 files   | <5s    | Blake3 hashing + rayon + batch DB ops |

## Component Responsibilities

### FileWalker
- Discover Markdown files recursively
- Apply exclusion patterns (.git/, node_modules/)
- Respect .gitignore files
- Collect file metadata (path, size, mtime)

### IndexDiff
- Query existing DB records
- Compute file hashes (blake3)
- Classify files: new/modified/deleted/unchanged
- Optimize: skip hashing if mtime unchanged

### IndexExecutor
- Orchestrate the full pipeline
- Parse files in parallel (rayon)
- Aggregate parse errors
- Update DB in single transaction
- Report progress via callbacks

### IndexVerifier
- Check DB vs filesystem consistency
- Detect orphaned files (in DB, not on disk)
- Detect missing files (on disk, not in DB)
- Detect hash mismatches
- Detect orphaned tasks/dependencies

### ProgressReporter
- Emit phase events (started/completed)
- Emit progress events (current/total)
- Thread-safe (atomic counters)
- Optional callback support

## Error Handling

### Strategy
- **Continue on parse errors** (default)
- **Aggregate all errors** for reporting
- **Transaction rollback** on DB failures
- **Clear error messages** with file paths

### Error Types
```rust
pub enum IndexError {
    Io(std::io::Error),
    Database(DbError),
    Sqlite(rusqlite::Error),
    Parse(LashError),
    RootNotFound(PathBuf),
    InvalidConfig(String),
    IndexingFailed(String),
}
```

## Dependencies

New dependencies for `lash-db`:

```toml
rayon = "1.8"           # Parallel parsing
blake3 = "1.5"          # Fast hashing
ignore = "0.4"          # File walking with gitignore
num_cpus = "1.16"       # Auto-detect cores
```

## Testing Strategy

### Unit Tests
- Walker: exclusion patterns, symlinks
- Diff: new/modified/deleted detection
- Verifier: drift detection

### Integration Tests
- Full index cycle (fresh + incremental)
- Error handling (continue vs fail-fast)
- Verification workflow

### Performance Tests
- Benchmark hash computation
- Benchmark 1000 file index
- Ensure <5s target met

## CLI Integration Example

```rust
use lash_db::{index_project, IndexConfig};

let root = find_project_root()?;
let conn = lash_db::open_database(&root.join(".lash/db.sqlite"))?;

let progress_callback = Arc::new(|event| {
    // Show progress to user
});

let config = IndexConfig {
    progress_callback: Some(progress_callback),
    ..Default::default()
};

let result = index_project(&root, &conn, &config)?;

println!("Indexed {} files in {:?}",
         result.stats.files_indexed,
         result.stats.duration);
```

## Future Enhancements

1. **Incremental Dependency Re-resolution** - Only update affected edges
2. **Memory-mapped Hashing** - For very large files (>10MB)
3. **Batch Transactions** - Commit every N files (if lock contention)
4. **Watch Mode** - Real-time indexing with file watchers
5. **Compression** - Optionally compress DB (not needed for v1)

## Related Documents

- `docs/indexing-architecture.md` - Full design specification
- `tasks/tasks.indexing.md` - Implementation task breakdown
- `docs/design-doc.md` - Overall Lash design
- `tasks/tasks.sqlite-schema.md` - Database schema
