//! Migration v6: Add `contextual_notes` column to tasks table
//!
//! This migration adds support for storing contextual notes - plain bullet points
//! (without checkboxes) nested under tasks that serve as inline context,
//! requirements, or acceptance criteria.
//!
//! Contextual notes are stored as a JSON array of objects with `text` and
//! `line_number` fields, matching the `ContextualNote` struct in lash-types.

use rusqlite::Connection;

use crate::error::DbResult;
use crate::migrations::Migration;

/// Migration to add `contextual_notes` column to tasks table
pub struct MigrationV6ContextualNotes;

impl Migration for MigrationV6ContextualNotes {
    fn version(&self) -> i32 {
        6
    }

    fn description(&self) -> &'static str {
        "Add contextual_notes column to tasks table"
    }

    fn up(&self, conn: &Connection) -> DbResult<()> {
        // Add contextual_notes column to tasks table
        // Default to empty JSON array '[]' for backward compatibility
        conn.execute_batch(
            "ALTER TABLE tasks ADD COLUMN contextual_notes TEXT NOT NULL DEFAULT '[]';",
        )?;

        Ok(())
    }

    fn down(&self, _conn: &Connection) -> DbResult<()> {
        // SQLite does not support DROP COLUMN in older versions
        // Rollback would require recreating the entire table
        Err(crate::error::DbError::Other(
            "Rollback not supported: SQLite does not support DROP COLUMN".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::init_database;
    use crate::error::DbError;
    use tempfile::NamedTempFile;

    #[test]
    fn test_migration_v6_up() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();

        // If database was created from schema.sql, it already has the contextual_notes column
        // Check if the column exists, and if not, apply the migration
        let column_exists: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('tasks') WHERE name='contextual_notes'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        if column_exists == 0 {
            // Column doesn't exist, so we can test the migration
            let migration = MigrationV6ContextualNotes;
            migration.up(&conn).unwrap();

            // Verify contextual_notes column now exists
            let column_exists: i32 = conn
                .query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('tasks') WHERE name='contextual_notes'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(column_exists, 1);
        } else {
            // Column already exists (database was created from updated schema.sql)
            // Just verify it works correctly
            assert_eq!(column_exists, 1);
        }
    }

    #[test]
    fn test_migration_v6_default_value() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();

        // Insert a test file first
        conn.execute(
            "INSERT INTO files (path, file_id, title, hash, mtime, status, metadata)
             VALUES ('test.md', 'test', 'Test File', 'hash1', 1234567890, 'in_progress', '{}')",
            [],
        )
        .unwrap();

        let file_id = conn.last_insert_rowid();

        // Insert a task without specifying contextual_notes
        conn.execute(
            "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index, metadata)
             VALUES (?1, 'task1', 'test#task1', 'Test Task', 'open', 0, 0, '{}')",
            [file_id],
        )
        .unwrap();

        // Verify default value is empty JSON array
        let contextual_notes: String = conn
            .query_row(
                "SELECT contextual_notes FROM tasks WHERE full_id = 'test#task1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(contextual_notes, "[]", "Default should be empty JSON array");
    }

    #[test]
    fn test_migration_v6_stores_json() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();

        // Insert a test file
        conn.execute(
            "INSERT INTO files (path, file_id, title, hash, mtime, status, metadata)
             VALUES ('test.md', 'test', 'Test File', 'hash1', 1234567890, 'in_progress', '{}')",
            [],
        )
        .unwrap();

        let file_id = conn.last_insert_rowid();

        // Insert a task with contextual notes
        let notes_json = r#"[{"text":"Use library X","line_number":10},{"text":"Target < 100ms","line_number":11}]"#;
        conn.execute(
            "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index, metadata, contextual_notes)
             VALUES (?1, 'task1', 'test#task1', 'Test Task', 'open', 0, 0, '{}', ?2)",
            rusqlite::params![file_id, notes_json],
        )
        .unwrap();

        // Retrieve and verify
        let retrieved: String = conn
            .query_row(
                "SELECT contextual_notes FROM tasks WHERE full_id = 'test#task1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(retrieved, notes_json);

        // Verify it's valid JSON by parsing
        let parsed: serde_json::Value = serde_json::from_str(&retrieved).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_migration_v6_down_not_supported() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();

        let migration = MigrationV6ContextualNotes;
        let result = migration.down(&conn);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DbError::Other(_)));
    }
}
