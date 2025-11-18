# SQLite Schema Tasks

**Module:** Database Layer
**Priority:** CRITICAL
**Estimated Duration:** 7-9 days
**Dependencies:** tasks.core-data-model (all tasks)

## Overview

Design and implement the SQLite database schema that serves as the acceleration layer for Lash. The database is fully reconstructible from Markdown files and optimized for fast queries.

**Key Principle:** Markdown is the source of truth; SQLite is for performance.

**Design:** See `docs/dependency-graph-analysis.md` for detailed schema recommendations.

## Tasks

### 1. Design Schema

- [ ] **Finalize table structures**
  - [ ] Review schema design from dependency-graph-analysis.md
  - [ ] Adjust based on final data model from core-data-model tasks
  - [ ] Document final schema in `docs/sqlite-schema.md`
- [ ] **Design `files` table**
  - [ ] Columns:
    - [ ] `id` INTEGER PRIMARY KEY
    - [ ] `path` TEXT UNIQUE NOT NULL - Relative path from root
    - [ ] `file_id` TEXT UNIQUE NOT NULL - File identifier (from @id or path)
    - [ ] `title` TEXT NOT NULL
    - [ ] `hash` TEXT NOT NULL - blake3 content hash
    - [ ] `mtime` INTEGER NOT NULL - Unix timestamp
    - [ ] `status` TEXT - Computed overall status
    - [ ] `metadata` TEXT - JSON blob for FileMetadata
    - [ ] `indexed_at` INTEGER - When indexed
  - [ ] Indexes:
    - [ ] UNIQUE on path
    - [ ] UNIQUE on file_id
    - [ ] INDEX on status
    - [ ] INDEX on hash (for change detection)
- [ ] **Design `tasks` table**
  - [ ] Columns:
    - [ ] `id` INTEGER PRIMARY KEY
    - [ ] `file_id` INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE
    - [ ] `local_id` TEXT NOT NULL - Task ID within file
    - [ ] `full_id` TEXT UNIQUE NOT NULL - file_id#local_id
    - [ ] `title` TEXT NOT NULL
    - [ ] `status` TEXT NOT NULL - open, done, waived, blocked
    - [ ] `depth` INTEGER NOT NULL - Nesting level (0-2)
    - [ ] `parent_id` INTEGER REFERENCES tasks(id) ON DELETE CASCADE
    - [ ] `order_index` INTEGER NOT NULL - Position among siblings
    - [ ] `owner` TEXT
    - [ ] `estimate` TEXT
    - [ ] `body` TEXT - Extended description
    - [ ] `metadata` TEXT - JSON blob for TaskMetadata
  - [ ] Indexes:
    - [ ] INDEX on file_id
    - [ ] UNIQUE on full_id
    - [ ] INDEX on status
    - [ ] INDEX on (file_id, order_index) - For ordered retrieval
    - [ ] INDEX on parent_id - For hierarchy queries
- [ ] **Design `dependencies` table**
  - [ ] Columns:
    - [ ] `id` INTEGER PRIMARY KEY
    - [ ] `from_task_id` INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE
    - [ ] `to_task_id` INTEGER REFERENCES tasks(id) ON DELETE CASCADE
    - [ ] `kind` TEXT NOT NULL - hierarchy, explicit_id, explicit_path, directory
    - [ ] `raw_ref` TEXT - Original reference string
  - [ ] Indexes:
    - [ ] INDEX on from_task_id
    - [ ] INDEX on to_task_id
    - [ ] INDEX on kind
  - [ ] Note: to_task_id can be NULL for unresolved references
- [ ] **Design `dependency_closure` table** (transitive closure)
  - [ ] Columns:
    - [ ] `ancestor_id` INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE
    - [ ] `descendant_id` INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE
    - [ ] `depth` INTEGER NOT NULL - Distance (1 = direct, 2+ = indirect)
  - [ ] Indexes:
    - [ ] PRIMARY KEY (ancestor_id, descendant_id)
    - [ ] INDEX on descendant_id (for finding all ancestors)
  - [ ] Used for fast "is A an ancestor of B?" queries
