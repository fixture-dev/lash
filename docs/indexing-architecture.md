# Lash Indexing Engine Architecture

**Version:** 1.0
**Date:** 2025-11-19
**Status:** Design

## Overview

This document defines the Rust architecture for the Lash indexing engine, which scans Markdown files, parses them into structured data, and populates the SQLite acceleration layer. The indexing engine is a critical component that bridges the Markdown source of truth with the queryable database layer.

## Design Goals

1. **Performance**: Index 1000 files in <5 seconds
2. **Incremental**: Only reparse files that have changed (hash-based detection)
3. **Parallel**: Parse files concurrently to maximize throughput
4. **Robust**: Aggregate errors rather than fail-fast; leave DB in consistent state
5. **Verifiable**: Detect drift between filesystem and database
6. **Testable**: Clean architecture with mockable components

## Performance Targets

| Project Size | Target Time | Notes |
|--------------|-------------|-------|
| 10 files     | <50ms       | Near-instantaneous |
| 100 files    | <500ms      | Pre-commit hook friendly |
| 1000 files   | <5s         | Large project support |

## Module Structure

The indexing engine lives in the `lash-db` crate and consists of these modules:

```
lash-db/
├── src/
│   ├── lib.rs                 # Public API exports
│   ├── indexing/              # NEW: Indexing engine
│   │   ├── mod.rs             # Public indexing API
│   │   ├── walker.rs          # File system walker
│   │   ├── diff.rs            # Incremental diff computation
│   │   ├── executor.rs        # Index execution engine
│   │   ├── verifier.rs        # Index verification
│   │   ├── progress.rs        # Progress reporting
│   │   └── error.rs           # Indexing-specific errors
│   ├── repository/            # EXISTING: DB repositories
│   │   ├── files.rs
│   │   ├── tasks.rs
│   │   ├── dependencies.rs
│   │   └── labels.rs
│   ├── connection.rs          # EXISTING: DB connection
│   ├── error.rs               # EXISTING: DB errors (extend)
│   └── migrations.rs          # EXISTING: Schema migrations
```

## Core Types and APIs

### 1. Public API (`indexing/mod.rs`)

The CLI will interact with these public functions:

```rust
/// Public indexing API
pub mod indexing;

// Re-exports for convenience
pub use indexing::{
    index_project, verify_index, IndexConfig, IndexResult, IndexStats,
    VerificationReport, IndexError,
};

/// Index a Lash project starting from the given root
///
/// This is the main entry point for the indexing operation. It:
/// 1. Discovers all Markdown files in the project
/// 2. Computes diff between filesystem and database
/// 3. Parses modified/new files in parallel
/// 4. Updates database in a transaction
/// 5. Reports progress and collects errors
///
/// # Arguments
///
/// * `root` - Project root directory (contains lash.index.md or .lash/)
/// * `db_conn` - SQLite database connection
/// * `config` - Indexing configuration
///
/// # Returns
///
/// Returns `IndexResult` containing statistics and any errors encountered.
/// Even if some files fail to parse, the operation continues and reports
/// all errors at the end.
///
/// # Example
///
/// ```no_run
/// use lash_db::{index_project, IndexConfig};
/// use lash_db::open_database;
/// use std::path::Path;
///
/// let root = Path::new("/path/to/project");
/// let conn = open_database(Path::new(".lash/db.sqlite")).unwrap();
/// let config = IndexConfig::default();
///
/// let result = index_project(root, &conn, &config).unwrap();
/// println!("Indexed {} files, {} errors",
///          result.stats.files_indexed,
///          result.errors.len());
/// ```
pub fn index_project(
    root: &Path,
    db_conn: &Connection,
    config: &IndexConfig,
) -> Result<IndexResult, IndexError>;

