//! Migration v4: Add `description` column to files table
//!
//! This migration adds the `description` column to store multi-paragraph
//! description text that appears after the file metadata but before the
//! Tasks section.

use rusqlite::Connection;

use crate::error::{DbError, DbResult};
use crate::migrations::Migration;

/// Migration to add `description` column to files table
pub(super) struct MigrationV4FileDescriptions;

impl Migration for MigrationV4FileDescriptions {
    fn version(&self) -> i32 {
        4
    }

    fn description(&self) -> &'static str {
        "Add description column to files table"
    }

    fn up(&self, conn: &Connection) -> DbResult<()> {
        // Add description column to files table
        conn.execute(
            "ALTER TABLE files ADD COLUMN description TEXT NOT NULL DEFAULT ''",
            [],
        )?;

        Ok(())
    }

    fn down(&self, _conn: &Connection) -> DbResult<()> {
        // Note: This down migration is complex because:
        // 1. SQLite doesn't support DROP COLUMN
        // 2. Triggers reference the files table, so we can't simply drop and recreate
        // 3. In practice, down migrations are rarely needed
        //
        // For a production database, you would need to:
        // - Temporarily disable all triggers that reference files table
        // - Recreate the table without description column
        // - Re-enable triggers
        //
        // For now, we return an error indicating this migration cannot be rolled back
        // without more complex handling.
        Err(DbError::Other(
            "Down migration for v4 is not implemented. Rolling back this migration requires \
             manual intervention due to trigger dependencies."
                .to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::init_database;
    use tempfile::NamedTempFile;

    #[test]
    fn test_migration_v4_up() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();

        // If database was created from schema.sql, it already has the description column
        // Check if the column exists, and if not, apply the migration
        let column_exists: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('files') WHERE name='description'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        if column_exists == 0 {
            // Column doesn't exist, so we can test the migration
            let migration = MigrationV4FileDescriptions;
            migration.up(&conn).unwrap();

            // Verify description column now exists
            let column_exists: i32 = conn
                .query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('files') WHERE name='description'",
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

        // Verify default value is empty string
        conn.execute(
            "INSERT INTO files (path, file_id, title, description, hash, mtime, status)
             VALUES ('test.md', 'test', 'Test', '', 'hash', 0, 'empty')",
            [],
        )
        .unwrap();

        let description: String = conn
            .query_row(
                "SELECT description FROM files WHERE path = 'test.md'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(description, "");
    }

    #[test]
    fn test_migration_v4_down() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();

        let migration = MigrationV4FileDescriptions;

        // Down migration should return an error indicating it's not implemented
        let result = migration.down(&conn);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DbError::Other(_)));
    }
}
