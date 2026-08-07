# Indexing Engine Tasks

**Module:** `lash-db` (indexing layer)
**Dependencies:** tasks.sqlite-schema.md, tasks.markdown-parser.md, tasks.core-data-model.md
**Effort:** 8.5-12.5 days (includes Task 0: 0.5 days)
**Priority:** CRITICAL

## Overview

The indexing engine walks the project tree, parses Markdown files, and populates the SQLite database. It must handle incremental updates efficiently, detect file changes, and maintain consistency between Markdown source and database state.

## Core Requirements

From design-doc.md:
- Rebuild SQLite DB from Markdown files (section 7.2)
- Track file hashes and mtimes for incremental indexing (section 13.2)
- Verify DB consistency with Markdown (section 7.2)
- Walk directory tree starting from root index (section 3.1)

---

## Task 0: Project Root Discovery

**Priority:** CRITICAL
**Effort:** 0.5 days
**Depends on:** None

### Description

Implement project root discovery logic to locate the Lash project root directory. This is a foundational task required by all other indexing components.

### Subtasks

- [x] Implement `find_project_root()` function
  - [x] Search upward from current directory for project markers
  - [x] Look for `.lash/` directory (explicit marker, highest precedence)
  - [x] Look for `lash.index.md` file (conventional marker)
  - [x] Return project root path if found
  - [x] Return error if no root found