/// Verify database consistency with filesystem
///
/// Checks that the database accurately reflects the current state of
/// Markdown files on disk. Detects:
/// - Files in DB but missing from filesystem
/// - Files on filesystem but not in DB
/// - Hash mismatches (file modified but not reindexed)
/// - Orphaned task/dependency records
///
/// # Arguments
///
/// * `root` - Project root directory
/// * `db_conn` - SQLite database connection
///
/// # Returns
///
/// Returns a `VerificationReport` describing any inconsistencies found.
///
/// # Example
///
/// ```no_run
/// use lash_db::{verify_index, open_database};
/// use std::path::Path;
///
/// let root = Path::new("/path/to/project");
/// let conn = open_database(Path::new(".lash/db.sqlite")).unwrap();
///
/// let report = verify_index(root, &conn).unwrap();
/// if report.has_issues() {
///     eprintln!("Found {} issues", report.total_issues());
/// }
/// ```
pub fn verify_index(
    root: &Path,
    db_conn: &Connection,
) -> Result<VerificationReport, IndexError>;
```

### 2. Configuration (`indexing/mod.rs`)

```rust
/// Configuration for indexing operations
#[derive(Debug, Clone)]
pub struct IndexConfig {
    /// Maximum number of parallel parsing threads
    /// Default: num_cpus::get()
    pub parallelism: usize,

    /// File extensions to include (default: [".md"])
    pub file_extensions: Vec<String>,

    /// Glob patterns to exclude (e.g., "node_modules/", ".git/")
    pub exclude_patterns: Vec<String>,

    /// Follow symbolic links (default: false for safety)
    pub follow_symlinks: bool,

    /// Progress reporting callback (optional)
    pub progress_callback: Option<ProgressCallback>,

    /// Parser configuration
    pub parser_config: LashConfig,

    /// Continue on parse errors (default: true)
    pub continue_on_error: bool,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            parallelism: num_cpus::get(),
            file_extensions: vec![".md".to_string()],
            exclude_patterns: vec![
                ".git/".to_string(),
                "node_modules/".to_string(),
                "target/".to_string(),
                ".lash/".to_string(), // Don't index our own DB directory
            ],
            follow_symlinks: false,
            progress_callback: None,
            parser_config: LashConfig::default(),
            continue_on_error: true,
        }
    }
}

/// Index operation result
#[derive(Debug)]
pub struct IndexResult {
    /// Statistics about the indexing operation
    pub stats: IndexStats,

    /// Parse errors encountered (non-fatal if continue_on_error=true)
    pub errors: Vec<ParseError>,

    /// Warnings (e.g., skipped files due to permissions)
    pub warnings: Vec<String>,
}

/// Statistics from an indexing operation
#[derive(Debug, Default)]
pub struct IndexStats {
    /// Total files discovered
    pub files_discovered: usize,

    /// Files that were new or modified
    pub files_changed: usize,

    /// Files that were unchanged (skipped)
    pub files_unchanged: usize,

    /// Files that were deleted from DB
    pub files_deleted: usize,

    /// Files successfully indexed
    pub files_indexed: usize,

    /// Files that failed to parse
    pub files_failed: usize,

    /// Tasks inserted/updated
    pub tasks_updated: usize,

    /// Tasks deleted
    pub tasks_deleted: usize,

    /// Duration of operation
    pub duration: std::time::Duration,
}

/// A parse error for a specific file
#[derive(Debug)]
pub struct ParseError {
    /// File path that failed
    pub path: PathBuf,

    /// Diagnostics from parser
    pub diagnostics: Vec<Diagnostic>,
}
```

### 3. File Walker (`indexing/walker.rs`)

Discovers Markdown files in the project tree.

```rust
/// File system walker for discovering Markdown files
pub struct FileWalker {
    /// Project root directory
    root: PathBuf,

    /// File extensions to include
    extensions: Vec<String>,

    /// Exclusion patterns
    exclude_patterns: Vec<String>,

    /// Follow symlinks flag
    follow_symlinks: bool,
}

impl FileWalker {
    /// Create a new file walker
    pub fn new(root: PathBuf, config: &IndexConfig) -> Self {
        Self {
            root,
            extensions: config.file_extensions.clone(),
            exclude_patterns: config.exclude_patterns.clone(),
            follow_symlinks: config.follow_symlinks,
        }
    }

    /// Discover all matching files in the project tree
    ///
    /// Returns a list of file metadata for all discovered files.
    /// Errors (permission denied, broken symlinks) are logged but don't
    /// stop the walk.
    pub fn discover(&self) -> Result<Vec<FileMetadata>, IndexError> {
        // Implementation will use the `ignore` crate for:
        // - Respect .gitignore files automatically
        // - Efficient directory traversal
        // - Built-in pattern matching
        // - Parallel directory scanning
    }
}

/// Metadata for a discovered file
#[derive(Debug, Clone)]
pub struct FileMetadata {
    /// Absolute path to file
    pub absolute_path: PathBuf,

