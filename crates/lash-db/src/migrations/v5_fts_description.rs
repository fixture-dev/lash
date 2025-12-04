//! Migration v5: Add description field to FTS5 index
//!
//! This migration enhances the FTS5 search index to include file description
//! content, making descriptions searchable. Descriptions are weighted higher
//! than task body content but lower than titles.

use rusqlite::Connection;

use crate::error::DbResult;
use crate::migrations::Migration;

/// Migration to add description to FTS5 index
pub struct MigrationV5FtsDescription;

impl Migration for MigrationV5FtsDescription {
    fn version(&self) -> i32 {
        5
    }

    fn description(&self) -> &'static str {
        "Add description field to FTS5 search index"
    }

    #[allow(clippy::too_many_lines)] // Migration SQL is verbose but clear
    fn up(&self, conn: &Connection) -> DbResult<()> {
        // Drop old FTS table and triggers
        conn.execute_batch(
            "
            DROP TRIGGER IF EXISTS tasks_ai;
            DROP TRIGGER IF EXISTS tasks_au;
            DROP TRIGGER IF EXISTS tasks_ad;
            DROP TRIGGER IF EXISTS task_labels_ai;
            DROP TRIGGER IF EXISTS task_labels_ad;
            DROP TABLE IF EXISTS tasks_fts;
            ",
        )?;

        // Create new FTS5 table with description field
        conn.execute_batch(
            "
            CREATE VIRTUAL TABLE tasks_fts USING fts5(
                full_id UNINDEXED,
                title,
                body,
                labels,
                file_path,
                file_description,
                tokenize='unicode61 remove_diacritics 2'
            );
            ",
        )?;

        // Create triggers to maintain FTS index
        // Column weights for BM25: title (3.0), body (1.0), labels (2.0), file_path (0.5), file_description (1.5)
        conn.execute_batch(
            "
            CREATE TRIGGER tasks_ai AFTER INSERT ON tasks BEGIN
                INSERT INTO tasks_fts(rowid, full_id, title, body, labels, file_path, file_description)
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
                    COALESCE(f.description, '')
                FROM files f
                WHERE f.id = new.file_id;
            END;

            CREATE TRIGGER tasks_au AFTER UPDATE ON tasks BEGIN
                DELETE FROM tasks_fts WHERE rowid = old.id;
                INSERT INTO tasks_fts(rowid, full_id, title, body, labels, file_path, file_description)
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
                    COALESCE(f.description, '')
                FROM files f
                WHERE f.id = new.file_id;
            END;

            CREATE TRIGGER tasks_ad AFTER DELETE ON tasks BEGIN
                DELETE FROM tasks_fts WHERE rowid = old.id;
            END;

            CREATE TRIGGER task_labels_ai AFTER INSERT ON task_labels BEGIN
                DELETE FROM tasks_fts WHERE rowid = new.task_id;
                INSERT INTO tasks_fts(rowid, full_id, title, body, labels, file_path, file_description)
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
                    COALESCE(f.description, '')
                FROM tasks t
                JOIN files f ON f.id = t.file_id
                WHERE t.id = new.task_id;
            END;

            CREATE TRIGGER task_labels_ad AFTER DELETE ON task_labels BEGIN
                DELETE FROM tasks_fts WHERE rowid = old.task_id;
                INSERT INTO tasks_fts(rowid, full_id, title, body, labels, file_path, file_description)
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
                    COALESCE(f.description, '')
                FROM tasks t
                JOIN files f ON f.id = t.file_id
                WHERE t.id = old.task_id;
            END;
            ",
        )?;

        // Repopulate FTS index from existing tasks
        conn.execute_batch(
            "
            INSERT INTO tasks_fts(rowid, full_id, title, body, labels, file_path, file_description)
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
                COALESCE(f.description, '')
            FROM tasks t
            JOIN files f ON f.id = t.file_id;
            ",
        )?;

        Ok(())
    }

    fn down(&self, _conn: &Connection) -> DbResult<()> {
        // Rollback is not supported for this migration
        // Would require recreating the old FTS schema
        Err(crate::error::DbError::Other(
            "Rollback not supported for FTS migration".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::init_database;

    use tempfile::NamedTempFile;

    #[test]
    fn test_migration_v5_up() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();

        // Run migration
        let migration = MigrationV5FtsDescription;
        migration.up(&conn).unwrap();

        // Verify new FTS table structure
        let column_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('tasks_fts')",
                [],
                |row| row.get(0),
            )
            .unwrap();

        // Should have 6 columns: full_id, title, body, labels, file_path, file_description
        assert_eq!(column_count, 6);
    }

    #[test]
    fn test_migration_v5_searches_description() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();

        // Run migration
        let migration = MigrationV5FtsDescription;
        migration.up(&conn).unwrap();

        // Insert a test file with description
        conn.execute(
            "INSERT INTO files (path, file_id, title, description, hash, mtime, status, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            (
                "test.md",
                "test",
                "Test File",
                "This is a test description with searchable content",
                "hash1",
                1_234_567_890_i64,
                "in_progress",
                "{}",
            ),
        )
        .unwrap();

        let file_id = conn.last_insert_rowid();

        // Insert a task
        conn.execute(
            "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index, body, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, ?6, '{}')",
            (
                file_id,
                "task1",
                "test#task1",
                "Test Task",
                "open",
                "Task body text",
            ),
        )
        .unwrap();

        // Search for description content
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks_fts WHERE tasks_fts MATCH 'searchable'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        // Should find the task because it includes the file description
        assert_eq!(count, 1);
    }
}
