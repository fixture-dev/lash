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