    /// Path relative to project root
    pub relative_path: PathBuf,

    /// File size in bytes
    pub size: u64,

    /// Modification time
    pub mtime: SystemTime,

    /// Content hash (computed lazily)
    pub hash: Option<String>,
}

impl FileMetadata {
    /// Compute and cache content hash
    pub fn compute_hash(&mut self) -> Result<&str, IndexError> {
        // Use blake3 for fast hashing
        // Cache result in self.hash
    }
}
```

### 4. Incremental Diff (`indexing/diff.rs`)

Compares filesystem state with database to determine what needs updating.

```rust
/// Computes the difference between filesystem and database
pub struct IndexDiff {
    /// Files that are new (not in DB)
    pub new_files: Vec<FileMetadata>,

    /// Files that have been modified (hash changed)
    pub modified_files: Vec<FileMetadata>,

    /// Files that were deleted (in DB but not on filesystem)
    pub deleted_files: Vec<PathBuf>,

    /// Files that are unchanged (skip parsing)
    pub unchanged_files: Vec<PathBuf>,
}

impl IndexDiff {
    /// Compute diff between filesystem and database
    ///
    /// # Arguments
    ///
    /// * `discovered_files` - Files found on filesystem
    /// * `db_conn` - Database connection to query existing records
    ///
    /// # Returns
    ///
    /// Returns `IndexDiff` describing all changes needed.
    pub fn compute(
        discovered_files: Vec<FileMetadata>,
        db_conn: &Connection,
    ) -> Result<Self, IndexError> {
        // 1. Query all file records from DB
        let file_repo = FileRepository::new(db_conn);
        let db_files = file_repo.list_all()?;

        // 2. Build hash maps for efficient lookup
        let mut fs_map: HashMap<PathBuf, FileMetadata> =
            discovered_files.into_iter()
                .map(|f| (f.relative_path.clone(), f))
                .collect();

        let db_map: HashMap<PathBuf, FileRecord> =
            db_files.into_iter()
                .map(|f| (f.path.clone(), f))
                .collect();

        // 3. Classify files
        let mut new_files = Vec::new();
        let mut modified_files = Vec::new();
        let mut unchanged_files = Vec::new();

        for (path, mut fs_file) in fs_map.drain() {
            if let Some(db_file) = db_map.get(&path) {
                // File exists in both - check if modified
                let fs_hash = fs_file.compute_hash()?;

                if fs_hash == db_file.hash {
                    // Unchanged
                    unchanged_files.push(path);
                } else {
                    // Modified
                    modified_files.push(fs_file);
                }
            } else {
                // New file
                new_files.push(fs_file);
            }
        }

        // 4. Find deleted files (in DB but not on filesystem)
        let deleted_files = db_map
            .keys()
            .filter(|path| !fs_map.contains_key(*path))
            .cloned()
            .collect();

        Ok(Self {
            new_files,
            modified_files,
            deleted_files,
            unchanged_files,
        })
    }

    /// Total number of files that need processing
    pub fn total_work(&self) -> usize {
        self.new_files.len() + self.modified_files.len() + self.deleted_files.len()
    }

    /// Check if there's any work to do
    pub fn has_changes(&self) -> bool {
        self.total_work() > 0
    }
}
```

### 5. Index Executor (`indexing/executor.rs`)

Orchestrates the full indexing process.

```rust
/// Main indexing execution engine
pub struct IndexExecutor<'a> {
    /// Project root
    root: &'a Path,

    /// Database connection
    db_conn: &'a Connection,

    /// Indexing configuration
    config: &'a IndexConfig,

    /// Progress reporter
    progress: ProgressReporter,
}

impl<'a> IndexExecutor<'a> {
    /// Create a new index executor
    pub fn new(
        root: &'a Path,
        db_conn: &'a Connection,
        config: &'a IndexConfig,
    ) -> Self {
        let progress = ProgressReporter::new(config.progress_callback.clone());
        Self {
            root,
            db_conn,
            config,
            progress,
        }
    }

