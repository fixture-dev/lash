//! Migration v2: Enhanced FTS5 index with labels and file paths
//!
//! This migration upgrades the FTS5 search index to include:
//! - Labels (space-separated)
//! - File paths for filename matching
//! - Improved tokenization with unicode61

use rusqlite::Connection;

use crate::error::DbResult;
use crate::migrations::Migration;

/// Migration to enhance the FTS5 index
pub struct MigrationV2EnhancedFts;

impl Migration for MigrationV2EnhancedFts {
    fn version(&self) -> i32 {
        2
    }

    fn description(&self) -> &'static str {
        "Enhance FTS5 index with labels and file paths"
    }

    #[allow(clippy::too_many_lines)] // Migration SQL is verbose but clear
    fn up(&self, conn: &Connection) -> DbResult<()> {
        // Drop old FTS table and triggers
        conn.execute_batch(
            "
            DROP TRIGGER IF EXISTS tasks_ai;
            DROP TRIGGER IF EXISTS tasks_au;
            DROP TRIGGER IF EXISTS tasks_ad;
            DROP TABLE IF EXISTS tasks_fts;
            ",
        )?;

        // Create new FTS5 table with enhanced schema
        conn.execute_batch(
            "
            CREATE VIRTUAL TABLE tasks_fts USING fts5(
                full_id UNINDEXED,
                title,
                body,
                labels,
                file_path,
                tokenize='unicode61 remove_diacritics 2'
            );
            ",
        )?;

        // Create new triggers (drop first if they exist)
        conn.execute_batch(
            "
            DROP TRIGGER IF EXISTS task_labels_ai;
            DROP TRIGGER IF EXISTS task_labels_ad;

            CREATE TRIGGER tasks_ai AFTER INSERT ON tasks BEGIN
                INSERT INTO tasks_fts(rowid, full_id, title, body, labels, file_path)
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
                    f.path
                FROM files f
                WHERE f.id = new.file_id;
            END;

            CREATE TRIGGER tasks_au AFTER UPDATE ON tasks BEGIN
                DELETE FROM tasks_fts WHERE rowid = old.id;
                INSERT INTO tasks_fts(rowid, full_id, title, body, labels, file_path)
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
                    f.path
                FROM files f
                WHERE f.id = new.file_id;
            END;

            CREATE TRIGGER tasks_ad AFTER DELETE ON tasks BEGIN
                DELETE FROM tasks_fts WHERE rowid = old.id;
            END;

            CREATE TRIGGER task_labels_ai AFTER INSERT ON task_labels BEGIN
                DELETE FROM tasks_fts WHERE rowid = new.task_id;
                INSERT INTO tasks_fts(rowid, full_id, title, body, labels, file_path)
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
                    f.path
                FROM tasks t
                JOIN files f ON f.id = t.file_id
                WHERE t.id = new.task_id;
            END;

            CREATE TRIGGER task_labels_ad AFTER DELETE ON task_labels BEGIN
                DELETE FROM tasks_fts WHERE rowid = old.task_id;
                INSERT INTO tasks_fts(rowid, full_id, title, body, labels, file_path)
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
                    f.path
                FROM tasks t
                JOIN files f ON f.id = t.file_id
                WHERE t.id = old.task_id;
            END;
            ",
        )?;

        // Repopulate FTS index from existing tasks
        conn.execute_batch(
            "
            INSERT INTO tasks_fts(rowid, full_id, title, body, labels, file_path)
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
                f.path
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
    fn test_migration_v2_up() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();

        // Run migration
        let migration = MigrationV2EnhancedFts;
        migration.up(&conn).unwrap();

        // Verify new FTS table structure
        let column_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('tasks_fts')",
                [],
                |row| row.get(0),
            )
            .unwrap();

        // Should have 5 columns: full_id, title, body, labels, file_path
        assert_eq!(column_count, 5);
    }
}
