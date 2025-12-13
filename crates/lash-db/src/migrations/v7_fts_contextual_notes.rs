//! Migration v7: Add `contextual_notes` field to FTS5 index
//!
//! This migration enhances the FTS5 search index to include contextual notes
//! content, making notes searchable. Contextual notes are weighted lower than
//! task body content but higher than file paths, as they provide context but
//! are secondary to the task description itself.

use rusqlite::Connection;

use crate::error::DbResult;
use crate::migrations::Migration;

/// Migration to add `contextual_notes` to FTS5 index
pub struct MigrationV7FtsContextualNotes;

impl Migration for MigrationV7FtsContextualNotes {
    fn version(&self) -> i32 {
        7
    }

    fn description(&self) -> &'static str {
        "Add contextual_notes field to FTS5 search index"
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

        // Create new FTS5 table with contextual_notes field
        conn.execute_batch(
            "
            CREATE VIRTUAL TABLE tasks_fts USING fts5(
                full_id UNINDEXED,
                title,
                body,
                labels,
                file_path,
                file_description,
                contextual_notes,
                tokenize='unicode61 remove_diacritics 2'
            );
            ",
        )?;

        // Create triggers to maintain FTS index
        // Column weights for BM25: title (3.0), body (1.0), labels (2.0),
        // file_path (0.5), file_description (1.5), contextual_notes (0.8)
        conn.execute_batch(
            "
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

            CREATE TRIGGER tasks_ad AFTER DELETE ON tasks BEGIN
                DELETE FROM tasks_fts WHERE rowid = old.id;
            END;

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
            ",
        )?;

        // Repopulate FTS index from existing tasks
        conn.execute_batch(
            "
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
    fn test_migration_v7_up() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();

        // Run migration
        let migration = MigrationV7FtsContextualNotes;
        migration.up(&conn).unwrap();

        // Verify new FTS table structure
        let column_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('tasks_fts')",
                [],
                |row| row.get(0),
            )
            .unwrap();

        // Should have 7 columns: full_id, title, body, labels, file_path, file_description, contextual_notes
        assert_eq!(column_count, 7);
    }

    #[test]
    fn test_migration_v7_searches_notes() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();

        // Run migration
        let migration = MigrationV7FtsContextualNotes;
        migration.up(&conn).unwrap();

        // Insert a test file
        conn.execute(
            "INSERT INTO files (path, file_id, title, description, hash, mtime, status, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            (
                "test.md",
                "test",
                "Test File",
                "File description",
                "hash1",
                1_234_567_890_i64,
                "in_progress",
                "{}",
            ),
        )
        .unwrap();

        let file_id = conn.last_insert_rowid();

        // Insert a task with contextual notes
        let notes_json = r#"[{"text":"Use library XYZ","line_number":10},{"text":"Target performance","line_number":11}]"#;
        conn.execute(
            "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index, body, metadata, contextual_notes)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, ?6, '{}', ?7)",
            (
                file_id,
                "task1",
                "test#task1",
                "Test Task",
                "open",
                "Task body text",
                notes_json,
            ),
        )
        .unwrap();

        // Search for content in notes
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks_fts WHERE tasks_fts MATCH 'library'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        // Should find the task because it includes "library" in contextual notes
        assert_eq!(count, 1);

        // Search for another note content
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks_fts WHERE tasks_fts MATCH 'performance'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        // Should find the task because it includes "performance" in contextual notes
        assert_eq!(count, 1);
    }

    #[test]
    fn test_migration_v7_notes_indexed_on_insert() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();

        // Run migration
        let migration = MigrationV7FtsContextualNotes;
        migration.up(&conn).unwrap();

        // Insert a test file
        conn.execute(
            "INSERT INTO files (path, file_id, title, hash, mtime, status, metadata)
             VALUES ('test.md', 'test', 'Test File', 'hash1', 1234567890, 'in_progress', '{}')",
            [],
        )
        .unwrap();

        let file_id = conn.last_insert_rowid();

        // Insert a task with contextual notes via trigger
        let notes_json = r#"[{"text":"Important criterion","line_number":5}]"#;
        conn.execute(
            "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index, metadata, contextual_notes)
             VALUES (?1, 'task1', 'test#task1', 'Test Task', 'open', 0, 0, '{}', ?2)",
            rusqlite::params![file_id, notes_json],
        )
        .unwrap();

        // Verify notes are searchable immediately after insert
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks_fts WHERE tasks_fts MATCH 'criterion'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_migration_v7_notes_updated_in_fts() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();

        // Run migration
        let migration = MigrationV7FtsContextualNotes;
        migration.up(&conn).unwrap();

        // Insert a test file
        conn.execute(
            "INSERT INTO files (path, file_id, title, hash, mtime, status, metadata)
             VALUES ('test.md', 'test', 'Test File', 'hash1', 1234567890, 'in_progress', '{}')",
            [],
        )
        .unwrap();

        let file_id = conn.last_insert_rowid();

        // Insert a task with initial notes
        let notes_json1 = r#"[{"text":"Initial note","line_number":5}]"#;
        conn.execute(
            "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index, metadata, contextual_notes)
             VALUES (?1, 'task1', 'test#task1', 'Test Task', 'open', 0, 0, '{}', ?2)",
            rusqlite::params![file_id, notes_json1],
        )
        .unwrap();

        // Verify initial note is searchable
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks_fts WHERE tasks_fts MATCH 'Initial'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        // Update task with new notes
        let notes_json2 = r#"[{"text":"Updated note with special keyword","line_number":5}]"#;
        conn.execute(
            "UPDATE tasks SET contextual_notes = ?1 WHERE full_id = 'test#task1'",
            [notes_json2],
        )
        .unwrap();

        // Verify old note is no longer found
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks_fts WHERE tasks_fts MATCH 'Initial'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);

        // Verify new note is now searchable
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks_fts WHERE tasks_fts MATCH 'special'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