    /// Execute the full indexing operation
    pub fn execute(&mut self) -> Result<IndexResult, IndexError> {
        let start = std::time::Instant::now();
        let mut stats = IndexStats::default();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Phase 1: Discover files
        self.progress.begin_phase("Discovering files");
        let walker = FileWalker::new(self.root.to_path_buf(), self.config);
        let discovered = walker.discover()?;
        stats.files_discovered = discovered.len();
        self.progress.end_phase();

        // Phase 2: Compute diff
        self.progress.begin_phase("Computing changes");
        let diff = IndexDiff::compute(discovered, self.db_conn)?;
        stats.files_changed = diff.new_files.len() + diff.modified_files.len();
        stats.files_unchanged = diff.unchanged_files.len();
        stats.files_deleted = diff.deleted_files.len();
        self.progress.end_phase();

        // Phase 3: Parse files in parallel
        self.progress.begin_phase("Parsing files");
        let parse_results = self.parse_files_parallel(&diff)?;
        stats.files_indexed = parse_results.successes.len();
        stats.files_failed = parse_results.failures.len();
        errors.extend(parse_results.failures);
        self.progress.end_phase();

        // Phase 4: Update database in transaction
        self.progress.begin_phase("Updating database");
        let db_stats = self.update_database(&diff, &parse_results)?;
        stats.tasks_updated = db_stats.tasks_updated;
        stats.tasks_deleted = db_stats.tasks_deleted;
        self.progress.end_phase();

        stats.duration = start.elapsed();

        Ok(IndexResult {
            stats,
            errors,
            warnings,
        })
    }

    /// Parse files in parallel using rayon
    fn parse_files_parallel(
        &mut self,
        diff: &IndexDiff,
    ) -> Result<ParseResults, IndexError> {
        use rayon::prelude::*;

        // Combine new and modified files
        let files_to_parse: Vec<_> = diff
            .new_files
            .iter()
            .chain(diff.modified_files.iter())
            .collect();

        let total = files_to_parse.len();
        self.progress.set_total(total);

        // Parse in parallel
        let results: Vec<_> = files_to_parse
            .par_iter()
            .map(|file_meta| {
                self.progress.increment();
                self.parse_file(file_meta)
            })
            .collect();

        // Separate successes from failures
        let mut successes = Vec::new();
        let mut failures = Vec::new();

        for result in results {
            match result {
                Ok(parsed) => successes.push(parsed),
                Err(err) => failures.push(err),
            }
        }

        Ok(ParseResults {
            successes,
            failures,
        })
    }

    /// Parse a single file
    fn parse_file(
        &self,
        file_meta: &FileMetadata,
    ) -> Result<ParsedTaskFile, ParseError> {
        match lash_core::parser::parse_file(
            &file_meta.absolute_path,
            &self.config.parser_config,
        ) {
            Ok(task_file) => Ok(ParsedTaskFile {
                metadata: file_meta.clone(),
                task_file,
            }),
            Err(err) => Err(ParseError {
                path: file_meta.relative_path.clone(),
                diagnostics: vec![err.to_diagnostic()],
            }),
        }
    }

    /// Update database with parsed results
    fn update_database(
        &mut self,
        diff: &IndexDiff,
        parse_results: &ParseResults,
    ) -> Result<DbUpdateStats, IndexError> {
        // Use a single transaction for all updates
        let tx = self.db_conn.transaction()?;

        let mut tasks_updated = 0;
        let mut tasks_deleted = 0;

        // 1. Delete removed files (cascades to tasks/deps)
        for path in &diff.deleted_files {
            FileRepository::new(&tx).delete(path)?;
        }

        // 2. Insert/update files and tasks
        for parsed in &parse_results.successes {
            let file_repo = FileRepository::new(&tx);
            let task_repo = TaskRepository::new(&tx);
            let label_repo = LabelRepository::new(&tx);
            let dep_repo = DependencyRepository::new(&tx);

            // Determine if insert or update
            let existing = file_repo.get_by_path(&parsed.metadata.relative_path)?;

            let file_db_id = if let Some(existing_file) = existing {
                // Update existing file
                file_repo.update(&parsed.task_file)?;

                // Delete old tasks (they'll be re-inserted)
                let old_tasks = task_repo.get_by_file(existing_file.id)?;
                tasks_deleted += old_tasks.len();
                for old_task in old_tasks {
                    task_repo.delete(&old_task.full_id)?;
                }

                existing_file.id
            } else {
                // Insert new file
                file_repo.insert(&parsed.task_file)?
            };

            // Insert tasks
            for task in parsed.task_file.tasks.tasks() {
                task_repo.insert(task, file_db_id, &parsed.task_file.id)?;
                tasks_updated += 1;

                // Insert task labels
                for label in &task.metadata.labels {
                    let label_id = label_repo.get_or_create(label)?;
                    label_repo.link_task_label(
                        &format!("{}#{}", parsed.task_file.id, task.id),
                        label_id,
                    )?;
                }
            }

            // Note: Dependency resolution happens separately
            // (see tasks.dependency-resolution.md)
        }

        // Commit transaction
        tx.commit()?;

        Ok(DbUpdateStats {
            tasks_updated,
            tasks_deleted,
        })
    }
}

