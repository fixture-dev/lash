# SQLite Schema Tasks

**Module:** Database Layer
**Priority:** CRITICAL
**Estimated Duration:** 7-9 days
**Dependencies:** tasks.core-data-model (all tasks)

## Overview

Design and implement the SQLite database schema that serves as the acceleration layer for Lash. The database is fully reconstructible from Markdown files and optimized for fast queries.

**Key Principle:** Markdown is the source of truth; SQLite is for performance.

**Design:** See `crates/lash-db/schema.sql` for the authoritative schema.

## Tasks

### 1. Design Schema

- [x] **Finalize table structures**
  - [x] Review initial schema design proposal
  - [x] Adjust based on final data model from core-data-model tasks
  - [x] Document final schema in `docs/sqlite-schema.md`
- [x] **Design `files` table**
  - [x] Columns:
    - [x] `id` INTEGER PRIMARY KEY
    - [x] `path` TEXT UNIQUE NOT NULL - Relative path from root
    - [x] `file_id` TEXT UNIQUE NOT NULL - File identifier (from @id or path)
    - [x] `title` TEXT NOT NULL
    - [x] `hash` TEXT NOT NULL - blake3 content hash
    - [x] `mtime` INTEGER NOT NULL - Unix timestamp
    - [x] `status` TEXT - Computed overall status
    - [x] `metadata` TEXT - JSON blob for FileMetadata
    - [x] `indexed_at` INTEGER - When indexed
  - [x] Indexes:
    - [x] UNIQUE on path
    - [x] UNIQUE on file_id
    - [x] INDEX on status
    - [x] INDEX on hash (for change detection)
- [x] **Design `tasks` table**
  - [x] Columns:
    - [x] `id` INTEGER PRIMARY KEY
    - [x] `file_id` INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE
    - [x] `local_id` TEXT NOT NULL - Task ID within file
    - [x] `full_id` TEXT UNIQUE NOT NULL - file_id#local_id
    - [x] `title` TEXT NOT NULL
    - [x] `status` TEXT NOT NULL - open, done, waived, blocked
    - [x] `depth` INTEGER NOT NULL - Nesting level (0-2)
    - [x] `parent_id` INTEGER REFERENCES tasks(id) ON DELETE CASCADE
    - [x] `order_index` INTEGER NOT NULL - Position among siblings
    - [x] `owner` TEXT
    - [x] `estimate` TEXT
    - [x] `body` TEXT - Extended description
    - [x] `metadata` TEXT - JSON blob for TaskMetadata
  - [x] Indexes:
    - [x] INDEX on file_id
    - [x] UNIQUE on full_id
    - [x] INDEX on status
    - [x] INDEX on (file_id, order_index) - For ordered retrieval
    - [x] INDEX on parent_id - For hierarchy queries
- [x] **Design `dependencies` table**
  - [x] Columns:
    - [x] `id` INTEGER PRIMARY KEY
    - [x] `from_task_id` INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE
    - [x] `to_task_id` INTEGER REFERENCES tasks(id) ON DELETE CASCADE
    - [x] `kind` TEXT NOT NULL - hierarchy, explicit_id, explicit_path, directory
    - [x] `raw_ref` TEXT - Original reference string
  - [x] Indexes:
    - [x] INDEX on from_task_id
    - [x] INDEX on to_task_id
    - [x] INDEX on kind
  - [x] Note: to_task_id can be NULL for unresolved references
- [x] **Design `dependency_closure` table** (transitive closure)
  - [x] Columns:
    - [x] `ancestor_id` INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE
    - [x] `descendant_id` INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE
    - [x] `depth` INTEGER NOT NULL - Distance (1 = direct, 2+ = indirect)
  - [x] Indexes:
    - [x] PRIMARY KEY (ancestor_id, descendant_id)
    - [x] INDEX on descendant_id (for finding all ancestors)
  - [x] Used for fast "is A an ancestor of B?" queries