- [ ] **Design `labels` table**
  - [ ] Columns:
    - [ ] `id` INTEGER PRIMARY KEY
    - [ ] `name` TEXT UNIQUE NOT NULL - Normalized label name
  - [ ] Indexes:
    - [ ] UNIQUE on name
- [ ] **Design `task_labels` junction table**
  - [ ] Columns:
    - [ ] `task_id` INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE
    - [ ] `label_id` INTEGER NOT NULL REFERENCES labels(id) ON DELETE CASCADE
  - [ ] Indexes:
    - [ ] PRIMARY KEY (task_id, label_id)
    - [ ] INDEX on label_id (for label queries)
- [ ] **Design `file_labels` junction table**
  - [ ] Columns:
    - [ ] `file_id` INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE
    - [ ] `label_id` INTEGER NOT NULL REFERENCES labels(id) ON DELETE CASCADE
  - [ ] Indexes:
    - [ ] PRIMARY KEY (file_id, label_id)
    - [ ] INDEX on label_id
- [ ] **Design `metadata` table** (schema version, stats)
  - [ ] Columns:
    - [ ] `key` TEXT PRIMARY KEY
    - [ ] `value` TEXT
  - [ ] Store:
    - [ ] `schema_version` - Current version (for migrations)
    - [ ] `project_root` - Root path
    - [ ] `last_indexed` - Timestamp
    - [ ] `total_files` - Count
    - [ ] `total_tasks` - Count
- [ ] **Design FTS5 virtual table for search**
  - [ ] Table: `tasks_fts`
  - [ ] Columns:
    - [ ] `full_id` - Join key
    - [ ] `title` - Searchable
    - [ ] `body` - Searchable
  - [ ] Configuration:
    - [ ] Tokenizer: unicode61
    - [ ] Content table: tasks
    - [ ] BM25 ranking
- [ ] **Document schema**
  - [ ] Create SQL schema file: `lash-db/schema.sql`
  - [ ] Add ER diagram to docs
  - [ ] Document all indexes and their purpose
  - [ ] Document foreign key cascades
  - [ ] Document JSON blob structures

**Priority:** CRITICAL
**Estimate:** 1 day
**Dependencies:** tasks.core-data-model#6
**Success Criteria:** Schema fully documented; SQL DDL ready

---

### 2. Implement Schema Creation

- [ ] **Set up SQLite integration**
  - [ ] Add `rusqlite` dependency to `lash-db`
  - [ ] Add `rusqlite` with features: `bundled`, `blob`, `time`
  - [ ] Create `lash-db/src/connection.rs` for connection management
- [ ] **Create database initialization**
  - [ ] Function: `init_database(path: &Path) -> Result<Connection>`
  - [ ] Create database file if doesn't exist
  - [ ] Run schema DDL
  - [ ] Set PRAGMAs:
    - [ ] `PRAGMA foreign_keys = ON` - Enforce FK constraints
    - [ ] `PRAGMA journal_mode = WAL` - Better concurrency
    - [ ] `PRAGMA synchronous = NORMAL` - Balance safety/speed
    - [ ] `PRAGMA temp_store = MEMORY` - Faster temp operations
  - [ ] Initialize metadata table with schema version
- [ ] **Implement schema migrations**
  - [ ] Create `lash-db/src/migrations.rs`
  - [ ] Define `Migration` trait
  - [ ] Track current schema version in metadata table
  - [ ] Function: `run_migrations(conn: &Connection) -> Result<()>`
  - [ ] Apply migrations in order
  - [ ] Record successful migrations
  - [ ] For v1: Only initial schema (no migrations yet)
- [ ] **Create schema DDL**
  - [ ] Write CREATE TABLE statements for all tables
  - [ ] Create all indexes
  - [ ] Create FTS5 virtual table with triggers
  - [ ] Create triggers for FTS5 updates:
    - [ ] INSERT trigger: Add to FTS index
    - [ ] UPDATE trigger: Update FTS index
    - [ ] DELETE trigger: Remove from FTS index