/// Results of parallel parsing
struct ParseResults {
    successes: Vec<ParsedTaskFile>,
    failures: Vec<ParseError>,
}

/// A successfully parsed task file with metadata
struct ParsedTaskFile {
    metadata: FileMetadata,
    task_file: TaskFile,
}

/// Statistics from database update
struct DbUpdateStats {
    tasks_updated: usize,
    tasks_deleted: usize,
}
```

### 6. Progress Reporting (`indexing/progress.rs`)

Provides progress updates during long-running operations.

```rust
/// Progress callback function type
pub type ProgressCallback = Arc<dyn Fn(ProgressEvent) + Send + Sync>;

/// Progress events emitted during indexing
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    /// Phase started (e.g., "Discovering files")
    PhaseStarted { phase: String },

    /// Phase completed
    PhaseCompleted { phase: String },

    /// Progress update (current/total)
    Progress { current: usize, total: usize },

    /// File being processed
    FileProcessing { path: PathBuf },

    /// File completed
    FileCompleted { path: PathBuf, success: bool },
}

/// Internal progress reporter
pub(crate) struct ProgressReporter {
    callback: Option<ProgressCallback>,
    current_phase: Option<String>,
    current: AtomicUsize,
    total: AtomicUsize,
}

impl ProgressReporter {
    pub fn new(callback: Option<ProgressCallback>) -> Self {
        Self {
            callback,
            current_phase: None,
            current: AtomicUsize::new(0),
            total: AtomicUsize::new(0),
        }
    }

    pub fn begin_phase(&mut self, phase: &str) {
        self.current_phase = Some(phase.to_string());
        if let Some(cb) = &self.callback {
            cb(ProgressEvent::PhaseStarted {
                phase: phase.to_string(),
            });
        }
    }

    pub fn end_phase(&mut self) {
        if let Some(phase) = self.current_phase.take() {
            if let Some(cb) = &self.callback {
                cb(ProgressEvent::PhaseCompleted { phase });
            }
        }
    }

    pub fn set_total(&self, total: usize) {
        self.total.store(total, Ordering::SeqCst);
        self.current.store(0, Ordering::SeqCst);
    }

    pub fn increment(&self) {
        let current = self.current.fetch_add(1, Ordering::SeqCst) + 1;
        let total = self.total.load(Ordering::SeqCst);

        if let Some(cb) = &self.callback {
            cb(ProgressEvent::Progress { current, total });
        }
    }
}
```

### 7. Index Verification (`indexing/verifier.rs`)

Detects drift between database and filesystem.

```rust
/// Index verifier for detecting inconsistencies
pub struct IndexVerifier<'a> {
    root: &'a Path,
    db_conn: &'a Connection,
}

impl<'a> IndexVerifier<'a> {
    pub fn new(root: &'a Path, db_conn: &'a Connection) -> Self {
        Self { root, db_conn }
    }

    /// Verify index consistency
    pub fn verify(&self) -> Result<VerificationReport, IndexError> {
        let mut report = VerificationReport::default();

        // 1. Check for orphaned files (in DB but not on filesystem)
        report.orphaned_files = self.find_orphaned_files()?;

        // 2. Check for missing files (on filesystem but not in DB)
        report.missing_files = self.find_missing_files()?;

        // 3. Check for hash mismatches
        report.hash_mismatches = self.find_hash_mismatches()?;

        // 4. Check for orphaned tasks (file deleted but tasks remain)
        report.orphaned_tasks = self.find_orphaned_tasks()?;

        // 5. Check for orphaned dependencies
        report.orphaned_dependencies = self.find_orphaned_dependencies()?;

        Ok(report)
    }

