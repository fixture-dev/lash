//! Migration v3: Add `doc_refs` table for @doc annotation storage
//!
//! This migration adds the `doc_refs` table to store documentation references
//! from both file-level and task-level @doc annotations.

use rusqlite::Connection;

use crate::error::DbResult;
use crate::migrations::Migration;

/// Migration to add `doc_refs` table
pub(super) struct MigrationV3DocRefs;

impl Migration for MigrationV3DocRefs {
    fn version(&self) -> i32 {
        3
    }

    fn description(&self) -> &'static str {
        "Add doc_refs table for @doc annotation storage"
    }

    fn up(&self, conn: &Connection) -> DbResult<()> {
        // Create doc_refs table
        conn.execute_batch(
            "
            -- ============================================================================
            -- Doc Refs table (documentation references from @doc annotations)
            -- ============================================================================

            CREATE TABLE IF NOT EXISTS doc_refs (
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
            CREATE INDEX IF NOT EXISTS idx_doc_refs_source_file
                ON doc_refs(source_file_id);

            -- Index for finding all doc refs for a task
            CREATE INDEX IF NOT EXISTS idx_doc_refs_source_task
                ON doc_refs(source_task_id) WHERE source_task_id IS NOT NULL;

            -- Index for reverse lookup (find all sources that reference a doc)
            CREATE INDEX IF NOT EXISTS idx_doc_refs_target_path
                ON doc_refs(target_path);
            ",
        )?;

        Ok(())
    }

    fn down(&self, conn: &Connection) -> DbResult<()> {
        // Drop the table and its indexes
        conn.execute_batch(
            "
            DROP INDEX IF EXISTS idx_doc_refs_target_path;
            DROP INDEX IF EXISTS idx_doc_refs_source_task;
            DROP INDEX IF EXISTS idx_doc_refs_source_file;
            DROP TABLE IF EXISTS doc_refs;
            ",
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::init_database;
    use tempfile::NamedTempFile;

    #[test]
    fn test_migration_v3_up() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();

        let migration = MigrationV3DocRefs;
        migration.up(&conn).unwrap();

        // Verify table exists
        let table_exists: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='doc_refs'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table_exists, 1);

        // Verify indexes exist
        let index_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name LIKE 'idx_doc_refs_%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(index_count, 3);
    }

    #[test]
    fn test_migration_v3_down() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();

        let migration = MigrationV3DocRefs;
        migration.up(&conn).unwrap();
        migration.down(&conn).unwrap();

        // Verify table is dropped
        let table_exists: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='doc_refs'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table_exists, 0);
    }
}