- [x] **Design `labels` table**
  - [x] Columns:
    - [x] `id` INTEGER PRIMARY KEY
    - [x] `name` TEXT UNIQUE NOT NULL - Normalized label name
  - [x] Indexes:
    - [x] UNIQUE on name
- [x] **Design `task_labels` junction table**
  - [x] Columns:
    - [x] `task_id` INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE
    - [x] `label_id` INTEGER NOT NULL REFERENCES labels(id) ON DELETE CASCADE
  - [x] Indexes:
    - [x] PRIMARY KEY (task_id, label_id)
    - [x] INDEX on label_id (for label queries)
- [x] **Design `file_labels` junction table**
  - [x] Columns:
    - [x] `file_id` INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE
    - [x] `label_id` INTEGER NOT NULL REFERENCES labels(id) ON DELETE CASCADE
  - [x] Indexes:
    - [x] PRIMARY KEY (file_id, label_id)
    - [x] INDEX on label_id
- [x] **Design `metadata` table** (schema version, stats)
  - [x] Columns:
    - [x] `key` TEXT PRIMARY KEY
    - [x] `value` TEXT
  - [x] Store:
    - [x] `schema_version` - Current version (for migrations)
    - [x] `project_root` - Root path
    - [x] `last_indexed` - Timestamp
    - [x] `total_files` - Count
    - [x] `total_tasks` - Count
- [x] **Design FTS5 virtual table for search**
  - [x] Table: `tasks_fts`
  - [x] Columns:
    - [x] `full_id` - Join key
    - [x] `title` - Searchable
    - [x] `body` - Searchable
  - [x] Configuration:
    - [x] Tokenizer: unicode61
    - [x] Content table: tasks
    - [x] BM25 ranking
- [x] **Document schema**
  - [x] Create SQL schema file: `lash-db/schema.sql`
  - [x] Add ER diagram to docs
  - [x] Document all indexes and their purpose
  - [x] Document foreign key cascades
  - [x] Document JSON blob structures

**Priority:** CRITICAL
**Estimate:** 1 day
**Dependencies:** tasks.core-data-model#6
**Success Criteria:** Schema fully documented; SQL DDL ready

---

### 2. Implement Schema Creation

- [x] **Set up SQLite integration**
  - [x] Add `rusqlite` dependency to `lash-db`
  - [x] Add `rusqlite` with features: `bundled`, `blob`, `time`
  - [x] Create `lash-db/src/connection.rs` for connection management
- [x] **Create database initialization**
  - [x] Function: `init_database(path: &Path) -> Result<Connection>`
  - [x] Create database file if doesn't exist
  - [x] Run schema DDL
  - [x] Set PRAGMAs:
    - [x] `PRAGMA foreign_keys = ON` - Enforce FK constraints
    - [x] `PRAGMA journal_mode = WAL` - Better concurrency
    - [x] `PRAGMA synchronous = NORMAL` - Balance safety/speed
    - [x] `PRAGMA temp_store = MEMORY` - Faster temp operations
  - [x] Initialize metadata table with schema version
- [x] **Implement schema migrations**
  - [x] Create `lash-db/src/migrations.rs`
  - [x] Define `Migration` trait
  - [x] Track current schema version in metadata table
  - [x] Function: `run_migrations(conn: &Connection) -> Result<()>`
  - [x] Apply migrations in order
  - [x] Record successful migrations
  - [x] For v1: Only initial schema (no migrations yet)
- [x] **Create schema DDL**
  - [x] Write CREATE TABLE statements for all tables
  - [x] Create all indexes
  - [x] Create FTS5 virtual table with triggers
  - [x] Create triggers for FTS5 updates:
    - [x] INSERT trigger: Add to FTS index
    - [x] UPDATE trigger: Update FTS index
    - [x] DELETE trigger: Remove from FTS index
- [x] **Implement schema version tracking**
  - [x] Store version in metadata table: `schema_version = 1`
  - [x] Function: `get_schema_version(conn: &Connection) -> Result<i32>`
  - [x] Function: `set_schema_version(conn: &Connection, version: i32) -> Result<()>`
  - [x] Check version on open, run migrations if needed