    fn find_orphaned_files(&self) -> Result<Vec<PathBuf>, IndexError> {
        let file_repo = FileRepository::new(self.db_conn);
        let db_files = file_repo.list_all()?;

        let mut orphaned = Vec::new();
        for db_file in db_files {
            let abs_path = self.root.join(&db_file.path);
            if !abs_path.exists() {
                orphaned.push(db_file.path);
            }
        }

        Ok(orphaned)
    }

    fn find_missing_files(&self) -> Result<Vec<PathBuf>, IndexError> {
        // Walk filesystem and check DB
        let walker = FileWalker::new(
            self.root.to_path_buf(),
            &IndexConfig::default(),
        );
        let discovered = walker.discover()?;

        let file_repo = FileRepository::new(self.db_conn);
        let mut missing = Vec::new();

        for file_meta in discovered {
            if file_repo.get_by_path(&file_meta.relative_path)?.is_none() {
                missing.push(file_meta.relative_path);
            }
        }

        Ok(missing)
    }

    fn find_hash_mismatches(&self) -> Result<Vec<HashMismatch>, IndexError> {
        let file_repo = FileRepository::new(self.db_conn);
        let db_files = file_repo.list_all()?;

        let mut mismatches = Vec::new();
        for db_file in db_files {
            let abs_path = self.root.join(&db_file.path);
            if !abs_path.exists() {
                continue; // Already reported as orphaned
            }

            let content = std::fs::read_to_string(&abs_path)?;
            let fs_hash = lash_types::file::compute_hash(&content);

            if fs_hash != db_file.hash {
                mismatches.push(HashMismatch {
                    path: db_file.path,
                    db_hash: db_file.hash,
                    fs_hash,
                });
            }
        }

        Ok(mismatches)
    }

    fn find_orphaned_tasks(&self) -> Result<Vec<String>, IndexError> {
        // Query tasks whose file_id doesn't match any file
        let mut stmt = self.db_conn.prepare(
            "SELECT t.full_id FROM tasks t
             LEFT JOIN files f ON t.file_id = f.id
             WHERE f.id IS NULL"
        )?;

        let orphaned = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(orphaned)
    }

    fn find_orphaned_dependencies(&self) -> Result<Vec<String>, IndexError> {
        // Query dependencies where source or target task doesn't exist
        let mut stmt = self.db_conn.prepare(
            "SELECT d.source_task_id FROM dependencies d
             LEFT JOIN tasks t ON d.source_task_id = t.full_id
             WHERE t.full_id IS NULL
             UNION
             SELECT d.target_task_id FROM dependencies d
             LEFT JOIN tasks t ON d.target_task_id = t.full_id
             WHERE t.full_id IS NULL"
        )?;

        let orphaned = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(orphaned)
    }
}

/// Verification report describing any issues found
#[derive(Debug, Default)]
pub struct VerificationReport {
    /// Files in DB but not on filesystem
    pub orphaned_files: Vec<PathBuf>,

    /// Files on filesystem but not in DB
    pub missing_files: Vec<PathBuf>,

    /// Files with hash mismatches
    pub hash_mismatches: Vec<HashMismatch>,

    /// Tasks whose parent file no longer exists
    pub orphaned_tasks: Vec<String>,

    /// Dependencies referencing non-existent tasks
    pub orphaned_dependencies: Vec<String>,
}

impl VerificationReport {
    /// Check if any issues were found
    pub fn has_issues(&self) -> bool {
        !self.orphaned_files.is_empty()
            || !self.missing_files.is_empty()
            || !self.hash_mismatches.is_empty()
            || !self.orphaned_tasks.is_empty()
            || !self.orphaned_dependencies.is_empty()
    }

    /// Total number of issues
    pub fn total_issues(&self) -> usize {
        self.orphaned_files.len()
            + self.missing_files.len()
            + self.hash_mismatches.len()
            + self.orphaned_tasks.len()
            + self.orphaned_dependencies.len()
    }
}

/// Hash mismatch between DB and filesystem
#[derive(Debug)]
pub struct HashMismatch {
    pub path: PathBuf,
    pub db_hash: String,
    pub fs_hash: String,
}
```

### 8. Error Types (`indexing/error.rs`)

```rust
use thiserror::Error;

