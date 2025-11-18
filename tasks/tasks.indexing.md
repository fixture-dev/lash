# Indexing Engine Tasks

**Module:** `lash-db` (indexing layer)
**Dependencies:** tasks.sqlite-schema.md, tasks.markdown-parser.md, tasks.core-data-model.md
**Effort:** 8-12 days
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

## Task 1: File System Walker

**Priority:** CRITICAL
**Effort:** 1-2 days
**Depends on:** tasks.core-data-model.md#1

### Description

Implement recursive directory traversal to discover all Markdown files in a Lash project tree.

### Subtasks

- [ ] Implement `FileWalker` struct with configuration
  - [ ] Support for starting from project root
  - [ ] Configurable file extensions (`.md`)
  - [ ] Exclude patterns (`.git/`, `node_modules/`, etc.)
  - [ ] Follow symlinks option (default: false for safety)
- [ ] Implement `discover_files()` function
  - [ ] Start from project root (locate `lash.index.md` or `.lash/`)
  - [ ] Recursively walk directories
  - [ ] Filter by extension and patterns
  - [ ] Return list of file paths with metadata
- [ ] Add file metadata collection
  - [ ] Absolute path
  - [ ] Relative path from project root
  - [ ] File size
  - [ ] Modification time (mtime)
  - [ ] Compute content hash (blake3 or similar)
- [ ] Handle edge cases
  - [ ] Permission denied errors (skip and warn)
  - [ ] Broken symlinks (skip and warn)
  - [ ] Very large directories (streaming/iterator approach)
  - [ ] Unicode filenames

### Success Criteria

- Can discover all `.md` files in test fixture directories
- Correctly excludes ignored patterns
- Returns accurate file metadata
- Handles errors gracefully without panicking
- Performance: <100ms for 1000 files

### Tests

- Unit: Test walker on fixture directories with various structures
- Unit: Test exclusion patterns work correctly
- Unit: Test symlink handling
- Integration: Test on real project structure

---

## Task 2: Incremental Indexing Logic

**Priority:** HIGH
**Effort:** 2-3 days
**Depends on:** Task 1, tasks.sqlite-schema.md#1

### Description

Implement logic to detect which files need re-parsing by comparing filesystem state with database records.

### Subtasks

- [ ] Implement `IndexDiff` struct to track changes
  - [ ] New files (not in DB)
  - [ ] Modified files (hash or mtime changed)
  - [ ] Deleted files (in DB but not on filesystem)
  - [ ] Unchanged files (skip parsing)
- [ ] Implement `compute_index_diff()` function
  - [ ] Query DB for existing file records
  - [ ] Compare hashes/mtimes with filesystem
  - [ ] Build diff structure
  - [ ] Handle missing DB (full reindex case)
- [ ] Implement hash computation strategy
  - [ ] Use blake3 for speed
  - [ ] Hash only file content (not metadata)
  - [ ] Cache hashes in DB
- [ ] Add fast-path optimizations
  - [ ] If mtime unchanged and hash exists, skip re-hashing
  - [ ] Batch DB queries for efficiency
  - [ ] Parallelize hash computation for large projects

### Success Criteria

- Correctly identifies new, modified, and deleted files
- Fast path: unchanged files detected in <10ms each
- Handles edge cases (clock skew, manual DB edits)
- Accurate diff even with concurrent file modifications

### Tests

- Unit: Test diff computation with various scenarios
- Unit: Test hash stability across runs
- Integration: Test incremental indexing after file modifications
- Performance: Benchmark diff computation for 1000 files

---

## Task 3: Index Execution Engine

**Priority:** CRITICAL
**Effort:** 3-4 days
**Depends on:** Task 2, tasks.markdown-parser.md#1-2

### Description

Coordinate the full indexing process: parse files, populate DB, handle errors, and report progress.

### Subtasks

- [ ] Implement `Indexer` struct
  - [ ] Hold references to DB connection, parser, file walker
  - [ ] Configuration (incremental vs full, parallelism level)
  - [ ] Progress tracking
- [ ] Implement `index_project()` function
  - [ ] Discover files (Task 1)
  - [ ] Compute diff (Task 2)
  - [ ] Parse modified/new files
  - [ ] Begin DB transaction
  - [ ] Delete records for removed files
  - [ ] Insert/update records for new/modified files
  - [ ] Commit transaction
  - [ ] Handle rollback on errors
- [ ] Add parallel parsing support
  - [ ] Parse files in parallel using rayon or similar
  - [ ] Collect results and errors
  - [ ] Insert into DB in single thread (SQLite limitation)
- [ ] Implement progress reporting
  - [ ] Track files processed vs total
  - [ ] Emit progress events (for CLI/TUI)
  - [ ] Support quiet mode (no output)