- [ ] **Implement schema version tracking**
  - [ ] Store version in metadata table: `schema_version = 1`
  - [ ] Function: `get_schema_version(conn: &Connection) -> Result<i32>`
  - [ ] Function: `set_schema_version(conn: &Connection, version: i32) -> Result<()>`
  - [ ] Check version on open, run migrations if needed
- [ ] **Create connection pool**
  - [ ] Add `r2d2` and `r2d2_sqlite` dependencies
  - [ ] Create `lash-db/src/pool.rs`
  - [ ] Function: `create_pool(db_path: &Path) -> Result<Pool>`
  - [ ] Configure pool size (default: 4 connections)
  - [ ] Provide `get_connection()` method
- [ ] **Write tests**
  - [ ] Test database initialization (creates file, runs DDL)
  - [ ] Test PRAGMA settings
  - [ ] Test schema version tracking
  - [ ] Test connection pool
  - [ ] Test foreign key enforcement
  - [ ] Test FTS triggers
  - [ ] 15+ tests

**Priority:** CRITICAL
**Estimate:** 1 day
**Dependencies:** Task #1
**Success Criteria:** Database initializes correctly; schema created; FKs enforced

---

### 3. Implement File Repository

- [ ] **Create `FileRepository` in `lash-db/src/repository/files.rs`**
  - [ ] Struct holds database connection
  - [ ] Methods for CRUD operations
- [ ] **Implement insert file**
  - [ ] Method: `insert(&self, file: &TaskFile) -> Result<i64>`
  - [ ] INSERT INTO files
  - [ ] Serialize metadata to JSON
  - [ ] Return auto-generated file.id
  - [ ] Handle unique constraint violations (path, file_id)
- [ ] **Implement update file**
  - [ ] Method: `update(&self, file: &TaskFile) -> Result<()>`
  - [ ] UPDATE files WHERE path = ?
  - [ ] Update hash, mtime, status, metadata
  - [ ] Return error if file not found
- [ ] **Implement delete file**
  - [ ] Method: `delete(&self, path: &Path) -> Result<()>`
  - [ ] DELETE FROM files WHERE path = ?
  - [ ] Cascades to tasks, labels (via FK)
  - [ ] Return Ok even if not found (idempotent)
- [ ] **Implement query by path**
  - [ ] Method: `get_by_path(&self, path: &Path) -> Result<Option<FileRecord>>`
  - [ ] SELECT * FROM files WHERE path = ?
  - [ ] Deserialize JSON metadata
  - [ ] Return None if not found
- [ ] **Implement query by file_id**
  - [ ] Method: `get_by_file_id(&self, file_id: &str) -> Result<Option<FileRecord>>`
  - [ ] SELECT * FROM files WHERE file_id = ?
- [ ] **Implement query by label**
  - [ ] Method: `find_by_label(&self, label: &str) -> Result<Vec<FileRecord>>`
  - [ ] JOIN file_labels and labels
  - [ ] Return all files with that label
- [ ] **Implement list all files**
  - [ ] Method: `list_all(&self) -> Result<Vec<FileRecord>>`
  - [ ] SELECT * FROM files ORDER BY path
  - [ ] Used for index verification
- [ ] **Implement bulk operations**
  - [ ] Method: `insert_batch(&self, files: &[TaskFile]) -> Result<()>`
  - [ ] Use transaction
  - [ ] Prepare statement once, execute multiple times
  - [ ] Much faster than individual inserts
- [ ] **Implement change detection**
  - [ ] Method: `get_changed_files(&self, root: &Path) -> Result<Vec<PathBuf>>`
  - [ ] Compare filesystem mtime/hash with DB
  - [ ] Return list of paths that need re-indexing
- [ ] **Define `FileRecord` struct**
  - [ ] Represents row from files table
  - [ ] Includes all columns plus deserialized metadata
  - [ ] Can convert to/from TaskFile
