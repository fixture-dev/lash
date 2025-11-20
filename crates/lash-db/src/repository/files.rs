//! File repository for CRUD operations on task files

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use lash_types::{FileMetadata, FileStatus, TaskFile};

use crate::error::{DbError, DbResult};

/// A file record from the database
///
/// Represents a row from the files table, including all metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    /// Database ID
    pub id: i64,

    /// Relative path from project root
    pub path: PathBuf,

    /// File identifier (from @id or synthesized)
    pub file_id: String,

    /// Title from first H1 heading
    pub title: String,

    /// blake3 content hash
    pub hash: String,

    /// Unix timestamp of last modification
    pub mtime: i64,

    /// Overall file status
    pub status: FileStatus,

    /// File-level metadata
    pub metadata: FileMetadata,

    /// When indexed into database
    pub indexed_at: i64,
}

/// Repository for file operations
pub struct FileRepository<'conn> {
    conn: &'conn Connection,
}

impl<'conn> FileRepository<'conn> {
    /// Create a new file repository
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_db::connection::init_database;
    /// use lash_db::repository::FileRepository;
    /// use std::path::Path;
    ///
    /// let conn = init_database(Path::new("/tmp/lash.db")).unwrap();
    /// let repo = FileRepository::new(&conn);
    /// ```
    #[must_use]
    pub fn new(conn: &'conn Connection) -> Self {
        Self { conn }
    }

    /// Insert a new file into the database
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use lash_db::connection::init_database;
    /// # use lash_db::repository::FileRepository;
    /// # use lash_types::{TaskFile, FileMetadata, TaskTree};
    /// # use std::path::{Path, PathBuf};
    /// # use std::time::SystemTime;
    /// # let conn = init_database(Path::new("/tmp/lash.db")).unwrap();
    /// # let repo = FileRepository::new(&conn);
    /// let file = TaskFile {
    ///     path: PathBuf::from("test.md"),
    ///     title: "Test File".to_string(),
    ///     id: "test".to_string(),
    ///     metadata: FileMetadata::default(),
    ///     tasks: TaskTree::new(),
    ///     hash: "abc123".to_string(),
    ///     mtime: SystemTime::now(),
    /// };
    ///
    /// let file_id = repo.insert(&file).unwrap();
    /// assert!(file_id > 0);
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - File path already exists (unique constraint)
    /// - File ID already exists (unique constraint)
    /// - Metadata serialization fails
    pub fn insert(&self, file: &TaskFile) -> DbResult<i64> {
        let metadata_json = serde_json::to_string(&file.metadata)?;
        let status = file.compute_status();
        // SQLite uses i64 for integers, so we cast u64 seconds to i64.
        // This is safe until the year 2262 (i64::MAX seconds from epoch).
        #[allow(clippy::cast_possible_wrap)]
        let mtime = file
            .mtime
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| DbError::Other(format!("Invalid mtime: {e}")))?
            .as_secs() as i64;

        self.conn.execute(
            "INSERT INTO files (path, file_id, title, hash, mtime, status, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (
                file.path.to_string_lossy().as_ref(),
                &file.id,
                &file.title,
                &file.hash,
                mtime,
                status.as_str(),
                metadata_json,
            ),
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Update an existing file
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - File not found
    /// - Metadata serialization fails
    pub fn update(&self, file: &TaskFile) -> DbResult<()> {
        let metadata_json = serde_json::to_string(&file.metadata)?;
        let status = file.compute_status();
        // SQLite uses i64 for integers, so we cast u64 seconds to i64.
        // This is safe until the year 2262 (i64::MAX seconds from epoch).
        #[allow(clippy::cast_possible_wrap)]
        let mtime = file
            .mtime
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| DbError::Other(format!("Invalid mtime: {e}")))?
            .as_secs() as i64;

        let rows = self.conn.execute(
            "UPDATE files
             SET file_id = ?1, title = ?2, hash = ?3, mtime = ?4, status = ?5, metadata = ?6
             WHERE path = ?7",
            (
                &file.id,
                &file.title,
                &file.hash,
                mtime,
                status.as_str(),
                metadata_json,
                file.path.to_string_lossy().as_ref(),
            ),
        )?;

        if rows == 0 {
            return Err(DbError::FileNotFound(
                file.path.to_string_lossy().to_string(),
            ));
        }