- [x] **Create connection pool**
  - [x] Add `r2d2` and `r2d2_sqlite` dependencies
  - [x] Create `lash-db/src/pool.rs`
  - [x] Function: `create_pool(db_path: &Path) -> Result<Pool>`
  - [x] Configure pool size (default: 4 connections)
  - [x] Provide `get_connection()` method
- [x] **Write tests**
  - [x] Test database initialization (creates file, runs DDL)
  - [x] Test PRAGMA settings
  - [x] Test schema version tracking
  - [x] Test connection pool
  - [x] Test foreign key enforcement
  - [x] Test FTS triggers
  - [x] 15+ tests

**Priority:** CRITICAL
**Estimate:** 1 day
**Dependencies:** Task #1
**Success Criteria:** Database initializes correctly; schema created; FKs enforced

---

### 3. Implement File Repository

- [x] **Create `FileRepository` in `lash-db/src/repository/files.rs`**
  - [x] Struct holds database connection
  - [x] Methods for CRUD operations
- [x] **Implement insert file**
  - [x] Method: `insert(&self, file: &TaskFile) -> Result<i64>`
  - [x] INSERT INTO files
  - [x] Serialize metadata to JSON
  - [x] Return auto-generated file.id
  - [x] Handle unique constraint violations (path, file_id)
- [x] **Implement update file**
  - [x] Method: `update(&self, file: &TaskFile) -> Result<()>`
  - [x] UPDATE files WHERE path = ?
  - [x] Update hash, mtime, status, metadata
  - [x] Return error if file not found
- [x] **Implement delete file**
  - [x] Method: `delete(&self, path: &Path) -> Result<()>`
  - [x] DELETE FROM files WHERE path = ?
  - [x] Cascades to tasks, labels (via FK)
  - [x] Return Ok even if not found (idempotent)
- [x] **Implement query by path**
  - [x] Method: `get_by_path(&self, path: &Path) -> Result<Option<FileRecord>>`
  - [x] SELECT * FROM files WHERE path = ?
  - [x] Deserialize JSON metadata
  - [x] Return None if not found
- [x] **Implement query by file_id**
  - [x] Method: `get_by_file_id(&self, file_id: &str) -> Result<Option<FileRecord>>`
  - [x] SELECT * FROM files WHERE file_id = ?
- [x] **Implement query by label**
  - [x] Method: `find_by_label(&self, label: &str) -> Result<Vec<FileRecord>>`
  - [x] JOIN file_labels and labels
  - [x] Return all files with that label
- [x] **Implement list all files**
  - [x] Method: `list_all(&self) -> Result<Vec<FileRecord>>`
  - [x] SELECT * FROM files ORDER BY path
  - [x] Used for index verification
- [x] **Implement bulk operations**
  - [x] Method: `insert_batch(&self, files: &[TaskFile]) -> Result<()>`
  - [x] Use transaction
  - [x] Prepare statement once, execute multiple times
  - [x] Much faster than individual inserts
- [x] **Implement change detection**
  - [x] Method: `get_changed_files(&self, root: &Path) -> Result<Vec<PathBuf>>`
  - [x] Compare filesystem mtime/hash with DB
  - [x] Return list of paths that need re-indexing
- [x] **Define `FileRecord` struct**
  - [x] Represents row from files table
  - [x] Includes all columns plus deserialized metadata
  - [x] Can convert to/from TaskFile
- [x] **Write tests**
  - [x] Insert file
  - [x] Update file
  - [x] Delete file
  - [x] Query by path (found and not found)
  - [x] Query by file_id
  - [x] Query by label
  - [x] List all
  - [x] Bulk insert (performance)
  - [x] Change detection
  - [x] 20+ tests

**Priority:** HIGH
**Estimate:** 1.5 days
**Dependencies:** Task #2
**Success Criteria:** Can CRUD file records; bulk operations work

---

### 4. Implement Task Repository