- [ ] **Write tests**
  - [ ] Insert file
  - [ ] Update file
  - [ ] Delete file
  - [ ] Query by path (found and not found)
  - [ ] Query by file_id
  - [ ] Query by label
  - [ ] List all
  - [ ] Bulk insert (performance)
  - [ ] Change detection
  - [ ] 20+ tests

**Priority:** HIGH
**Estimate:** 1.5 days
**Dependencies:** Task #2
**Success Criteria:** Can CRUD file records; bulk operations work

---

### 4. Implement Task Repository

- [ ] **Create `TaskRepository` in `lash-db/src/repository/tasks.rs`**
  - [ ] Struct holds database connection
  - [ ] Methods for CRUD and queries
- [ ] **Implement insert task**
  - [ ] Method: `insert(&self, task: &Task, file_db_id: i64) -> Result<i64>`
  - [ ] INSERT INTO tasks
  - [ ] Serialize metadata to JSON
  - [ ] Return task.id
- [ ] **Implement update task**
  - [ ] Method: `update(&self, task: &Task) -> Result<()>`
  - [ ] UPDATE tasks WHERE full_id = ?
  - [ ] Update status, title, metadata, etc.
- [ ] **Implement delete task**
  - [ ] Method: `delete(&self, full_id: &str) -> Result<()>`
  - [ ] DELETE FROM tasks WHERE full_id = ?
  - [ ] Cascades to dependencies
- [ ] **Implement query by full_id**
  - [ ] Method: `get_by_full_id(&self, full_id: &str) -> Result<Option<TaskRecord>>`
  - [ ] SELECT * FROM tasks WHERE full_id = ?
- [ ] **Implement query by file**
  - [ ] Method: `get_by_file(&self, file_id: i64) -> Result<Vec<TaskRecord>>`
  - [ ] SELECT * FROM tasks WHERE file_id = ? ORDER BY order_index
  - [ ] Returns tasks in document order
- [ ] **Implement query by status**
  - [ ] Method: `find_by_status(&self, status: TaskStatus) -> Result<Vec<TaskRecord>>`
  - [ ] SELECT * FROM tasks WHERE status = ?
  - [ ] Useful for "list all open tasks"
- [ ] **Implement query by label**
  - [ ] Method: `find_by_label(&self, label: &str) -> Result<Vec<TaskRecord>>`
  - [ ] JOIN task_labels and labels
  - [ ] Return all tasks with that label
- [ ] **Implement hierarchical queries**
  - [ ] Method: `get_children(&self, task_id: i64) -> Result<Vec<TaskRecord>>`
  - [ ] SELECT * FROM tasks WHERE parent_id = ? ORDER BY order_index
  - [ ] Direct children only
  - [ ] Method: `get_descendants(&self, task_id: i64) -> Result<Vec<TaskRecord>>`
  - [ ] Recursive query or use dependency_closure table
  - [ ] All descendants (children, grandchildren, etc.)
  - [ ] Method: `get_ancestors(&self, task_id: i64) -> Result<Vec<TaskRecord>>`
  - [ ] Walk up parent_id chain
  - [ ] Or use dependency_closure table
- [ ] **Implement filtering and sorting**
  - [ ] Method: `find(&self, filter: TaskFilter) -> Result<Vec<TaskRecord>>`
  - [ ] `TaskFilter` struct with optional fields:
    - [ ] `status: Option<TaskStatus>`
    - [ ] `labels: Vec<String>`
    - [ ] `owner: Option<String>`
    - [ ] `file_path: Option<String>`
    - [ ] `blocked: Option<bool>`
  - [ ] Build WHERE clause dynamically
  - [ ] Support multiple filter criteria (AND)
  - [ ] Sort options: by file, by status, by created date
- [ ] **Implement bulk operations**
  - [ ] Method: `insert_batch(&self, tasks: &[Task], file_id: i64) -> Result<()>`
  - [ ] Use transaction
  - [ ] Prepared statement
- [ ] **Define `TaskRecord` struct**
  - [ ] Represents row from tasks table
  - [ ] Includes all columns plus deserialized metadata
  - [ ] Can convert to/from Task
