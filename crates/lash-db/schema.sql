-- Lash SQLite Schema v1
--
-- This is the acceleration layer for Lash. Markdown files are the source of truth;
-- this database is fully reconstructible from them.
--
-- Design principles:
-- - WAL mode for better concurrency
-- - Foreign keys ON for referential integrity
-- - Indexes on all query paths
-- - Transitive closure table for fast dependency queries
-- - FTS5 for full-text search

-- ============================================================================
-- Metadata table (schema version and statistics)
-- ============================================================================

CREATE TABLE metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Initialize schema version
INSERT INTO metadata (key, value) VALUES ('schema_version', '9');

-- ============================================================================
-- Files table (task files from the project)
-- ============================================================================

CREATE TABLE files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Path relative to project root
    path TEXT UNIQUE NOT NULL,

    -- File identifier (from @id annotation or synthesized from path)
    file_id TEXT UNIQUE NOT NULL,

    -- Title from first H1 heading
    title TEXT NOT NULL,

    -- Description text (multi-paragraph text after metadata, before tasks)
    description TEXT NOT NULL DEFAULT '',

    -- blake3 content hash for change detection
    hash TEXT NOT NULL,

    -- Unix timestamp of last modification
    mtime INTEGER NOT NULL,

    -- Computed overall status (complete, in_progress, blocked, empty)
    status TEXT CHECK(status IN ('complete', 'in_progress', 'blocked', 'empty')),

    -- FileMetadata as JSON blob (labels, owner, created, etc.)
    metadata TEXT NOT NULL DEFAULT '{}',

    -- When this file was indexed into the database
    indexed_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Indexes for files table
CREATE INDEX idx_files_status ON files(status);
CREATE INDEX idx_files_hash ON files(hash);
CREATE INDEX idx_files_mtime ON files(mtime);

-- ============================================================================
-- Tasks table (individual tasks within files)
-- ============================================================================

CREATE TABLE tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Reference to parent file
    file_id INTEGER NOT NULL,

    -- Task ID within the file (from @id or synthesized)
    local_id TEXT NOT NULL,

    -- Full unique identifier: file_id#local_id
    full_id TEXT UNIQUE NOT NULL,

    -- Task title/description
    title TEXT NOT NULL,

    -- Current status (open, in-progress, done, waived, blocked)
    status TEXT NOT NULL CHECK(status IN ('open', 'in-progress', 'done', 'waived', 'blocked')),

    -- Nesting level (0 = top-level, max typically 2-3)
    depth INTEGER NOT NULL CHECK(depth >= 0),

    -- Parent task (for hierarchical dependencies)
    parent_id INTEGER,

    -- Position among siblings (for ordering)
    order_index INTEGER NOT NULL,

    -- Optional owner
    owner TEXT,

    -- Optional estimate
    estimate TEXT,

    -- Extended description/notes
    body TEXT,

    -- TaskMetadata as JSON blob (labels, dependencies, etc.)
    metadata TEXT NOT NULL DEFAULT '{}',

    -- Contextual notes as JSON array (plain bullet points nested under the task)
    contextual_notes TEXT NOT NULL DEFAULT '[]',

    FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_id) REFERENCES tasks(id) ON DELETE CASCADE
);

-- Indexes for tasks table
CREATE INDEX idx_tasks_file_id ON tasks(file_id);
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_parent_id ON tasks(parent_id);
CREATE INDEX idx_tasks_file_order ON tasks(file_id, order_index);
CREATE INDEX idx_tasks_owner ON tasks(owner);

-- ============================================================================
-- Dependencies table (explicit dependency edges)
-- ============================================================================