- [x] **Create `TaskRepository` in `lash-db/src/repository/tasks.rs`**
  - [x] Struct holds database connection
  - [x] Methods for CRUD and queries
- [x] **Implement insert task**
  - [x] Method: `insert(&self, task: &Task, file_db_id: i64) -> Result<i64>`
  - [x] INSERT INTO tasks
  - [x] Serialize metadata to JSON
  - [x] Return task.id
- [x] **Implement update task**
  - [x] Method: `update(&self, task: &Task) -> Result<()>`
  - [x] UPDATE tasks WHERE full_id = ?
  - [x] Update status, title, metadata, etc.
- [x] **Implement delete task**
  - [x] Method: `delete(&self, full_id: &str) -> Result<()>`
  - [x] DELETE FROM tasks WHERE full_id = ?
  - [x] Cascades to dependencies
- [x] **Implement query by full_id**
  - [x] Method: `get_by_full_id(&self, full_id: &str) -> Result<Option<TaskRecord>>`
  - [x] SELECT * FROM tasks WHERE full_id = ?
- [x] **Implement query by file**
  - [x] Method: `get_by_file(&self, file_id: i64) -> Result<Vec<TaskRecord>>`
  - [x] SELECT * FROM tasks WHERE file_id = ? ORDER BY order_index
  - [x] Returns tasks in document order
- [x] **Implement query by status**
  - [x] Method: `find_by_status(&self, status: TaskStatus) -> Result<Vec<TaskRecord>>`
  - [x] SELECT * FROM tasks WHERE status = ?
  - [x] Useful for "list all open tasks"
- [x] **Implement query by label**
  - [x] Method: `find_by_label(&self, label: &str) -> Result<Vec<TaskRecord>>`
  - [x] JOIN task_labels and labels
  - [x] Return all tasks with that label
- [x] **Implement hierarchical queries**
  - [x] Method: `get_children(&self, task_id: i64) -> Result<Vec<TaskRecord>>`
  - [x] SELECT * FROM tasks WHERE parent_id = ? ORDER BY order_index
  - [x] Direct children only
  - [x] Method: `get_descendants(&self, task_id: i64) -> Result<Vec<TaskRecord>>`
  - [x] Recursive query or use dependency_closure table
  - [x] All descendants (children, grandchildren, etc.)
  - [x] Method: `get_ancestors(&self, task_id: i64) -> Result<Vec<TaskRecord>>`
  - [x] Walk up parent_id chain
  - [x] Or use dependency_closure table
- [x] **Implement filtering and sorting**
  - [x] Method: `find(&self, filter: TaskFilter) -> Result<Vec<TaskRecord>>`
  - [x] `TaskFilter` struct with optional fields:
    - [x] `status: Option<TaskStatus>`
    - [x] `labels: Vec<String>`
    - [x] `owner: Option<String>`
    - [x] `file_path: Option<String>`
    - [x] `blocked: Option<bool>`
  - [x] Build WHERE clause dynamically
  - [x] Support multiple filter criteria (AND)
  - [x] Sort options: by file, by status, by created date
- [x] **Implement bulk operations**
  - [x] Method: `insert_batch(&self, tasks: &[Task], file_id: i64) -> Result<()>`
  - [x] Use transaction
  - [x] Prepared statement
- [x] **Define `TaskRecord` struct**
  - [x] Represents row from tasks table
  - [x] Includes all columns plus deserialized metadata
  - [x] Can convert to/from Task
- [x] **Write tests**
  - [x] CRUD operations
  - [x] Query by full_id
  - [x] Query by file
  - [x] Query by status
  - [x] Query by label
  - [x] Hierarchical queries (children, descendants, ancestors)
  - [x] Filtering with multiple criteria
  - [x] Sorting
  - [x] Bulk insert
  - [x] 30+ tests

**Priority:** HIGH
**Estimate:** 2 days
**Dependencies:** Task #2
**Success Criteria:** Can CRUD task records efficiently; hierarchical queries work

---

### 5. Implement Dependency Repository