- [ ] **Write tests**
  - [ ] CRUD operations
  - [ ] Query by full_id
  - [ ] Query by file
  - [ ] Query by status
  - [ ] Query by label
  - [ ] Hierarchical queries (children, descendants, ancestors)
  - [ ] Filtering with multiple criteria
  - [ ] Sorting
  - [ ] Bulk insert
  - [ ] 30+ tests

**Priority:** HIGH
**Estimate:** 2 days
**Dependencies:** Task #2
**Success Criteria:** Can CRUD task records efficiently; hierarchical queries work

---

### 5. Implement Dependency Repository

- [ ] **Create `DependencyRepository` in `lash-db/src/repository/dependencies.rs`**
- [ ] **Implement insert dependency**
  - [ ] Method: `insert(&self, dep: &Dependency) -> Result<i64>`
  - [ ] INSERT INTO dependencies
  - [ ] Return dependency id
- [ ] **Implement delete dependency**
  - [ ] Method: `delete(&self, from_task_id: i64, to_task_id: i64) -> Result<()>`
  - [ ] DELETE FROM dependencies WHERE ...
- [ ] **Implement query dependencies (outgoing)**
  - [ ] Method: `get_dependencies(&self, task_id: i64) -> Result<Vec<DependencyRecord>>`
  - [ ] SELECT * FROM dependencies WHERE from_task_id = ?
  - [ ] Returns tasks that `task_id` depends ON
- [ ] **Implement query dependents (incoming)**
  - [ ] Method: `get_dependents(&self, task_id: i64) -> Result<Vec<DependencyRecord>>`
  - [ ] SELECT * FROM dependencies WHERE to_task_id = ?
  - [ ] Returns tasks that depend on `task_id`
- [ ] **Implement transitive closure queries**
  - [ ] Method: `get_all_dependencies(&self, task_id: i64) -> Result<Vec<i64>>`
  - [ ] SELECT descendant_id FROM dependency_closure WHERE ancestor_id = ?
  - [ ] All transitive dependencies
  - [ ] Method: `get_all_dependents(&self, task_id: i64) -> Result<Vec<i64>>`
  - [ ] SELECT ancestor_id FROM dependency_closure WHERE descendant_id = ?
  - [ ] All transitive dependents
- [ ] **Implement cycle detection query**
  - [ ] Method: `is_cyclic(&self, from: i64, to: i64) -> Result<bool>`
  - [ ] Check: Would adding edge (from → to) create cycle?
  - [ ] Query dependency_closure: is `from` already a descendant of `to`?
  - [ ] Fast O(1) lookup with closure table
- [ ] **Implement build transitive closure**
  - [ ] Method: `rebuild_closure(&self) -> Result<()>`
  - [ ] DELETE FROM dependency_closure
  - [ ] Build closure from dependencies table:
    - [ ] Insert direct edges (depth = 1)
    - [ ] Iteratively add transitive edges
    - [ ] Or use recursive CTE in SQLite
  - [ ] Called after bulk dependency changes
- [ ] **Implement incremental closure update**
  - [ ] Method: `update_closure_for_edge(&self, from: i64, to: i64) -> Result<()>`
  - [ ] Add edge (from → to) and all implied transitive edges
  - [ ] Algorithm:
    - [ ] For each ancestor A of `from`: add edge (A → to)
    - [ ] For each descendant D of `to`: add edge (from → D)
    - [ ] For each (A, D) pair: add edge (A → D)
  - [ ] More efficient than full rebuild for small changes
- [ ] **Implement bulk operations**
  - [ ] Method: `insert_batch(&self, deps: &[Dependency]) -> Result<()>`
  - [ ] Use transaction
  - [ ] Rebuild closure at end (more efficient than incremental)
- [ ] **Define `DependencyRecord` struct**
  - [ ] Represents row from dependencies table
  - [ ] Can convert to/from Dependency