        Ok(())
    }

    /// Delete a file by path
    ///
    /// This will cascade delete all tasks in the file due to foreign key constraints.
    ///
    /// # Errors
    ///
    /// Returns error if deletion fails
    pub fn delete(&self, path: &Path) -> DbResult<()> {
        self.conn.execute(
            "DELETE FROM files WHERE path = ?1",
            [path.to_string_lossy().as_ref()],
        )?;
        Ok(())
    }

    /// Get a file by its path
    ///
    /// # Errors
    ///
    /// Returns error if query fails or metadata deserialization fails
    pub fn get_by_path(&self, path: &Path) -> DbResult<Option<FileRecord>> {
        self.conn
            .query_row(
                "SELECT id, path, file_id, title, hash, mtime, status, metadata, indexed_at
                 FROM files WHERE path = ?1",
                [path.to_string_lossy().as_ref()],
                |row| {
                    let metadata_json: String = row.get(7)?;
                    let metadata: FileMetadata =
                        serde_json::from_str(&metadata_json).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                7,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?;

                    let status_str: String = row.get(6)?;
                    let status = FileStatus::from_str_lossy(&status_str);

                    Ok(FileRecord {
                        id: row.get(0)?,
                        path: PathBuf::from(row.get::<_, String>(1)?),
                        file_id: row.get(2)?,
                        title: row.get(3)?,
                        hash: row.get(4)?,
                        mtime: row.get(5)?,
                        status,
                        metadata,
                        indexed_at: row.get(8)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    /// Get a file by its file ID
    ///
    /// # Errors
    ///
    /// Returns error if query fails or metadata deserialization fails
    pub fn get_by_file_id(&self, file_id: &str) -> DbResult<Option<FileRecord>> {
        self.conn
            .query_row(
                "SELECT id, path, file_id, title, hash, mtime, status, metadata, indexed_at
                 FROM files WHERE file_id = ?1",
                [file_id],
                |row| {
                    let metadata_json: String = row.get(7)?;
                    let metadata: FileMetadata =
                        serde_json::from_str(&metadata_json).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                7,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?;

                    let status_str: String = row.get(6)?;
                    let status = FileStatus::from_str_lossy(&status_str);

                    Ok(FileRecord {
                        id: row.get(0)?,
                        path: PathBuf::from(row.get::<_, String>(1)?),
                        file_id: row.get(2)?,
                        title: row.get(3)?,
                        hash: row.get(4)?,
                        mtime: row.get(5)?,
                        status,
                        metadata,
                        indexed_at: row.get(8)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    /// List all files in the database
    ///
    /// # Errors
    ///
    /// Returns error if query fails
    pub fn list_all(&self) -> DbResult<Vec<FileRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, file_id, title, hash, mtime, status, metadata, indexed_at
             FROM files ORDER BY path",
        )?;

        let files = stmt
            .query_map([], |row| {
                let metadata_json: String = row.get(7)?;
                let metadata: FileMetadata = serde_json::from_str(&metadata_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        7,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

                let status_str: String = row.get(6)?;
                let status = FileStatus::from_str_lossy(&status_str);

                Ok(FileRecord {
                    id: row.get(0)?,
                    path: PathBuf::from(row.get::<_, String>(1)?),
                    file_id: row.get(2)?,
                    title: row.get(3)?,
                    hash: row.get(4)?,
                    mtime: row.get(5)?,
                    status,
                    metadata,
                    indexed_at: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(files)
    }

    /// Insert multiple files in a single transaction
    ///
    /// Much faster than individual inserts for bulk operations.
    ///
    /// # Errors
    ///
    /// Returns error if any insert fails
    pub fn insert_batch(&self, files: &[TaskFile]) -> DbResult<()> {
        let tx = self.conn.unchecked_transaction()?;

        for file in files {
            let metadata_json = serde_json::to_string(&file.metadata)?;
            let status = file.compute_status();
            // SQLite uses i64 for integers, so we cast u64 seconds to i64.
            // This is safe until the year 2262 (i64::MAX seconds from epoch).
            #[allow(clippy::cast_possible_wrap)]
            let mtime = file
                .mtime
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| DbError::Other(format!("Invalid mtime: {e}")))?
                .as_secs() as i64;

            tx.execute(
                "INSERT INTO files (path, file_id, title, hash, mtime, status, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                (
                    file.path.to_string_lossy().as_ref(),
                    &file.id,
                    &file.title,
                    &file.hash,
                    mtime,
                    status.as_str(),
                    metadata_json,
                ),
            )?;
        }

        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::init_database;
    use lash_types::TaskTree;
    use std::time::SystemTime;
    use tempfile::NamedTempFile;

    fn create_test_file(path: &str) -> TaskFile {
        TaskFile {
            path: PathBuf::from(path),
            title: "Test File".to_string(),
            id: path.replace('/', "."),
            metadata: FileMetadata::default(),
            tasks: TaskTree::new(),
            hash: "test_hash".to_string(),
            mtime: SystemTime::now(),
        }
    }

    #[test]
    fn test_insert_file() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        let repo = FileRepository::new(&conn);

        let file = create_test_file("test.md");
        let id = repo.insert(&file).unwrap();

        assert!(id > 0);
    }

    #[test]
    fn test_insert_duplicate_path_fails() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        let repo = FileRepository::new(&conn);

        let file = create_test_file("test.md");
        repo.insert(&file).unwrap();

        // Inserting again with same path should fail
        let result = repo.insert(&file);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_by_path() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        let repo = FileRepository::new(&conn);

        let file = create_test_file("test.md");
        repo.insert(&file).unwrap();

        let retrieved = repo.get_by_path(Path::new("test.md")).unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.path, PathBuf::from("test.md"));
        assert_eq!(retrieved.title, "Test File");
    }

    #[test]
    fn test_get_by_path_not_found() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        let repo = FileRepository::new(&conn);

        let retrieved = repo.get_by_path(Path::new("nonexistent.md")).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_get_by_file_id() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        let repo = FileRepository::new(&conn);

        let file = create_test_file("test.md");
        repo.insert(&file).unwrap();

        let retrieved = repo.get_by_file_id("test.md").unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.file_id, "test.md");
    }

    #[test]
    fn test_update_file() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        let repo = FileRepository::new(&conn);

        let mut file = create_test_file("test.md");
        repo.insert(&file).unwrap();

        // Update the file
        file.title = "Updated Title".to_string();
        file.hash = "new_hash".to_string();
        repo.update(&file).unwrap();

        let retrieved = repo.get_by_path(Path::new("test.md")).unwrap().unwrap();
        assert_eq!(retrieved.title, "Updated Title");
        assert_eq!(retrieved.hash, "new_hash");
    }

    #[test]
    fn test_update_nonexistent_file() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        let repo = FileRepository::new(&conn);

        let file = create_test_file("nonexistent.md");
        let result = repo.update(&file);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DbError::FileNotFound(_)));
    }

    #[test]
    fn test_delete_file() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        let repo = FileRepository::new(&conn);

        let file = create_test_file("test.md");
        repo.insert(&file).unwrap();

        repo.delete(Path::new("test.md")).unwrap();

        let retrieved = repo.get_by_path(Path::new("test.md")).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_list_all() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        let repo = FileRepository::new(&conn);

        let file1 = create_test_file("a.md");
        let file2 = create_test_file("b.md");
        let file3 = create_test_file("c.md");

        repo.insert(&file1).unwrap();
        repo.insert(&file2).unwrap();
        repo.insert(&file3).unwrap();

        let all_files = repo.list_all().unwrap();
        assert_eq!(all_files.len(), 3);

        // Should be ordered by path
        assert_eq!(all_files[0].path, PathBuf::from("a.md"));
        assert_eq!(all_files[1].path, PathBuf::from("b.md"));
        assert_eq!(all_files[2].path, PathBuf::from("c.md"));
    }

    #[test]
    fn test_insert_batch() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        let repo = FileRepository::new(&conn);

        let files = vec![
            create_test_file("file1.md"),
            create_test_file("file2.md"),
            create_test_file("file3.md"),
        ];

        repo.insert_batch(&files).unwrap();

        let all_files = repo.list_all().unwrap();
        assert_eq!(all_files.len(), 3);
    }

    #[test]
    fn test_insert_batch_rollback_on_error() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        let repo = FileRepository::new(&conn);

        // Insert one file first
        let existing_file = create_test_file("duplicate.md");
        repo.insert(&existing_file).unwrap();

        // Try to insert batch with duplicate - should fail and rollback
        let batch_files = vec![
            create_test_file("new1.md"),
            create_test_file("duplicate.md"), // This will cause unique constraint violation
            create_test_file("new2.md"),
        ];

        let result = repo.insert_batch(&batch_files);
        assert!(result.is_err());

        // Verify rollback - new1.md and new2.md should not be in database
        let all_files = repo.list_all().unwrap();
        assert_eq!(all_files.len(), 1);
        assert_eq!(all_files[0].path, PathBuf::from("duplicate.md"));
    }
}