- [x] **Create `DependencyRepository` in `lash-db/src/repository/dependencies.rs`**
- [x] **Implement insert dependency**
  - [x] Method: `insert(&self, dep: &Dependency) -> Result<i64>`
  - [x] INSERT INTO dependencies
  - [x] Return dependency id
- [x] **Implement delete dependency**
  - [x] Method: `delete(&self, from_task_id: i64, to_task_id: i64) -> Result<()>`
  - [x] DELETE FROM dependencies WHERE ...
- [x] **Implement query dependencies (outgoing)**
  - [x] Method: `get_dependencies(&self, task_id: i64) -> Result<Vec<DependencyRecord>>`
  - [x] SELECT * FROM dependencies WHERE from_task_id = ?
  - [x] Returns tasks that `task_id` depends ON
- [x] **Implement query dependents (incoming)**
  - [x] Method: `get_dependents(&self, task_id: i64) -> Result<Vec<DependencyRecord>>`
  - [x] SELECT * FROM dependencies WHERE to_task_id = ?
  - [x] Returns tasks that depend on `task_id`
- [x] **Implement transitive closure queries**
  - [x] Method: `get_all_dependencies(&self, task_id: i64) -> Result<Vec<i64>>`
  - [x] SELECT descendant_id FROM dependency_closure WHERE ancestor_id = ?
  - [x] All transitive dependencies
  - [x] Method: `get_all_dependents(&self, task_id: i64) -> Result<Vec<i64>>`
  - [x] SELECT ancestor_id FROM dependency_closure WHERE descendant_id = ?
  - [x] All transitive dependents
- [x] **Implement cycle detection query**
  - [x] Method: `is_cyclic(&self, from: i64, to: i64) -> Result<bool>`
  - [x] Check: Would adding edge (from → to) create cycle?
  - [x] Query dependency_closure: is `from` already a descendant of `to`?
  - [x] Fast O(1) lookup with closure table
- [x] **Implement build transitive closure**
  - [x] Method: `rebuild_closure(&self) -> Result<()>`
  - [x] DELETE FROM dependency_closure
  - [x] Build closure from dependencies table:
    - [x] Insert direct edges (depth = 1)
    - [x] Iteratively add transitive edges
    - [x] Or use recursive CTE in SQLite
  - [x] Called after bulk dependency changes
- [x] **Implement incremental closure update**
  - [x] Method: `update_closure_for_edge(&self, from: i64, to: i64) -> Result<()>`
  - [x] Add edge (from → to) and all implied transitive edges
  - [x] Algorithm:
    - [x] For each ancestor A of `from`: add edge (A → to)
    - [x] For each descendant D of `to`: add edge (from → D)
    - [x] For each (A, D) pair: add edge (A → D)
  - [x] More efficient than full rebuild for small changes
- [x] **Implement bulk operations**
  - [x] Method: `insert_batch(&self, deps: &[Dependency]) -> Result<()>`
  - [x] Use transaction
  - [x] Rebuild closure at end (more efficient than incremental)
- [x] **Define `DependencyRecord` struct**
  - [x] Represents row from dependencies table
  - [x] Can convert to/from Dependency
- [x] **Write tests**
  - [x] Insert/delete dependency
  - [x] Query dependencies (outgoing)
  - [x] Query dependents (incoming)
  - [x] Transitive closure queries
  - [x] Cycle detection (various patterns)
  - [x] Rebuild closure
  - [x] Incremental closure update
  - [x] Bulk insert
  - [x] 25+ tests

**Priority:** HIGH
**Estimate:** 1.5 days
**Dependencies:** Task #2
**Success Criteria:** Can manage dependency graph in DB; cycle detection works

---

### 6. Implement Label Repository

- [x] **Create `LabelRepository` in `lash-db/src/repository/labels.rs`**
- [x] **Implement get or create label**
  - [x] Method: `get_or_create(&self, name: &str) -> Result<i64>`
  - [x] Normalize label name
  - [x] SELECT id FROM labels WHERE name = ?
  - [x] If not found: INSERT and return new id
  - [x] If found: return existing id
  - [x] Handle race conditions (UNIQUE constraint)