- [ ] **Write tests**
  - [ ] Insert/delete dependency
  - [ ] Query dependencies (outgoing)
  - [ ] Query dependents (incoming)
  - [ ] Transitive closure queries
  - [ ] Cycle detection (various patterns)
  - [ ] Rebuild closure
  - [ ] Incremental closure update
  - [ ] Bulk insert
  - [ ] 25+ tests

**Priority:** HIGH
**Estimate:** 1.5 days
**Dependencies:** Task #2
**Success Criteria:** Can manage dependency graph in DB; cycle detection works

---

### 6. Implement Label Repository

- [ ] **Create `LabelRepository` in `lash-db/src/repository/labels.rs`**
- [ ] **Implement get or create label**
  - [ ] Method: `get_or_create(&self, name: &str) -> Result<i64>`
  - [ ] Normalize label name
  - [ ] SELECT id FROM labels WHERE name = ?
  - [ ] If not found: INSERT and return new id
  - [ ] If found: return existing id
  - [ ] Handle race conditions (UNIQUE constraint)
- [ ] **Implement associate task with label**
  - [ ] Method: `add_task_label(&self, task_id: i64, label_id: i64) -> Result<()>`
  - [ ] INSERT INTO task_labels
  - [ ] Ignore if already exists (INSERT OR IGNORE)
- [ ] **Implement associate file with label**
  - [ ] Method: `add_file_label(&self, file_id: i64, label_id: i64) -> Result<()>`
  - [ ] INSERT INTO file_labels
- [ ] **Implement remove associations**
  - [ ] Method: `remove_task_label(&self, task_id: i64, label_id: i64) -> Result<()>`
  - [ ] DELETE FROM task_labels WHERE ...
  - [ ] Method: `remove_file_label(&self, file_id: i64, label_id: i64) -> Result<()>`
- [ ] **Implement query labels for task**
  - [ ] Method: `get_task_labels(&self, task_id: i64) -> Result<Vec<String>>`
  - [ ] JOIN task_labels and labels
  - [ ] Return label names
- [ ] **Implement query labels for file**
  - [ ] Method: `get_file_labels(&self, file_id: i64) -> Result<Vec<String>>`
- [ ] **Implement query all labels**
  - [ ] Method: `list_all(&self) -> Result<Vec<LabelRecord>>`
  - [ ] SELECT * FROM labels ORDER BY name
  - [ ] Include counts (how many tasks/files per label)
  - [ ] Useful for autocomplete
- [ ] **Implement batch label operations**
  - [ ] Method: `set_task_labels(&self, task_id: i64, labels: &[String]) -> Result<()>`
  - [ ] Delete existing associations
  - [ ] Get or create label IDs
  - [ ] Insert new associations
  - [ ] Use transaction
  - [ ] Method: `set_file_labels(&self, file_id: i64, labels: &[String]) -> Result<()>`
- [ ] **Implement label statistics**
  - [ ] Method: `get_label_stats(&self) -> Result<Vec<LabelStats>>`
  - [ ] For each label: count of tasks, count of files
  - [ ] Used for reporting popular labels
- [ ] **Define `LabelRecord` and `LabelStats` structs**
- [ ] **Write tests**
  - [ ] Get or create (new and existing)
  - [ ] Add/remove task label
  - [ ] Add/remove file label
  - [ ] Query labels for task/file
  - [ ] List all labels
  - [ ] Batch set labels
  - [ ] Label statistics
  - [ ] 15+ tests

**Priority:** MEDIUM
**Estimate:** 1 day
**Dependencies:** Task #2
**Success Criteria:** Can manage labels efficiently; batch operations work

---

## Summary

### Total Estimate
**7-9 days** total for SQLite schema implementation

### Completion Criteria
- [ ] All tasks above completed
- [ ] Database schema created and documented
- [ ] All repositories implemented (File, Task, Dependency, Label)
- [ ] Transitive closure for fast dependency queries
- [ ] FTS5 for full-text search
- [ ] Comprehensive tests for all CRUD operations
- [ ] Performance acceptable for 1000+ task projects

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