/// Indexing-specific errors
#[derive(Error, Debug)]
pub enum IndexError {
    /// I/O error during file operations
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Database error
    #[error("Database error: {0}")]
    Database(#[from] crate::error::DbError),

    /// SQLite error
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// Parser error (from lash-core)
    #[error("Parse error: {0}")]
    Parse(#[from] lash_types::LashError),

    /// Project root not found
    #[error("Project root not found: {0}")]
    RootNotFound(PathBuf),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Indexing operation failed
    #[error("Indexing failed: {0}")]
    IndexingFailed(String),
}
```

## Integration with Existing Code

### FileRepository Extension

The existing `FileRepository` already provides the needed CRUD operations:
- `insert(&TaskFile)` - Insert new file
- `update(&TaskFile)` - Update existing file
- `delete(&Path)` - Delete file (cascades to tasks)
- `get_by_path(&Path)` - Query file by path
- `list_all()` - List all files

No changes needed to repository layer.

### TaskRepository Extension

The existing `TaskRepository` provides:
- `insert(&Task, file_db_id, file_id)` - Insert task
- `update(&Task, file_id)` - Update task
- `delete(&full_id)` - Delete task
- `get_by_file(file_db_id)` - Get all tasks in file
- `insert_batch(&[(Task, i64, String)])` - Batch insert

We can use `insert_batch` for better performance when indexing large files.

## Parallelism Strategy

### Using Rayon for Parsing

**Why Rayon?**
- Data parallelism model (perfect for embarrassingly parallel parsing)
- Work stealing for load balancing
- Zero-cost abstraction over thread pools
- Simpler than tokio for CPU-bound work

**Implementation:**

```rust
use rayon::prelude::*;

let results: Vec<_> = files_to_parse
    .par_iter()  // Parallel iterator
    .map(|file| parse_file(file))
    .collect();
```

**Threading Model:**
- Parse files in parallel using Rayon thread pool
- Database writes remain single-threaded (SQLite limitation)
- Use a single transaction for all DB updates

**Why Not Tokio?**
- Parsing is CPU-bound, not I/O-bound
- No benefit from async for file parsing
- Rayon is simpler and more efficient for this use case

## Hash Computation Strategy

### Blake3 Hashing

**Why Blake3?**
- Extremely fast (faster than SHA-256)
- Cryptographically secure (not needed but nice)
- Built-in parallelism for large files (>1MB)
- Small dependency footprint

**Implementation:**

```rust
use blake3;

fn compute_hash(content: &str) -> String {
    blake3::hash(content.as_bytes()).to_hex().to_string()
}
```

**Optimization:**
- For small files (<100KB): Single-threaded blake3
- For large files (>1MB): Use blake3's built-in parallelism
- Cache hashes in database to avoid recomputation

**Fast Path:**
- If `mtime` unchanged and hash exists in DB, skip re-hashing
- Only compute hash when needed for diff comparison

## Transaction Strategy

### Single Transaction for All Updates

**Rationale:**
- Ensures database consistency (all-or-nothing)
- Better performance than per-file transactions
- SQLite handles rollback automatically on error

**Trade-offs:**
- Long-running transaction holds write lock
- But: Indexing should be fast enough (<5s) that this is acceptable
- Alternative: Batch transactions (commit every N files) adds complexity

**Implementation:**

```rust
let tx = conn.transaction()?;

// All inserts/updates/deletes here

tx.commit()?;  // Atomic commit
```

## Testing Strategy

### Unit Tests

Each module should have comprehensive unit tests:

```rust
// walker_tests.rs
#[test]
fn test_discover_excludes_patterns() {
    // Create temp directory with .git/ and node_modules/
    // Verify they're excluded
}

// diff_tests.rs
#[test]
fn test_diff_identifies_new_files() {
    // Create temp DB with some files
    // Discover new files on filesystem
    // Verify diff correctly identifies new files
}

// executor_tests.rs
#[test]
fn test_index_empty_project() {
    // Index empty directory
    // Verify stats are correct
}

#[test]
fn test_index_with_errors() {
    // Index project with parse errors
    // Verify errors are collected, operation continues
}
```

### Integration Tests

Test the full indexing pipeline:

```rust
#[test]
fn test_full_index_cycle() {
    // 1. Create temp project with Markdown files
    // 2. Index from scratch
    // 3. Verify DB contains correct data
    // 4. Modify a file
    // 5. Incremental index
    // 6. Verify only modified file was reparsed
}

#[test]
fn test_verification_detects_drift() {
    // 1. Index project
    // 2. Manually delete file from filesystem
    // 3. Run verification
    // 4. Verify orphaned file detected
}
```

### Performance Tests

Benchmark critical paths:

```rust
#[bench]
fn bench_hash_computation(b: &mut Bencher) {
    let content = std::fs::read_to_string("large_file.md").unwrap();
    b.iter(|| compute_hash(&content));
}

#[bench]
fn bench_index_1000_files(b: &mut Bencher) {
    // Generate 1000 test files
    // Benchmark full index operation
    // Target: <5s
}
```

## CLI Integration

The CLI will call the indexing API like this:

```rust
// In lash-cli/src/commands/index.rs

use lash_db::{index_project, IndexConfig, ProgressEvent};
use std::sync::Arc;

pub fn run_index_command(args: &IndexArgs) -> Result<()> {
    let root = find_project_root()?;
    let db_path = root.join(".lash/db.sqlite");
    let conn = lash_db::open_database(&db_path)?;

    // Setup progress reporting
    let progress_callback = Arc::new(|event: ProgressEvent| {
        match event {
            ProgressEvent::PhaseStarted { phase } => {
                println!("{}", phase);
            }
            ProgressEvent::Progress { current, total } => {
                print!("\r{}/{} files", current, total);
                std::io::stdout().flush().unwrap();
            }
            _ => {}
        }
    });

    let config = IndexConfig {
        progress_callback: Some(progress_callback),
        ..Default::default()
    };

    let result = index_project(&root, &conn, &config)?;

    println!("\nIndexed {} files in {:?}",
             result.stats.files_indexed,
             result.stats.duration);

    if !result.errors.is_empty() {
        eprintln!("{} files failed to parse:", result.errors.len());
        for err in &result.errors {
            eprintln!("  {}: {}", err.path.display(),
                     err.diagnostics[0].message);
        }
    }

    Ok(())
}
```

## Dependency Tree

```
lash-cli
  └─ lash-db (indexing)
       ├─ lash-core (parser)
       │    └─ lash-types
       ├─ rusqlite
       ├─ rayon
       ├─ blake3
       ├─ ignore (for file walking)
       └─ num_cpus
```

### New Dependencies Needed

Add to `lash-db/Cargo.toml`:

```toml
[dependencies]
# Existing dependencies
lash-types = { workspace = true }
lash-core = { workspace = true }
rusqlite = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }

# NEW: Indexing dependencies
rayon = "1.8"           # Parallel file parsing
blake3 = "1.5"          # Fast content hashing
ignore = "0.4"          # File walking with gitignore support
num_cpus = "1.16"       # Auto-detect parallelism level
```

## Future Optimizations

These can be added later if needed:

1. **Incremental Dependency Re-resolution**: When a file changes, only update dependency edges involving tasks in that file (see Task 5 in tasks.indexing.md)

2. **Memory-mapped File Hashing**: For very large files (>10MB), use memory-mapped I/O for faster hashing

3. **Persistent Hash Cache**: Store hashes in a separate cache file to avoid DB queries during diff computation

4. **Batch Transactions**: Commit every N files instead of one transaction for entire index (trade-off: complexity vs. lock duration)

5. **Watch Mode**: Add file watching for real-time indexing (would require tokio + notify crate)

## Open Design Questions

1. **Parallelism Level**: Should we auto-detect (`num_cpus::get()`) or make it configurable?
   - **Recommendation**: Auto-detect with override option

2. **Error Handling**: Continue on errors or fail-fast?
   - **Recommendation**: Continue by default, add `--strict` flag for fail-fast

3. **Progress Reporting**: Use callbacks or channels?
   - **Recommendation**: Callbacks (simpler, more flexible)

4. **Transaction Granularity**: One transaction or batched?
   - **Recommendation**: Start with one transaction, add batching if lock contention is an issue

## Summary

This architecture provides:

- **Clean separation of concerns**: Walker, differ, executor, verifier are independent modules
- **Testable components**: Each module can be unit tested in isolation
- **Performance**: Parallel parsing with Rayon, fast hashing with blake3, single transaction
- **Robustness**: Error aggregation, transaction safety, verification tools
- **Extensibility**: Easy to add watch mode, incremental dep resolution, etc.

The design prioritizes simplicity and maintainability while meeting all performance targets. The public API is minimal and easy to use from the CLI, while the internal architecture is modular and testable.
