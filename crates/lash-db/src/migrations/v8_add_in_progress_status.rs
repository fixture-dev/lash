//! Migration v8: Add 'in-progress' status to tasks table CHECK constraint
//!
//! This migration updates the tasks table CHECK constraint to include the
//! 'in-progress' status, which was missing from the original schema. This
//! enables the `lash start` command to transition tasks to `InProgress` state.
//!
//! The migration recreates the tasks table with the updated constraint, preserving
//! all existing data.

use rusqlite::Connection;

use crate::error::DbResult;
use crate::migrations::Migration;

/// Migration to add 'in-progress' status to tasks table
pub struct MigrationV8AddInProgressStatus;

impl Migration for MigrationV8AddInProgressStatus {
    fn version(&self) -> i32 {
        8
    }

    fn description(&self) -> &'static str {
        "Add 'in-progress' status to tasks table CHECK constraint"
    }

    #[allow(clippy::too_many_lines)] // Migration SQL is verbose but clear
    fn up(&self, conn: &Connection) -> DbResult<()> {
        // SQLite doesn't support ALTER TABLE to modify CHECK constraints,
        // so we need to recreate the table with the new constraint.

        // Step 1: Create new tasks table with updated CHECK constraint
        conn.execute_batch(
            "
            CREATE TABLE tasks_new (
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

                -- Optional agent notes (JSON array of note objects)
                agent_notes TEXT NOT NULL DEFAULT '[]',

                -- Optional body content (additional task description)
                body TEXT,

                -- Contextual notes (JSON array of note objects with text and line_number)
                contextual_notes TEXT NOT NULL DEFAULT '[]',

                FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE,
                FOREIGN KEY (parent_id) REFERENCES tasks(id) ON DELETE CASCADE
            );
            ",
        )?;

        // Step 2: Copy all data from old table to new table
        conn.execute_batch(
            "
            INSERT INTO tasks_new (
                id, file_id, local_id, full_id, title, status, depth,
                parent_id, order_index, owner, estimate, agent_notes,
                body, contextual_notes
            )
            SELECT
                id, file_id, local_id, full_id, title, status, depth,
                parent_id, order_index, owner, estimate, agent_notes,
                body, contextual_notes
            FROM tasks;
            ",
        )?;

        // Step 3: Drop old table
        conn.execute_batch("DROP TABLE tasks;")?;

        // Step 4: Rename new table to tasks
        conn.execute_batch("ALTER TABLE tasks_new RENAME TO tasks;")?;

        // Step 5: Recreate all indexes on the tasks table
        conn.execute_batch(
            "
            CREATE INDEX idx_tasks_file_id ON tasks(file_id);
            CREATE INDEX idx_tasks_full_id ON tasks(full_id);
            CREATE INDEX idx_tasks_status ON tasks(status);
            CREATE INDEX idx_tasks_parent_id ON tasks(parent_id) WHERE parent_id IS NOT NULL;
            CREATE INDEX idx_tasks_owner ON tasks(owner) WHERE owner IS NOT NULL;
            ",
        )?;

        // Step 6: Recreate FTS triggers for the tasks table
        // These triggers keep the full-text search index in sync
        conn.execute_batch(
            "
            -- Drop existing triggers if they exist
            DROP TRIGGER IF EXISTS tasks_ai;
            DROP TRIGGER IF EXISTS tasks_au;
            DROP TRIGGER IF EXISTS tasks_ad;

            -- Create trigger for INSERT
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

            -- Create trigger for UPDATE
            CREATE TRIGGER tasks_au AFTER UPDATE ON tasks BEGIN
                UPDATE tasks_fts
                SET
                    full_id = new.full_id,
                    title = new.title,
                    body = COALESCE(new.body, ''),
                    labels = COALESCE((
                        SELECT GROUP_CONCAT(l.name, ' ')
                        FROM task_labels tl
                        JOIN labels l ON l.id = tl.label_id
                        WHERE tl.task_id = new.id
                    ), ''),
                    file_path = (SELECT path FROM files WHERE id = new.file_id),
                    file_description = COALESCE((SELECT description FROM files WHERE id = new.file_id), ''),
                    contextual_notes = COALESCE((
                        SELECT GROUP_CONCAT(json_extract(value, '$.text'), ' ')
                        FROM json_each(new.contextual_notes)
                    ), '')
                WHERE rowid = new.id;
            END;

            -- Create trigger for DELETE
            CREATE TRIGGER tasks_ad AFTER DELETE ON tasks BEGIN
                DELETE FROM tasks_fts WHERE rowid = old.id;
            END;
            ",
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// Create a mock v7 database schema (before in-progress was added)
    fn create_mock_v7_database() -> Connection {
        let conn = Connection::open_in_memory().unwrap();

        // Create files table (unchanged between v7 and v8)
        conn.execute_batch(
            "
            CREATE TABLE files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT UNIQUE NOT NULL,
                file_id TEXT UNIQUE NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                hash TEXT NOT NULL,
                mtime INTEGER NOT NULL,
                status TEXT NOT NULL,
                metadata TEXT NOT NULL DEFAULT '{}'
            );
            ",
        )
        .unwrap();

        // Create v7 tasks table (without 'in-progress' in CHECK constraint)
        conn.execute_batch(
            "
            CREATE TABLE tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_id INTEGER NOT NULL,
                local_id TEXT NOT NULL,
                full_id TEXT UNIQUE NOT NULL,
                title TEXT NOT NULL,
                status TEXT NOT NULL CHECK(status IN ('open', 'done', 'waived', 'blocked')),
                depth INTEGER NOT NULL CHECK(depth >= 0),
                parent_id INTEGER,
                order_index INTEGER NOT NULL,
                owner TEXT,
                estimate TEXT,
                agent_notes TEXT NOT NULL DEFAULT '[]',
                body TEXT,
                contextual_notes TEXT NOT NULL DEFAULT '[]',
                FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE,
                FOREIGN KEY (parent_id) REFERENCES tasks(id) ON DELETE CASCADE
            );

            CREATE INDEX idx_tasks_file_id ON tasks(file_id);
            CREATE INDEX idx_tasks_full_id ON tasks(full_id);
            CREATE INDEX idx_tasks_status ON tasks(status);
            CREATE INDEX idx_tasks_parent_id ON tasks(parent_id) WHERE parent_id IS NOT NULL;
            CREATE INDEX idx_tasks_owner ON tasks(owner) WHERE owner IS NOT NULL;

            -- Create labels table
            CREATE TABLE labels (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT UNIQUE NOT NULL
            );

            -- Create task_labels junction table
            CREATE TABLE task_labels (
                task_id INTEGER NOT NULL,
                label_id INTEGER NOT NULL,
                PRIMARY KEY (task_id, label_id),
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
                FOREIGN KEY (label_id) REFERENCES labels(id) ON DELETE CASCADE
            );

            -- Create FTS table
            CREATE VIRTUAL TABLE tasks_fts USING fts5(
                full_id,
                title,
                body,
                labels,
                file_path,
                file_description,
                contextual_notes,
                content='tasks',
                content_rowid='id'
            );
            ",
        )
        .unwrap();

        conn
    }

    #[test]
    fn test_migration_v8_adds_in_progress_status() {
        let conn = create_mock_v7_database();

        // Migration should succeed
        let migration = MigrationV8AddInProgressStatus;
        migration.up(&conn).unwrap();

        // Verify we can insert a task with in-progress status
        conn.execute(
            "INSERT INTO files (path, file_id, title, hash, mtime, status, metadata)
             VALUES ('test.md', 'test', 'Test', 'hash', 0, 'in_progress', '{}')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index)
             VALUES (1, 'task1', 'test#task1', 'Task 1', 'in-progress', 0, 0)",
            [],
        )
        .unwrap();

        // Verify the task was inserted
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks WHERE status = 'in-progress'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_migration_v8_preserves_existing_data() {
        let conn = create_mock_v7_database();

        // Insert test data before migration
        conn.execute(
            "INSERT INTO files (path, file_id, title, hash, mtime, status, metadata)
             VALUES ('test.md', 'test', 'Test', 'hash', 0, 'in_progress', '{}')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index)
             VALUES (1, 'task1', 'test#task1', 'Task 1', 'open', 0, 0)",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index)
             VALUES (1, 'task2', 'test#task2', 'Task 2', 'done', 0, 1)",
            [],
        )
        .unwrap();

        // Run migration
        let migration = MigrationV8AddInProgressStatus;
        migration.up(&conn).unwrap();

        // Verify all tasks still exist
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2);

        // Verify specific tasks
        let task1_title: String = conn
            .query_row(
                "SELECT title FROM tasks WHERE full_id = 'test#task1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(task1_title, "Task 1");

        let task2_status: String = conn
            .query_row(
                "SELECT status FROM tasks WHERE full_id = 'test#task2'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(task2_status, "done");
    }
}