- [ ] Error aggregation
  - [ ] Collect all parse errors (don't stop on first)
  - [ ] Associate errors with file paths
  - [ ] Return structured error report

### Success Criteria

- Can index a project from scratch successfully
- Incremental indexing correctly updates only changed files
- Handles parse errors gracefully (collects all, continues)
- Progress reporting works for long-running operations
- Transaction safety: DB left in consistent state on error

### Tests

- Integration: Index empty project
- Integration: Index project with valid files
- Integration: Index project with parse errors (verify error collection)
- Integration: Incremental indexing after modifications
- Integration: Verify DB consistency after index

---

## Task 4: Index Verification

**Priority:** HIGH
**Effort:** 2-3 days
**Depends on:** Task 3

### Description

Implement the `lash check-index` command to verify database consistency with Markdown files.

### Subtasks

- [ ] Implement `IndexVerifier` struct
  - [ ] Compare DB records with filesystem
  - [ ] Detect drift (DB out of sync)
  - [ ] Report discrepancies
- [ ] Implement verification checks
  - [ ] Files in DB but not on filesystem
  - [ ] Files on filesystem but not in DB
  - [ ] Hash mismatches (file modified but not reindexed)
  - [ ] Orphaned task records (file deleted but tasks remain)
  - [ ] Orphaned dependency records
- [ ] Implement `verify_index()` function
  - [ ] Run all checks
  - [ ] Collect discrepancies
  - [ ] Return verification report
- [ ] Add fix suggestions
  - [ ] "Run `lash index` to resync"
  - [ ] "Remove stale DB records"
  - [ ] Auto-fix option (with confirmation)

### Success Criteria

- Detects all common drift scenarios
- Clear, actionable error messages
- Fast verification (<500ms for 1000 files)
- Optional auto-fix works safely

### Tests

- Unit: Test each verification check independently
- Integration: Verify clean project (no drift)
- Integration: Introduce drift and verify detection
- Integration: Test auto-fix functionality

---

## Task 5: Incremental Dependency Re-resolution

**Priority:** MEDIUM
**Effort:** 2-3 days
**Depends on:** Task 3, tasks.dependency-resolution.md#1-2

### Description

When files change, efficiently update only affected dependency edges without full graph recomputation.

### Subtasks

- [ ] Implement `DependencyUpdater` struct
  - [ ] Identify tasks affected by file changes
  - [ ] Delete stale dependency edges
  - [ ] Re-resolve dependencies for affected tasks
- [ ] Implement `update_dependencies()` function
  - [ ] Query tasks in modified files
  - [ ] Delete dependency edges from/to these tasks
  - [ ] Re-run dependency resolution (Task 3 from dependency-resolution.md)
  - [ ] Insert new edges
- [ ] Optimize for minimal graph updates
  - [ ] Only update edges for changed tasks
  - [ ] Preserve edges for unchanged tasks
  - [ ] Batch DB operations
- [ ] Handle cascading updates
  - [ ] If task IDs change, update references
  - [ ] If dependencies break, mark tasks as blocked

### Success Criteria

- Incremental dependency updates are faster than full resolution
- Dependency graph remains consistent after updates
- Broken references detected and reported

### Tests

- Integration: Modify file with dependencies, verify edges updated
- Integration: Add new dependency, verify edge created
- Integration: Remove dependency, verify edge deleted
- Performance: Compare incremental vs full resolution time

---

## Task 6: Index Performance Optimization

**Priority:** MEDIUM
**Effort:** 2-3 days
**Depends on:** Task 3

### Description

Profile and optimize indexing performance for large projects (1000+ files).

### Subtasks

- [ ] Add performance instrumentation
  - [ ] Measure time per indexing phase
  - [ ] Track DB query times
  - [ ] Track parse times
  - [ ] Memory usage profiling
- [ ] Optimize bottlenecks
  - [ ] Batch INSERT statements (use transactions effectively)
  - [ ] Parallelize file parsing (already in Task 3)
  - [ ] Optimize hash computation (memory-mapped files?)
  - [ ] Use prepared statements for DB queries
- [ ] Add caching layer
  - [ ] Cache frequently queried data (file IDs, task IDs)
  - [ ] Cache dependency graph structure
- [ ] Benchmark and document performance
  - [ ] Small project (10 files): <50ms
  - [ ] Medium project (100 files): <500ms
  - [ ] Large project (1000 files): <5s

### Success Criteria

- Indexing meets performance targets
- Bottlenecks identified and documented
- Profiling tools integrated for future optimization

### Tests

- Benchmark: Generate fixture projects of various sizes
- Benchmark: Measure indexing time for each size
- Benchmark: Compare incremental vs full indexing

---

## Non-Goals (for v1)

- Real-time file watching (use manual `lash index` or git hooks)
- Distributed indexing across machines
- Index compression or advanced storage optimizations

---

## Open Questions

- **Hash algorithm:** blake3 vs SHA-256? (blake3 is faster)
- **Parallelism level:** Auto-detect CPU cores vs configurable?
- **Transaction granularity:** One transaction for entire index vs per-file?
- **Error handling:** Continue on parse errors vs fail-fast?

---

## References

- Design doc section 7.2 (Indexing commands)
- Design doc section 9.1 (Acceleration layer principles)
- Design doc section 13.2 (Performance considerations)
