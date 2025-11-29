//! Documentation reference repository for managing @doc annotations
//!
//! Provides CRUD operations for documentation references stored in the database.
//! Supports both file-level and task-level @doc annotations.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use lash_types::dependency::DocRef;

use crate::error::DbResult;

/// A documentation reference record from the database
///
/// Represents a single @doc annotation, linking a source (file or task) to
/// a target documentation resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocRefRow {
    /// Database ID
    pub id: i64,

    /// Source file database ID
    pub source_file_id: i64,

    /// Source task database ID (None for file-level @doc)
    pub source_task_id: Option<i64>,

    /// Target document path
    pub target_path: String,

    /// Optional fragment identifier
    pub fragment: Option<String>,
}

impl DocRefRow {
    /// Convert to a `DocRef` type
    #[must_use]
    pub fn to_doc_ref(&self) -> DocRef {
        DocRef::new(self.target_path.clone(), self.fragment.clone())
    }
}

/// Repository for documentation reference operations
pub struct DocRefRepository<'conn> {
    conn: &'conn Connection,
}

impl<'conn> DocRefRepository<'conn> {
    /// Create a new documentation reference repository
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_db::connection::init_database;
    /// use lash_db::repository::DocRefRepository;
    /// use std::path::Path;
    ///
    /// let conn = init_database(Path::new("/tmp/lash.db")).unwrap();
    /// let repo = DocRefRepository::new(&conn);
    /// ```
    #[must_use]
    pub fn new(conn: &'conn Connection) -> Self {
        Self { conn }
    }