- [x] Define clear precedence rules
  - [x] `.lash/` directory takes precedence over `lash.index.md`
  - [x] Stop at filesystem root (don't search forever)
  - [x] Handle edge cases (permission denied, symlinks)
- [x] Add configuration options
  - [x] Allow explicit root path override (for testing)
  - [x] Configurable search depth limit (default: unlimited)
- [x] Document project root conventions
  - [x] Where to place `.lash/` directory
  - [x] Naming conventions for index file

### Success Criteria

- Correctly finds project root in typical project layouts
- Handles nested projects (stops at nearest root)
- Clear error message when no root found
- Performance: <10ms for typical case

### Tests

- [x] Unit: Test with `.lash/` directory present
- [x] Unit: Test with `lash.index.md` file present
- [x] Unit: Test with both markers (verify precedence)
- [x] Unit: Test with no markers (verify error)
- [x] Unit: Test nested directory search
- [x] Unit: Test at filesystem root (verify termination)

---

## Task 1: File System Walker

**Priority:** CRITICAL
**Effort:** 1-2 days
**Depends on:** Task 0

### Description

Implement recursive directory traversal to discover all Markdown files in a Lash project tree.

### Subtasks

- [x] Implement `FileWalker` struct with configuration
  - [x] Accept project root path (from Task 0)
  - [x] Configurable file extensions (`.md`)
  - [x] Exclude patterns (`.git/`, `node_modules/`, `target/`, `.lash/db.sqlite`)
  - [x] Respect `.gitignore` by default (add `--no-ignore` override)
  - [x] Follow symlinks option (default: false for safety)
- [x] Implement `discover_files()` function
  - [x] Start from project root (provided by Task 0)
  - [x] Recursively walk directories using `ignore` crate
  - [x] Filter by extension and patterns
  - [x] Return list of file paths with metadata
- [x] Add file metadata collection
  - [x] Absolute path
  - [x] Relative path from project root
  - [x] File size
  - [x] Modification time (mtime)
  - [x] Compute content hash (blake3 or similar)
- [x] Handle edge cases
  - [x] Permission denied errors (skip and warn)
  - [x] Broken symlinks (skip and warn)
  - [x] Very large directories (streaming/iterator approach)
  - [x] Unicode filenames

### Success Criteria

- Can discover all `.md` files in test fixture directories
- Correctly excludes ignored patterns
- Returns accurate file metadata
- Handles errors gracefully without panicking
- Performance: <100ms for 1000 files

### Tests

- [x] Unit: Test walker on fixture directories with various structures
- [x] Unit: Test exclusion patterns work correctly
- [x] Unit: Test symlink handling
- [x] Integration: Test on real project structure

---

## Task 2: Incremental Indexing Logic

**Priority:** HIGH
**Effort:** 2-3 days
**Depends on:** Task 1, tasks.sqlite-schema.md#1

### Description

Implement logic to detect which files need re-parsing by comparing filesystem state with database records.

### Subtasks

- [x] Implement `IndexDiff` struct to track changes
  - [x] New files (not in DB)
  - [x] Modified files (hash or mtime changed)
  - [x] Deleted files (in DB but not on filesystem)
  - [x] Unchanged files (skip parsing)
- [x] Implement `compute_index_diff()` function
  - [x] Query DB for existing file records
  - [x] Compare hashes/mtimes with filesystem
  - [x] Build diff structure
  - [x] Handle missing DB (full reindex case)
- [x] Implement hash computation strategy
  - [x] Use blake3 for speed
  - [x] Hash only file content (not metadata)
  - [x] Cache hashes in DB
- [x] Add fast-path optimizations
  - [x] If mtime unchanged and hash exists, skip re-hashing
  - [x] Batch DB queries for efficiency
  - [x] Parallelize hash computation for large projects

### Success Criteria

- [x] Correctly identifies new, modified, and deleted files
- [x] Fast path: unchanged files detected in <10ms each
- [x] Handles edge cases (clock skew, manual DB edits)
- [x] Accurate diff even with concurrent file modifications

### Tests

- [x] Unit: Test diff computation with various scenarios
- [x] Unit: Test hash stability across runs
- [x] Integration: Test incremental indexing after file modifications
- [ ] Performance: Benchmark diff computation for 1000 files

---

## Task 3: Index Execution Engine

**Priority:** CRITICAL
**Effort:** 3-4 days
**Depends on:** Task 2, tasks.markdown-parser.md#1-2

### Description

Coordinate the full indexing process: parse files, populate DB, handle errors, and report progress.

### Subtasks

- [x] Implement `Indexer` struct
  - [x] Hold references to DB connection, parser, file walker
  - [x] Configuration (incremental vs full, parallelism level)
  - [x] Progress tracking
- [x] Implement `index_project()` function
  - [x] Discover files (Task 1)
  - [x] Compute diff (Task 2)
  - [x] Parse modified/new files
  - [x] Begin DB transaction
  - [x] Delete records for removed files
  - [x] Insert/update records for new/modified files
  - [x] Commit transaction
  - [x] Handle rollback on errors
- [x] Add parallel parsing support
  - [x] Parse files in parallel using rayon
  - [x] Collect results and errors
  - [x] Insert into DB in single thread (SQLite limitation)
- [x] Implement progress reporting
  - [x] Track files processed vs total
  - [x] Emit progress events (for CLI/TUI)
  - [x] Support quiet mode (no output)
- [x] Error aggregation
  - [x] Collect all parse errors (don't stop on first)
  - [x] Associate errors with file paths
  - [x] Return structured error report

### Success Criteria

- [x] Can index a project from scratch successfully
- [x] Incremental indexing correctly updates only changed files
- [x] Handles parse errors gracefully (collects all, continues)
- [x] Progress reporting works for long-running operations
- [x] Transaction safety: DB left in consistent state on error

### Tests

- [x] Integration: Index empty project
- [x] Integration: Index project with valid files
- [x] Integration: Index project with parse errors (verify error collection)
- [x] Integration: Incremental indexing after modifications
- [x] Integration: Verify DB consistency after index

---

## Task 4: Index Verification

**Priority:** HIGH
**Effort:** 2-3 days
**Depends on:** Task 3
**Status:** COMPLETED

### Description

Implement the `lash check-index` command to verify database consistency with Markdown files.

### Subtasks

- [x] Implement `IndexVerifier` struct
  - [x] Compare DB records with filesystem
  - [x] Detect drift (DB out of sync)
  - [x] Report discrepancies
- [x] Implement verification checks
  - [x] Files in DB but not on filesystem
  - [x] Files on filesystem but not in DB
  - [x] Hash mismatches (file modified but not reindexed)
  - [x] Orphaned task records (file deleted but tasks remain)
  - [x] Orphaned dependency records
- [x] Implement `verify_index()` function
  - [x] Run all checks
  - [x] Collect discrepancies
  - [x] Return verification report
- [x] Add fix suggestions
  - [x] "Run `lash index` to resync"
  - [x] "Remove stale DB records"
  - [x] Auto-fix option (with confirmation)

### Success Criteria

- [x] Detects all common drift scenarios
- [x] Clear, actionable error messages
- [x] Fast verification (<500ms for 1000 files)
- [x] Optional auto-fix works safely

### Tests

- [x] Unit: Test each verification check independently
- [x] Integration: Verify clean project (no drift)
- [x] Integration: Introduce drift and verify detection
- [x] Integration: Test auto-fix functionality

### Implementation Notes

- Created `verifier.rs` module in lash-db crate
- Implemented `IndexVerifier` struct with configurable verification options
- Defined `VerificationReport` with detailed issue categorization
- Implemented 5 types of issue detection:
  - `StaleFile`: Files in DB but not on filesystem
  - `MissingFile`: Files on filesystem but not in DB
  - `HashMismatch`: File content has changed but not reindexed
  - `OrphanedTasks`: Tasks exist for files that no longer exist
  - `OrphanedDependencies`: Dependencies reference non-existent tasks
- Auto-fix functionality safely removes stale data (but does not re-index)
- All tests passing (14 unit tests + comprehensive doctests)

---

## Task 5: Incremental Dependency Re-resolution

**Priority:** MEDIUM
**Effort:** 2-3 days
**Depends on:** Task 3, tasks.dependency-resolution.md#1-2
**Status:** COMPLETED

### Description

When files change, efficiently update only affected dependency edges without full graph recomputation.

### Subtasks

- [x] Implement `DependencyUpdater` struct
  - [x] Identify tasks affected by file changes
  - [x] Delete stale dependency edges
  - [x] Re-resolve dependencies for affected tasks
- [x] Implement `update_dependencies()` function
  - [x] Query tasks in modified files
  - [x] Delete dependency edges from/to these tasks
  - [x] Re-run dependency resolution (hierarchy dependencies only for now)
  - [x] Insert new edges
- [x] Optimize for minimal graph updates
  - [x] Only update edges for changed tasks
  - [x] Preserve edges for unchanged tasks
  - [x] Batch DB operations
- [x] Handle cascading updates
  - [x] Task parent_id references handled via database FK constraints
  - [x] Transitive closure rebuilt after batch updates
- [x] Integrate into indexer workflow
  - [x] Insert hierarchy dependencies after task insertion
  - [x] Rebuild closure after all files indexed

### Success Criteria

- [x] Incremental dependency updates are faster than full resolution
- [x] Dependency graph remains consistent after updates
- [x] Broken references detected and reported (via verifier module)

### Tests

- [x] Unit: `test_insert_hierarchy_dependencies_no_parents` - flat tasks
- [x] Unit: `test_insert_hierarchy_dependencies_with_parents` - parent-child relationships
- [x] Unit: `test_insert_hierarchy_dependencies_nested` - 3-level hierarchy
- [x] Unit: `test_delete_dependencies_for_files` - selective deletion
- [x] Unit: `test_update_dependencies_for_files` - full update workflow
- [x] Unit: `test_verify_hierarchy_dependencies` - verification helper
- [x] Integration: `test_hierarchy_dependencies_created` - indexing creates dependencies
- [x] Integration: `test_hierarchy_dependencies_updated_on_file_change` - incremental update
- [x] Integration: `test_transitive_closure_built` - closure table populated correctly

### Implementation Notes

- Created `dependency_updater.rs` module with comprehensive functionality
- Hierarchy dependencies implemented; explicit `@depends-on` deferred to future work
- Transitive closure rebuilt after batch file operations for efficiency
- All 8 unit tests + 3 integration tests passing
- Total test count: 111 unit tests + 63 doctests in lash-db crate

---

## Task 6: Index Performance Optimization

**Priority:** MEDIUM
**Effort:** 2-3 days
**Depends on:** Task 3

### Description

Profile and optimize indexing performance for large projects (1000+ files).

### Subtasks

- [x] Add performance instrumentation
  - [x] Measure time per indexing phase
  - [x] Track DB query times
  - [x] Track parse times
  - [x] Memory usage profiling (deferred - basic profiling sufficient)
- [x] Optimize bottlenecks
  - [x] Batch INSERT statements using upsert_batch
  - [x] Parallelize file parsing (already in Task 3)
  - [x] Evaluated hash computation (blake3 already optimal)
  - [x] Evaluated prepared statements (rusqlite handles internally)
- [x] Evaluate caching layer (not needed - documented why)
  - [x] Analyzed cache needs (upsert_batch returns path_to_id mapping)
  - [x] Documented that one-shot indexing doesn't benefit from caching
- [x] Benchmark and document performance
  - [x] Small project (10 files): <50ms (10.5ms achieved - 4.8x better)
  - [x] Medium project (100 files): <500ms (61ms achieved - 8.2x better)
  - [x] Large project (1000 files): <5s (425ms achieved - 11.8x better)

### Success Criteria

- [x] Indexing meets performance targets (exceeds by 8-12x)
- [x] Bottlenecks identified and documented
- [x] Profiling tools integrated for future optimization

### Tests

- [x] Benchmark: Generate fixture projects of various sizes
- [x] Benchmark: Measure indexing time for each size
- [x] Benchmark: Compare incremental vs full indexing
- [x] Unit tests: upsert_batch functionality (3 new tests)

### Implementation Notes

**Subtask 1: Performance Instrumentation** (Completed)
- Created `profiler.rs` module with `IndexProfiler` and `ProfileReport`
- Non-invasive RAII-based timing via `PhaseGuard`
- Tracks phase times, per-file parse times, and DB operations
- Configurable via `IndexerConfig::with_profiling()`
- JSON serialization for analysis
- <1% overhead when enabled
- All 8 unit tests passing

**Subtask 2: Benchmark Infrastructure** (Completed)
- Created `benches/indexing.rs` with Criterion benchmarks
- Project sizes: small (10 files), medium (100 files), large (1000 files)
- Scenarios: full indexing, incremental (no changes), incremental (10% modified), incremental (10% churn)
- Profiling overhead benchmark (measures <2% impact)
- HTML reports generated in `target/criterion/`

**Baseline Performance Results (Before Optimization):**
- Small project (10 files): ~12ms ✓ (target: <50ms)
- Medium project (100 files): ~73ms ✓ (target: <500ms)
- Large project (1000 files): ~700ms ✓ (target: <5s)
- Incremental (no changes) - Small: ~1.4ms
- Incremental (no changes) - Medium: ~4ms
- Incremental (no changes) - Large: ~32ms
- Profiling overhead: ~1.4% (73ms → 74ms)

All performance targets met! ✓

**Subtask 3: Optimize Bottlenecks** (Completed)
Implemented batch file upsert optimization targeting the DB bottleneck:

1. **Added `FileRepository::upsert_batch()` method**
   - Uses SQLite's `INSERT ... ON CONFLICT ... DO UPDATE` for efficient upserts
   - Single transaction for all files
   - Returns `HashMap<PathBuf, i64>` with path→ID mappings
   - Eliminates separate existence checks and ID lookups

2. **Refactored `Indexer::index_project()` to use batch operations**
   - Changed from: N × (get_by_path → insert/update → get_by_path)
   - Changed to: Single upsert_batch call
   - Preserved correct reporting of files_added vs files_updated
   - Maintained all progress reporting functionality

3. **Added comprehensive tests**
   - `test_upsert_batch_insert` - Pure inserts
   - `test_upsert_batch_update` - Pure updates
   - `test_upsert_batch_mixed` - Mixed insert/update scenario
   - All 238 existing tests still passing

**Performance Improvements:**
- Small (10 files): 11.6ms → 10.5ms (**9.4% faster**) - now 4.8x better than target
- Medium (100 files): 73ms → 61ms (**12.5% faster**) - now 8.2x better than target
- Large (1000 files): 698ms → 425ms (**39% faster**) - now 11.8x better than target
- Incremental (no changes) - Large: 32ms → 18ms (**44% faster**)
- Incremental (10% modified) - Large: 110ms → 18ms (**84% faster**)

**Subtask 4: Evaluate Caching Layer** (Completed - Not Implemented)
Analyzed potential caching opportunities and determined none are needed:

1. **File ID Cache:** Not needed - `upsert_batch()` returns path→ID mapping
2. **Task ID Cache:** Not needed - tasks inserted in batch per file
3. **Hash Cache:** Already in DB via file records
4. **Dependency Graph Cache:** Already optimized via closure table

**Rationale:** Indexing is typically one-shot operation, not repeated in tight loops. The batch operations provide efficient access patterns without additional caching complexity.

**Decision:** No caching layer needed for v1.0. Monitor real-world usage and add only if profiling shows benefit.

---

## Non-Goals (for v1)

- Real-time file watching (use manual `lash index` or git hooks)
- Distributed indexing across machines
- Index compression or advanced storage optimizations

---

## Design Decisions (Resolved)

- **Hash algorithm:** blake3 ✅ (10x faster than SHA-256, sufficient for integrity checking)
- **Parallelism level:** Auto-detect CPU cores with `--jobs N` override ✅
- **Transaction granularity:** Single transaction with savepoints every 100 files ✅
- **Error handling:** Continue on parse errors and collect all ✅ (better UX, consistent with parser)
- **Project root markers:** `.lash/` directory (precedence) or `lash.index.md` file ✅
- **.gitignore respect:** Yes by default, with `--no-ignore` flag ✅
- **File walking:** Use `ignore` crate (battle-tested, same as ripgrep) ✅

## Open Questions

- **Index metadata tracking:** What additional metadata should be stored in the `metadata` table?
- **Progress reporting details:** Callback signature and event granularity for TUI integration?

---

## References

- Design doc section 7.2 (Indexing commands)
- Design doc section 9.1 (Acceleration layer principles)
- Design doc section 13.2 (Performance considerations)
