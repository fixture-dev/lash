//! Repository for task IDs moved by a derivation-rule change
//!
//! See [`crate::migrations`] v9 for why the mapping has to be captured at
//! re-derive time rather than reconstructed later.

use rusqlite::Connection;

use crate::error::DbResult;
use lash_types::dependency::make_full_id;
use std::path::{Path, PathBuf};

/// A task ID that moved because the derivation rules changed
///
/// Both spellings refer to the same task in the same file at the same line.
/// The old one is what existing `@depends-on` references were written against;
/// the new one is what lash derives today.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskIdRename {
    /// Path of the file the task lives in, relative to the project root
    pub file_path: PathBuf,

    /// The file's own id — the left half of a qualified task id
    pub file_id: String,

    /// The id stored before the derivation rules changed
    pub old_local_id: String,

    /// The id the current rules derive for the same task
    pub new_local_id: String,

    /// The task's title, so a rename is legible without re-reading the file
    pub title: String,
}

impl TaskIdRename {
    /// The qualified id references were written against
    #[must_use]
    pub fn old_full_id(&self) -> String {
        make_full_id(&self.file_id, &self.old_local_id)
    }

    /// The qualified id that resolves today
    #[must_use]
    pub fn new_full_id(&self) -> String {
        make_full_id(&self.file_id, &self.new_local_id)
    }
}

/// Reads and writes pending ID renames
pub struct IdMigrationRepository<'conn> {
    conn: &'conn Connection,
}

impl<'conn> IdMigrationRepository<'conn> {
    /// Create a repository over an open connection
    #[must_use]
    pub fn new(conn: &'conn Connection) -> Self {
        Self { conn }
    }

    /// Record renames detected during a re-derive
    ///
    /// Re-detecting a rename already on file replaces it rather than adding a
    /// duplicate, so running `lash index` repeatedly before migrating does not
    /// inflate the pending list.
    ///
    /// # Errors
    ///
    /// Returns error if any insert fails
    pub fn record_all(&self, renames: &[TaskIdRename]) -> DbResult<()> {
        let mut stmt = self.conn.prepare(
            "INSERT OR REPLACE INTO id_migrations
                 (file_path, file_id, old_local_id, new_local_id, title)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;

        for rename in renames {
            stmt.execute(rusqlite::params![
                rename.file_path.to_string_lossy(),
                rename.file_id,
                rename.old_local_id,
                rename.new_local_id,
                rename.title,
            ])?;
        }

        Ok(())
    }

    /// Every rename still awaiting a reference rewrite
    ///
    /// # Errors
    ///
    /// Returns error if the query fails
    pub fn list_pending(&self) -> DbResult<Vec<TaskIdRename>> {
        let mut stmt = self.conn.prepare(
            "SELECT file_path, file_id, old_local_id, new_local_id, title
             FROM id_migrations
             ORDER BY file_path, old_local_id",
        )?;

        let rows = stmt.query_map([], |row| {
            let file_path: String = row.get(0)?;
            Ok(TaskIdRename {
                file_path: PathBuf::from(file_path),
                file_id: row.get(1)?,
                old_local_id: row.get(2)?,
                new_local_id: row.get(3)?,
                title: row.get(4)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// How many renames are pending
    ///
    /// # Errors
    ///
    /// Returns error if the query fails
    pub fn pending_count(&self) -> DbResult<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM id_migrations", [], |row| row.get(0))?;
        #[allow(clippy::cast_sign_loss)]
        Ok(count as usize)
    }

    /// Drop the renames for one file, once its references have been rewritten
    ///
    /// # Errors
    ///
    /// Returns error if the delete fails
    pub fn clear_file(&self, file_path: &Path) -> DbResult<()> {
        self.conn.execute(
            "DELETE FROM id_migrations WHERE file_path = ?1",
            [file_path.to_string_lossy()],
        )?;
        Ok(())
    }

    /// Drop every pending rename
    ///
    /// # Errors
    ///
    /// Returns error if the delete fails
    pub fn clear_all(&self) -> DbResult<()> {
        self.conn.execute("DELETE FROM id_migrations", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::init_database;
    use tempfile::NamedTempFile;

    fn rename(file: &str, old: &str, new: &str) -> TaskIdRename {
        TaskIdRename {
            file_path: PathBuf::from(file),
            file_id: file.trim_end_matches(".md").to_string(),
            old_local_id: old.to_string(),
            new_local_id: new.to_string(),
            title: "A task".to_string(),
        }
    }

    fn test_conn() -> Connection {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();
        // Keep the file alive for the connection's lifetime.
        std::mem::forget(temp_file);
        conn
    }

    #[test]
    fn test_record_and_list() {
        let conn = test_conn();
        let repo = IdMigrationRepository::new(&conn);

        repo.record_all(&[
            rename("tasks.md", "old-b", "new-b"),
            rename("tasks.md", "old-a", "new-a"),
        ])
        .unwrap();

        let pending = repo.list_pending().unwrap();
        assert_eq!(pending.len(), 2);
        // Ordered, so output is stable between runs.
        assert_eq!(pending[0].old_local_id, "old-a");
        assert_eq!(pending[1].old_local_id, "old-b");
        assert_eq!(repo.pending_count().unwrap(), 2);
    }

    #[test]
    fn test_recording_the_same_rename_twice_does_not_duplicate() {
        // `lash index` may run several times before anyone gets around to
        // migrating, and each run re-detects the same drift.
        let conn = test_conn();
        let repo = IdMigrationRepository::new(&conn);

        repo.record_all(&[rename("tasks.md", "old-a", "new-a")])
            .unwrap();
        repo.record_all(&[rename("tasks.md", "old-a", "new-a")])
            .unwrap();

        assert_eq!(repo.pending_count().unwrap(), 1);
    }

    #[test]
    fn test_clear_file_leaves_other_files_alone() {
        let conn = test_conn();
        let repo = IdMigrationRepository::new(&conn);

        repo.record_all(&[
            rename("tasks.md", "old-a", "new-a"),
            rename("other.md", "old-b", "new-b"),
        ])
        .unwrap();

        repo.clear_file(Path::new("tasks.md")).unwrap();

        let pending = repo.list_pending().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].file_path, PathBuf::from("other.md"));
    }

    #[test]
    fn test_clear_all() {
        let conn = test_conn();
        let repo = IdMigrationRepository::new(&conn);

        repo.record_all(&[rename("tasks.md", "old-a", "new-a")])
            .unwrap();
        repo.clear_all().unwrap();

        assert_eq!(repo.pending_count().unwrap(), 0);
        assert!(repo.list_pending().unwrap().is_empty());
    }

    #[test]
    fn test_full_ids_are_qualified_with_the_file() {
        // References are written qualified, so that is the form the rewrite
        // has to search for and produce.
        let r = rename("tasks.md", "old-a", "new-a");
        assert_eq!(r.old_full_id(), "tasks#old-a");
        assert_eq!(r.new_full_id(), "tasks#new-a");
    }
}
