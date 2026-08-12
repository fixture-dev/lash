//! Migration v9: Add `id_migrations` table for recording task IDs that moved
//!
//! A derived task ID is a function of the derivation rules, and those rules
//! can change between releases. When they do, the indexer re-derives every
//! file and the stored IDs shift underneath references written against the old
//! ones. The re-derive is the only moment both spellings exist at once, so it
//! is the only moment the old→new mapping can be recorded exactly rather than
//! guessed at. This table is where it goes, for `lash migrate-ids` to consume
//! afterwards.

use rusqlite::Connection;

use crate::error::DbResult;
use crate::migrations::Migration;

/// Migration to add the `id_migrations` table
pub(super) struct MigrationV9IdMigrations;

impl Migration for MigrationV9IdMigrations {
    fn version(&self) -> i32 {
        9
    }

    fn description(&self) -> &'static str {
        "Add id_migrations table recording task IDs moved by a derivation change"
    }

    fn up(&self, conn: &Connection) -> DbResult<()> {
        conn.execute_batch(
            "
            -- ============================================================================
            -- ID migrations table (task IDs moved by a derivation-rule change)
            -- ============================================================================

            CREATE TABLE IF NOT EXISTS id_migrations (
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

                -- One pending rename per (file, old id). Re-detecting the same
                -- rename overwrites rather than accumulating duplicates.
                UNIQUE(file_path, old_local_id)
            );

            -- Rewriting references means looking up by the id that appears in
            -- them, which is the old one.
            CREATE INDEX IF NOT EXISTS idx_id_migrations_old
                ON id_migrations(file_id, old_local_id);
            ",
        )?;

        Ok(())
    }

    fn down(&self, conn: &Connection) -> DbResult<()> {
        conn.execute_batch(
            "
            DROP INDEX IF EXISTS idx_id_migrations_old;
            DROP TABLE IF EXISTS id_migrations;
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
    fn test_migration_v9_up() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();

        let migration = MigrationV9IdMigrations;
        migration.up(&conn).unwrap();

        let table_exists: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='id_migrations'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table_exists, 1);
    }

    #[test]
    fn test_migration_v9_is_idempotent() {
        // `init_database` already creates the table from schema.sql, so the
        // migration has to be a no-op on a fresh database as well as on an
        // upgraded one.
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();

        let migration = MigrationV9IdMigrations;
        migration.up(&conn).unwrap();
        migration.up(&conn).unwrap();
    }

    #[test]
    fn test_migration_v9_down() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();

        let migration = MigrationV9IdMigrations;
        migration.up(&conn).unwrap();
        migration.down(&conn).unwrap();

        let table_exists: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='id_migrations'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table_exists, 0);
    }

    #[test]
    fn test_one_pending_rename_per_file_and_old_id() {
        // Re-running the detection must not accumulate duplicate rows, or
        // `lash migrate-ids` would report the same rename several times.
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();
        MigrationV9IdMigrations.up(&conn).unwrap();

        let insert = "INSERT OR REPLACE INTO id_migrations
             (file_path, file_id, old_local_id, new_local_id, title)
             VALUES ('tasks.md', 'tasks', 'old-id', ?1, 'A task')";
        conn.execute(insert, ["new-id"]).unwrap();
        conn.execute(insert, ["newer-id"]).unwrap();

        let (count, new_id): (i64, String) = conn
            .query_row(
                "SELECT COUNT(*), MAX(new_local_id) FROM id_migrations",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(count, 1);
        assert_eq!(new_id, "newer-id");
    }
}