    /// Insert a documentation reference
    ///
    /// # Arguments
    ///
    /// * `doc_ref` - The documentation reference to insert
    /// * `file_id` - Database ID of the source file
    /// * `task_id` - Optional database ID of the source task (None for file-level @doc)
    ///
    /// # Returns
    ///
    /// Database ID of the inserted row
    ///
    /// # Errors
    ///
    /// Returns error if insert fails
    pub fn insert(&self, doc_ref: &DocRef, file_id: i64, task_id: Option<i64>) -> DbResult<i64> {
        self.conn.execute(
            "INSERT INTO doc_refs (source_file_id, source_task_id, target_path, fragment)
             VALUES (?1, ?2, ?3, ?4)",
            (file_id, task_id, &doc_ref.path, &doc_ref.fragment),
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Insert multiple documentation references in a batch
    ///
    /// More efficient than individual inserts for bulk operations.
    ///
    /// # Arguments
    ///
    /// * `doc_refs` - Slice of tuples (`DocRef`, `file_id`, `task_id`)
    ///
    /// # Errors
    ///
    /// Returns error if any insert fails. Transaction is rolled back on error.
    pub fn insert_batch(&self, doc_refs: &[(DocRef, i64, Option<i64>)]) -> DbResult<()> {
        let tx = self.conn.unchecked_transaction()?;

        for (doc_ref, file_id, task_id) in doc_refs {
            tx.execute(
                "INSERT INTO doc_refs (source_file_id, source_task_id, target_path, fragment)
                 VALUES (?1, ?2, ?3, ?4)",
                (file_id, task_id, &doc_ref.path, &doc_ref.fragment),
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Find all documentation references for a file (both file-level and task-level)
    ///
    /// Returns all @doc references associated with the file, including those on
    /// individual tasks within the file.
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn find_by_file(&self, file_id: i64) -> DbResult<Vec<DocRefRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_file_id, source_task_id, target_path, fragment
             FROM doc_refs
             WHERE source_file_id = ?1
             ORDER BY source_task_id NULLS FIRST, id",
        )?;

        let rows = stmt
            .query_map([file_id], |row| {
                Ok(DocRefRow {
                    id: row.get(0)?,
                    source_file_id: row.get(1)?,
                    source_task_id: row.get(2)?,
                    target_path: row.get(3)?,
                    fragment: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    /// Find all file-level documentation references for a file
    ///
    /// Returns only @doc references at the file level, not task-level references.
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn find_file_level(&self, file_id: i64) -> DbResult<Vec<DocRefRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_file_id, source_task_id, target_path, fragment
             FROM doc_refs
             WHERE source_file_id = ?1 AND source_task_id IS NULL
             ORDER BY id",
        )?;

        let rows = stmt
            .query_map([file_id], |row| {
                Ok(DocRefRow {
                    id: row.get(0)?,
                    source_file_id: row.get(1)?,
                    source_task_id: row.get(2)?,
                    target_path: row.get(3)?,
                    fragment: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    /// Find all documentation references for a specific task
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn find_by_task(&self, task_id: i64) -> DbResult<Vec<DocRefRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_file_id, source_task_id, target_path, fragment
             FROM doc_refs
             WHERE source_task_id = ?1
             ORDER BY id",
        )?;

        let rows = stmt
            .query_map([task_id], |row| {
                Ok(DocRefRow {
                    id: row.get(0)?,
                    source_file_id: row.get(1)?,
                    source_task_id: row.get(2)?,
                    target_path: row.get(3)?,
                    fragment: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    /// Find all sources that reference a given target path (reverse lookup)
    ///
    /// Returns tuples of (`file_id`, `task_id`) where `task_id` is None for file-level refs.
    /// This is useful for finding all tasks/files that link to a particular document.
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn find_by_target(&self, path: &str) -> DbResult<Vec<(i64, Option<i64>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT source_file_id, source_task_id
             FROM doc_refs
             WHERE target_path = ?1
             ORDER BY source_file_id, source_task_id",
        )?;

        let sources = stmt
            .query_map([path], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(sources)
    }

    /// Find all sources that reference any target starting with a given prefix
    ///
    /// Useful for finding all references to documents in a directory.
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn find_by_target_prefix(&self, prefix: &str) -> DbResult<Vec<DocRefRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_file_id, source_task_id, target_path, fragment
             FROM doc_refs
             WHERE target_path LIKE ?1
             ORDER BY target_path, source_file_id, source_task_id",
        )?;

        let pattern = format!("{prefix}%");
        let rows = stmt
            .query_map([pattern], |row| {
                Ok(DocRefRow {
                    id: row.get(0)?,
                    source_file_id: row.get(1)?,
                    source_task_id: row.get(2)?,
                    target_path: row.get(3)?,
                    fragment: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    /// Delete all documentation references for a file (for re-indexing)
    ///
    /// Removes both file-level and task-level @doc references.
    /// Returns the number of rows deleted.
    ///
    /// # Errors
    ///
    /// Returns error if delete fails
    pub fn delete_by_file(&self, file_id: i64) -> DbResult<usize> {
        let count = self
            .conn
            .execute("DELETE FROM doc_refs WHERE source_file_id = ?1", [file_id])?;
        Ok(count)
    }

    /// Delete all documentation references for a specific task
    ///
    /// Returns the number of rows deleted.
    ///
    /// # Errors
    ///
    /// Returns error if delete fails
    pub fn delete_by_task(&self, task_id: i64) -> DbResult<usize> {
        let count = self
            .conn
            .execute("DELETE FROM doc_refs WHERE source_task_id = ?1", [task_id])?;
        Ok(count)
    }

    /// Get statistics about documentation references
    ///
    /// Returns (`total_count`, `file_level_count`, `task_level_count`)
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn get_stats(&self) -> DbResult<(usize, usize, usize)> {
        let total: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM doc_refs", [], |row| row.get(0))?;

        let file_level: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM doc_refs WHERE source_task_id IS NULL",
            [],
            |row| row.get(0),
        )?;

        #[allow(clippy::cast_sign_loss)]
        Ok((
            total as usize,
            file_level as usize,
            (total - file_level) as usize,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::init_database;
    use crate::migrations::run_migrations;
    use crate::repository::{FileRepository, TaskRepository};
    use lash_types::{FileMetadata, Task, TaskFile, TaskMetadata, TaskStatus, TaskTree};
    use std::path::PathBuf;
    use std::time::SystemTime;
    use tempfile::NamedTempFile;

    fn setup_test_db() -> (NamedTempFile, rusqlite::Connection) {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        run_migrations(&conn).unwrap();
        (temp_db, conn)
    }

    fn create_test_file(path: &str, file_id: &str) -> TaskFile {
        TaskFile {
            path: PathBuf::from(path),
            title: "Test File".to_string(),
            id: file_id.to_string(),
            metadata: FileMetadata::default(),
            tasks: TaskTree::new(),
            hash: "test_hash".to_string(),
            mtime: SystemTime::now(),
        }
    }

    fn create_test_task(id: &str, title: &str) -> Task {
        Task {
            id: id.to_string(),
            title: title.to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            metadata: TaskMetadata::default(),
            body: None,
        }
    }

    #[test]
    fn test_insert_file_level_doc_ref() {
        let (_temp_db, conn) = setup_test_db();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let doc_ref = DocRef::new("../docs/design.md", Some("section-1".to_string()));
        let doc_repo = DocRefRepository::new(&conn);
        let doc_id = doc_repo.insert(&doc_ref, file_db_id, None).unwrap();

        assert!(doc_id > 0);
    }

    #[test]
    fn test_insert_task_level_doc_ref() {
        let (_temp_db, conn) = setup_test_db();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task = create_test_task("task1", "Task 1");
        let task_repo = TaskRepository::new(&conn);
        let task_db_id = task_repo.insert(&task, file_db_id, "test").unwrap();

        let doc_ref = DocRef::new("../docs/api.md", None);
        let doc_repo = DocRefRepository::new(&conn);
        let doc_id = doc_repo
            .insert(&doc_ref, file_db_id, Some(task_db_id))
            .unwrap();

        assert!(doc_id > 0);
    }

    #[test]
    fn test_find_by_file() {
        let (_temp_db, conn) = setup_test_db();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task = create_test_task("task1", "Task 1");
        let task_repo = TaskRepository::new(&conn);
        let task_db_id = task_repo.insert(&task, file_db_id, "test").unwrap();

        let doc_repo = DocRefRepository::new(&conn);

        // Insert file-level doc ref
        let doc1 = DocRef::new("../docs/design.md", None);
        doc_repo.insert(&doc1, file_db_id, None).unwrap();

        // Insert task-level doc ref
        let doc2 = DocRef::new("../docs/api.md", Some("auth".to_string()));
        doc_repo
            .insert(&doc2, file_db_id, Some(task_db_id))
            .unwrap();

        // Find all doc refs for file
        let refs = doc_repo.find_by_file(file_db_id).unwrap();
        assert_eq!(refs.len(), 2);

        // File-level ref should be first (NULLS FIRST in ORDER BY)
        assert_eq!(refs[0].source_task_id, None);
        assert_eq!(refs[0].target_path, "../docs/design.md");

        assert_eq!(refs[1].source_task_id, Some(task_db_id));
        assert_eq!(refs[1].target_path, "../docs/api.md");
        assert_eq!(refs[1].fragment, Some("auth".to_string()));
    }

    #[test]
    fn test_find_file_level() {
        let (_temp_db, conn) = setup_test_db();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task = create_test_task("task1", "Task 1");
        let task_repo = TaskRepository::new(&conn);
        let task_db_id = task_repo.insert(&task, file_db_id, "test").unwrap();

        let doc_repo = DocRefRepository::new(&conn);

        // Insert file-level doc ref
        let doc1 = DocRef::new("../docs/design.md", None);
        doc_repo.insert(&doc1, file_db_id, None).unwrap();

        // Insert task-level doc ref
        let doc2 = DocRef::new("../docs/api.md", None);
        doc_repo
            .insert(&doc2, file_db_id, Some(task_db_id))
            .unwrap();

        // Find only file-level refs
        let refs = doc_repo.find_file_level(file_db_id).unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].target_path, "../docs/design.md");
        assert_eq!(refs[0].source_task_id, None);
    }

    #[test]
    fn test_find_by_task() {
        let (_temp_db, conn) = setup_test_db();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task = create_test_task("task1", "Task 1");
        let task_repo = TaskRepository::new(&conn);
        let task_db_id = task_repo.insert(&task, file_db_id, "test").unwrap();

        let doc_repo = DocRefRepository::new(&conn);

        // Insert task-level doc ref
        let doc = DocRef::new("../docs/api.md", Some("section".to_string()));
        doc_repo.insert(&doc, file_db_id, Some(task_db_id)).unwrap();

        // Find doc refs for task
        let refs = doc_repo.find_by_task(task_db_id).unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].target_path, "../docs/api.md");
        assert_eq!(refs[0].fragment, Some("section".to_string()));
    }

    #[test]
    fn test_find_by_target() {
        let (_temp_db, conn) = setup_test_db();

        let file1 = create_test_file("test1.md", "test1");
        let file2 = create_test_file("test2.md", "test2");
        let file_repo = FileRepository::new(&conn);
        let file1_db_id = file_repo.insert(&file1).unwrap();
        let file2_db_id = file_repo.insert(&file2).unwrap();

        let doc_repo = DocRefRepository::new(&conn);
        let target = "../docs/design.md";

        // Insert refs from two different files to same target
        let doc = DocRef::new(target, None);
        doc_repo.insert(&doc, file1_db_id, None).unwrap();
        doc_repo.insert(&doc, file2_db_id, None).unwrap();

        // Find all sources that reference this target
        let sources = doc_repo.find_by_target(target).unwrap();
        assert_eq!(sources.len(), 2);
        assert!(sources.contains(&(file1_db_id, None)));
        assert!(sources.contains(&(file2_db_id, None)));
    }

    #[test]
    fn test_find_by_target_prefix() {
        let (_temp_db, conn) = setup_test_db();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let doc_repo = DocRefRepository::new(&conn);

        // Insert refs to different docs in same directory
        let doc1 = DocRef::new("../docs/design.md", None);
        let doc2 = DocRef::new("../docs/api.md", None);
        let doc3 = DocRef::new("../other/guide.md", None);

        doc_repo.insert(&doc1, file_db_id, None).unwrap();
        doc_repo.insert(&doc2, file_db_id, None).unwrap();
        doc_repo.insert(&doc3, file_db_id, None).unwrap();

        // Find all refs to docs in ../docs/
        let refs = doc_repo.find_by_target_prefix("../docs/").unwrap();
        assert_eq!(refs.len(), 2);

        let paths: Vec<_> = refs.iter().map(|r| r.target_path.as_str()).collect();
        assert!(paths.contains(&"../docs/design.md"));
        assert!(paths.contains(&"../docs/api.md"));
    }

    #[test]
    fn test_delete_by_file() {
        let (_temp_db, conn) = setup_test_db();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let doc_repo = DocRefRepository::new(&conn);

        // Insert multiple doc refs
        let doc1 = DocRef::new("../docs/design.md", None);
        let doc2 = DocRef::new("../docs/api.md", None);
        doc_repo.insert(&doc1, file_db_id, None).unwrap();
        doc_repo.insert(&doc2, file_db_id, None).unwrap();

        // Delete all doc refs for file
        let deleted = doc_repo.delete_by_file(file_db_id).unwrap();
        assert_eq!(deleted, 2);

        // Verify they're gone
        let refs = doc_repo.find_by_file(file_db_id).unwrap();
        assert_eq!(refs.len(), 0);
    }

    #[test]
    fn test_delete_by_task() {
        let (_temp_db, conn) = setup_test_db();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task = create_test_task("task1", "Task 1");
        let task_repo = TaskRepository::new(&conn);
        let task_db_id = task_repo.insert(&task, file_db_id, "test").unwrap();

        let doc_repo = DocRefRepository::new(&conn);

        // Insert task-level doc ref
        let doc = DocRef::new("../docs/api.md", None);
        doc_repo.insert(&doc, file_db_id, Some(task_db_id)).unwrap();

        // Delete doc refs for task
        let deleted = doc_repo.delete_by_task(task_db_id).unwrap();
        assert_eq!(deleted, 1);

        // Verify it's gone
        let refs = doc_repo.find_by_task(task_db_id).unwrap();
        assert_eq!(refs.len(), 0);
    }

    #[test]
    fn test_cascade_delete_on_file_delete() {
        let (_temp_db, conn) = setup_test_db();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let doc_repo = DocRefRepository::new(&conn);

        // Insert doc ref
        let doc = DocRef::new("../docs/design.md", None);
        doc_repo.insert(&doc, file_db_id, None).unwrap();

        // Delete file (should cascade delete doc refs)
        file_repo.delete(&file.path).unwrap();

        // Verify doc refs were deleted
        let refs = doc_repo.find_by_file(file_db_id).unwrap();
        assert_eq!(refs.len(), 0);
    }

    #[test]
    fn test_cascade_delete_on_task_delete() {
        let (_temp_db, conn) = setup_test_db();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task = create_test_task("task1", "Task 1");
        let task_repo = TaskRepository::new(&conn);
        let task_db_id = task_repo.insert(&task, file_db_id, "test").unwrap();

        let doc_repo = DocRefRepository::new(&conn);

        // Insert task-level doc ref
        let doc = DocRef::new("../docs/api.md", None);
        doc_repo.insert(&doc, file_db_id, Some(task_db_id)).unwrap();

        // Delete task directly (should cascade delete doc refs)
        conn.execute("DELETE FROM tasks WHERE id = ?1", [task_db_id])
            .unwrap();

        // Verify doc refs were deleted
        let refs = doc_repo.find_by_task(task_db_id).unwrap();
        assert_eq!(refs.len(), 0);
    }

    #[test]
    fn test_get_stats() {
        let (_temp_db, conn) = setup_test_db();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let task = create_test_task("task1", "Task 1");
        let task_repo = TaskRepository::new(&conn);
        let task_db_id = task_repo.insert(&task, file_db_id, "test").unwrap();

        let doc_repo = DocRefRepository::new(&conn);

        // Insert 2 file-level and 3 task-level refs
        doc_repo
            .insert(&DocRef::new("../docs/a.md", None), file_db_id, None)
            .unwrap();
        doc_repo
            .insert(&DocRef::new("../docs/b.md", None), file_db_id, None)
            .unwrap();
        doc_repo
            .insert(
                &DocRef::new("../docs/c.md", None),
                file_db_id,
                Some(task_db_id),
            )
            .unwrap();
        doc_repo
            .insert(
                &DocRef::new("../docs/d.md", None),
                file_db_id,
                Some(task_db_id),
            )
            .unwrap();
        doc_repo
            .insert(
                &DocRef::new("../docs/e.md", None),
                file_db_id,
                Some(task_db_id),
            )
            .unwrap();

        let (total, file_level, task_level) = doc_repo.get_stats().unwrap();
        assert_eq!(total, 5);
        assert_eq!(file_level, 2);
        assert_eq!(task_level, 3);
    }

    #[test]
    fn test_insert_batch() {
        let (_temp_db, conn) = setup_test_db();

        let file = create_test_file("test.md", "test");
        let file_repo = FileRepository::new(&conn);
        let file_db_id = file_repo.insert(&file).unwrap();

        let doc_repo = DocRefRepository::new(&conn);

        let batch = vec![
            (DocRef::new("../docs/a.md", None), file_db_id, None),
            (DocRef::new("../docs/b.md", None), file_db_id, None),
            (
                DocRef::new("../docs/c.md", Some("section".to_string())),
                file_db_id,
                None,
            ),
        ];

        doc_repo.insert_batch(&batch).unwrap();

        let refs = doc_repo.find_by_file(file_db_id).unwrap();
        assert_eq!(refs.len(), 3);
    }

    #[test]
    fn test_doc_ref_row_to_doc_ref() {
        let row = DocRefRow {
            id: 1,
            source_file_id: 10,
            source_task_id: Some(20),
            target_path: "../docs/api.md".to_string(),
            fragment: Some("auth".to_string()),
        };

        let doc_ref = row.to_doc_ref();
        assert_eq!(doc_ref.path, "../docs/api.md");
        assert_eq!(doc_ref.fragment, Some("auth".to_string()));
    }
}