- [x] **Implement associate task with label**
  - [x] Method: `add_task_label(&self, task_id: i64, label_id: i64) -> Result<()>`
  - [x] INSERT INTO task_labels
  - [x] Ignore if already exists (INSERT OR IGNORE)
- [x] **Implement associate file with label**
  - [x] Method: `add_file_label(&self, file_id: i64, label_id: i64) -> Result<()>`
  - [x] INSERT INTO file_labels
- [x] **Implement remove associations**
  - [x] Method: `remove_task_label(&self, task_id: i64, label_id: i64) -> Result<()>`
  - [x] DELETE FROM task_labels WHERE ...
  - [x] Method: `remove_file_label(&self, file_id: i64, label_id: i64) -> Result<()>`
- [x] **Implement query labels for task**
  - [x] Method: `get_task_labels(&self, task_id: i64) -> Result<Vec<String>>`
  - [x] JOIN task_labels and labels
  - [x] Return label names
- [x] **Implement query labels for file**
  - [x] Method: `get_file_labels(&self, file_id: i64) -> Result<Vec<String>>`
- [x] **Implement query all labels**
  - [x] Method: `list_all(&self) -> Result<Vec<LabelRecord>>`
  - [x] SELECT * FROM labels ORDER BY name
  - [x] Include counts (how many tasks/files per label)
  - [x] Useful for autocomplete
- [x] **Implement batch label operations**
  - [x] Method: `set_task_labels(&self, task_id: i64, labels: &[String]) -> Result<()>`
  - [x] Delete existing associations
  - [x] Get or create label IDs
  - [x] Insert new associations
  - [x] Use transaction
  - [x] Method: `set_file_labels(&self, file_id: i64, labels: &[String]) -> Result<()>`
- [x] **Implement label statistics**
  - [x] Method: `get_label_stats(&self) -> Result<Vec<LabelStats>>`
  - [x] For each label: count of tasks, count of files
  - [x] Used for reporting popular labels
- [x] **Define `LabelRecord` and `LabelStats` structs**
- [x] **Write tests**
  - [x] Get or create (new and existing)
  - [x] Add/remove task label
  - [x] Add/remove file label
  - [x] Query labels for task/file
  - [x] List all labels
  - [x] Batch set labels
  - [x] Label statistics
  - [x] 15+ tests

**Priority:** MEDIUM
**Estimate:** 1 day
**Dependencies:** Task #2
**Success Criteria:** Can manage labels efficiently; batch operations work

---

## Summary

### Total Estimate
**7-9 days** total for SQLite schema implementation

### Completion Criteria
- [x] All tasks above completed
- [x] Database schema created and documented
- [x] All repositories implemented (File, Task, Dependency, Label)
- [x] Transitive closure for fast dependency queries
- [x] FTS5 for full-text search
- [x] Comprehensive tests for all CRUD operations
- [x] Performance acceptable for 1000+ task projects

### Schema Summary

**Tables (9):**
1. `files` - Task files
2. `tasks` - Individual tasks
3. `dependencies` - Dependency edges
4. `dependency_closure` - Transitive closure
5. `labels` - Unique labels
6. `task_labels` - Task-label associations
7. `file_labels` - File-label associations
8. `metadata` - Schema version and stats
9. `tasks_fts` - FTS5 virtual table

**Repositories (4):**
1. `FileRepository` - CRUD for files
2. `TaskRepository` - CRUD for tasks, filtering, hierarchical queries
3. `DependencyRepository` - Dependency graph management, cycle detection
4. `LabelRepository` - Label management, associations

### Performance Optimizations

- WAL mode for better concurrency
- Connection pooling (r2d2)
- Bulk operations with transactions
- Prepared statements
- Appropriate indexes on all query paths
- Transitive closure table for O(1) reachability

### Next Steps

After completing SQLite schema, proceed to:
1. **tasks.indexing.md** - Use repositories to build index from files
2. **tasks.dependency-resolution.md** - Build dependency graph using queries