CREATE TABLE dependencies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Task that has the dependency (depends ON to_task_id)
    from_task_id INTEGER NOT NULL,

    -- Task that is depended upon (can be NULL for unresolved refs)
    to_task_id INTEGER,

    -- Kind of dependency (hierarchy, explicit_id, explicit_path, directory)
    kind TEXT NOT NULL CHECK(kind IN ('hierarchy', 'explicit_id', 'explicit_path', 'directory')),

    -- Original reference string (for diagnostics and error messages)
    raw_ref TEXT,

    FOREIGN KEY (from_task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (to_task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

-- Indexes for dependencies table
CREATE INDEX idx_dependencies_from ON dependencies(from_task_id);
CREATE INDEX idx_dependencies_to ON dependencies(to_task_id);
CREATE INDEX idx_dependencies_kind ON dependencies(kind);

-- ============================================================================
-- Dependency closure table (transitive closure for fast queries)
-- ============================================================================
--
-- This table stores all transitive dependencies for O(1) reachability queries.
-- It answers: "Is task A an ancestor/dependency of task B?"
--
-- Maintained via triggers or explicit rebuild after bulk changes.

CREATE TABLE dependency_closure (
    -- The ancestor task (task that is depended upon transitively)
    ancestor_id INTEGER NOT NULL,

    -- The descendant task (task that depends on ancestor transitively)
    descendant_id INTEGER NOT NULL,

    -- Distance in the graph (1 = direct, 2+ = indirect)
    depth INTEGER NOT NULL CHECK(depth > 0),

    PRIMARY KEY (ancestor_id, descendant_id),

    FOREIGN KEY (ancestor_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (descendant_id) REFERENCES tasks(id) ON DELETE CASCADE
);

-- Index for reverse lookups (finding all ancestors of a task)
CREATE INDEX idx_closure_descendant ON dependency_closure(descendant_id);

-- ============================================================================
-- Labels table (unique labels across the system)
-- ============================================================================

CREATE TABLE labels (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Normalized label name (lowercase, trimmed)
    name TEXT UNIQUE NOT NULL
);

-- ============================================================================
-- Task-label junction table
-- ============================================================================

CREATE TABLE task_labels (
    task_id INTEGER NOT NULL,
    label_id INTEGER NOT NULL,

    PRIMARY KEY (task_id, label_id),

    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (label_id) REFERENCES labels(id) ON DELETE CASCADE
);

-- Index for querying all tasks with a given label
CREATE INDEX idx_task_labels_label ON task_labels(label_id);

-- ============================================================================
-- File-label junction table
-- ============================================================================

CREATE TABLE file_labels (
    file_id INTEGER NOT NULL,
    label_id INTEGER NOT NULL,

    PRIMARY KEY (file_id, label_id),

    FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE,
    FOREIGN KEY (label_id) REFERENCES labels(id) ON DELETE CASCADE
);

-- Index for querying all files with a given label
CREATE INDEX idx_file_labels_label ON file_labels(label_id);

-- ============================================================================
-- Doc Refs table (documentation references from @doc annotations)
-- ============================================================================

CREATE TABLE doc_refs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Source file (required - every doc ref belongs to a file)
    source_file_id INTEGER NOT NULL,

    -- Source task (NULL for file-level @doc annotations)
    source_task_id INTEGER NULL,

    -- Target document path (relative path to the doc)
    target_path TEXT NOT NULL,

    -- Optional fragment (e.g., section anchor)
    fragment TEXT NULL,

    FOREIGN KEY (source_file_id) REFERENCES files(id) ON DELETE CASCADE,
    FOREIGN KEY (source_task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

-- Index for finding all doc refs for a file
CREATE INDEX idx_doc_refs_source_file ON doc_refs(source_file_id);

-- Index for finding all doc refs for a task
CREATE INDEX idx_doc_refs_source_task ON doc_refs(source_task_id) WHERE source_task_id IS NOT NULL;

-- Index for reverse lookup (find all sources that reference a doc)
CREATE INDEX idx_doc_refs_target_path ON doc_refs(target_path);

-- ============================================================================
-- FTS5 virtual table for full-text search
-- ============================================================================
--
-- Column weights for relevance ranking (configured via bm25()):
-- - title: highest weight (most important)
-- - labels: medium-high weight
-- - file_description: medium weight (higher than task body, lower than title)
-- - body: standard weight
-- - contextual_notes: lower than body but higher than file_path
-- - file_path: lower weight

CREATE VIRTUAL TABLE tasks_fts USING fts5(
    full_id UNINDEXED,  -- Join key (not searchable)
    title,              -- Searchable title (highest weight)
    body,               -- Searchable body text
    labels,             -- Space-separated labels
    file_path,          -- File path for filename matching
    file_description,   -- File description text (searchable)
    contextual_notes,   -- Contextual notes text (searchable)
    tokenize='unicode61 remove_diacritics 2'
);

-- ============================================================================
-- FTS5 triggers (keep search index in sync with tasks table)
-- ============================================================================
--
-- Note: FTS5 triggers need to be manually maintained since we're joining
-- data from multiple tables (files and labels). Updates to labels or files
-- need to trigger re-indexing of related tasks.

-- Insert trigger: add new task to search index
CREATE TRIGGER tasks_ai AFTER INSERT ON tasks BEGIN
    INSERT INTO tasks_fts(rowid, full_id, title, body, labels, file_path, file_description, contextual_notes)
    SELECT
        new.id,
        new.full_id,
        new.title,
        COALESCE(new.body, ''),
        COALESCE((
            SELECT GROUP_CONCAT(l.name, ' ')
            FROM task_labels tl
            JOIN labels l ON l.id = tl.label_id
            WHERE tl.task_id = new.id
        ), ''),
        f.path,
        COALESCE(f.description, ''),
        COALESCE((
            SELECT GROUP_CONCAT(json_extract(value, '$.text'), ' ')
            FROM json_each(new.contextual_notes)
        ), '')
    FROM files f
    WHERE f.id = new.file_id;
END;

-- Update trigger: update search index when task changes
CREATE TRIGGER tasks_au AFTER UPDATE ON tasks BEGIN
    DELETE FROM tasks_fts WHERE rowid = old.id;
    INSERT INTO tasks_fts(rowid, full_id, title, body, labels, file_path, file_description, contextual_notes)
    SELECT
        new.id,
        new.full_id,
        new.title,
        COALESCE(new.body, ''),
        COALESCE((
            SELECT GROUP_CONCAT(l.name, ' ')
            FROM task_labels tl
            JOIN labels l ON l.id = tl.label_id
            WHERE tl.task_id = new.id
        ), ''),
        f.path,
        COALESCE(f.description, ''),
        COALESCE((
            SELECT GROUP_CONCAT(json_extract(value, '$.text'), ' ')
            FROM json_each(new.contextual_notes)
        ), '')
    FROM files f
    WHERE f.id = new.file_id;
END;

-- Delete trigger: remove from search index
CREATE TRIGGER tasks_ad AFTER DELETE ON tasks BEGIN
    DELETE FROM tasks_fts WHERE rowid = old.id;
END;

-- Trigger to update FTS when labels change
CREATE TRIGGER task_labels_ai AFTER INSERT ON task_labels BEGIN
    DELETE FROM tasks_fts WHERE rowid = new.task_id;
    INSERT INTO tasks_fts(rowid, full_id, title, body, labels, file_path, file_description, contextual_notes)
    SELECT
        t.id,
        t.full_id,
        t.title,
        COALESCE(t.body, ''),
        COALESCE((
            SELECT GROUP_CONCAT(l.name, ' ')
            FROM task_labels tl
            JOIN labels l ON l.id = tl.label_id
            WHERE tl.task_id = t.id
        ), ''),
        f.path,
        COALESCE(f.description, ''),
        COALESCE((
            SELECT GROUP_CONCAT(json_extract(value, '$.text'), ' ')
            FROM json_each(t.contextual_notes)
        ), '')
    FROM tasks t
    JOIN files f ON f.id = t.file_id
    WHERE t.id = new.task_id;
END;

CREATE TRIGGER task_labels_ad AFTER DELETE ON task_labels BEGIN
    DELETE FROM tasks_fts WHERE rowid = old.task_id;
    INSERT INTO tasks_fts(rowid, full_id, title, body, labels, file_path, file_description, contextual_notes)
    SELECT
        t.id,
        t.full_id,
        t.title,
        COALESCE(t.body, ''),
        COALESCE((
            SELECT GROUP_CONCAT(l.name, ' ')
            FROM task_labels tl
            JOIN labels l ON l.id = tl.label_id
            WHERE tl.task_id = t.id
        ), ''),
        f.path,
        COALESCE(f.description, ''),
        COALESCE((
            SELECT GROUP_CONCAT(json_extract(value, '$.text'), ' ')
            FROM json_each(t.contextual_notes)
        ), '')
    FROM tasks t
    JOIN files f ON f.id = t.file_id
    WHERE t.id = old.task_id;
END;

-- ============================================================================
-- ID migrations table (task IDs moved by a derivation-rule change)
-- ============================================================================

-- A derived task ID is a function of the derivation rules, and those rules can
-- change between releases. The re-derive that follows such a change is the only
-- moment both spellings of an ID exist at once, so it is the only moment the
-- old->new mapping can be recorded exactly rather than guessed at. Rows here
-- are pending work for `lash migrate-ids`, which rewrites references and clears
-- them.

CREATE TABLE id_migrations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Path of the file the task lives in, relative to project root
    file_path TEXT NOT NULL,

    -- The file's own id, the left half of a qualified task id
    file_id TEXT NOT NULL,

    -- The id stored before the derivation rules changed
    old_local_id TEXT NOT NULL,

    -- The id the current rules derive for the same task
    new_local_id TEXT NOT NULL,

    -- Task title, so the record is legible without re-reading the file
    title TEXT NOT NULL,

    UNIQUE(file_path, old_local_id)
);

CREATE INDEX idx_id_migrations_old ON id_migrations(file_id, old_local_id);
