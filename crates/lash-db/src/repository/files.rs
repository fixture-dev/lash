//! File repository for CRUD operations on task files

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use lash_types::{FileMetadata, FileStatus, TaskFile};

use crate::error::{DbError, DbResult};

/// Convert a path to a normalized string representation with forward slashes
///
/// This ensures consistent path representation across all platforms:
/// - Windows paths with backslashes are converted to forward slashes
/// - Unix paths remain unchanged
/// - Enables cross-platform database compatibility
///
/// # Example
///
/// ```
/// # use std::path::Path;
/// # use lash_db::repository::normalize_path_for_db;
/// // On Windows: "features\\auth.md" -> "features/auth.md"
/// // On Unix:    "features/auth.md"  -> "features/auth.md"
/// let path = Path::new("features").join("auth.md");
/// let normalized = normalize_path_for_db(&path);
/// assert_eq!(normalized, "features/auth.md");
/// ```
#[must_use]
pub fn normalize_path_for_db(path: &Path) -> String {
    // Convert path to string using lossy conversion (handles invalid UTF-8)
    let path_str = path.to_string_lossy();

    // On Windows, replace backslashes with forward slashes
    // On Unix, this is a no-op since paths already use forward slashes
    #[cfg(windows)]
    {
        path_str.replace('\\', "/")
    }

    #[cfg(not(windows))]
    {
        path_str.to_string()
    }
}

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
                normalize_path_for_db(&file.path),
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
                normalize_path_for_db(&file.path),
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
            [normalize_path_for_db(path)],
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
                [normalize_path_for_db(path)],
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

    /// Get a file by its database ID
    ///
    /// # Errors
    ///
    /// Returns error if query fails or metadata deserialization fails
    pub fn get_by_db_id(&self, id: i64) -> DbResult<Option<FileRecord>> {
        self.conn
            .query_row(
                "SELECT id, path, file_id, title, hash, mtime, status, metadata, indexed_at
                 FROM files WHERE id = ?1",
                [id],
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
                    normalize_path_for_db(&file.path),
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

    /// Upsert (insert or update) multiple files in a single batch transaction
    ///
    /// Uses `SQLite`'s `INSERT ... ON CONFLICT ... DO UPDATE` for efficient upserts.
    /// This is significantly faster than checking existence and then doing separate
    /// insert/update operations.
    ///
    /// Returns a map of file paths to their database IDs.
    ///
    /// # Errors
    ///
    /// Returns error if any operation fails or if metadata serialization fails.
    pub fn upsert_batch(
        &self,
        files: &[TaskFile],
    ) -> DbResult<std::collections::HashMap<PathBuf, i64>> {
        use std::collections::HashMap;

        let tx = self.conn.unchecked_transaction()?;
        let mut path_to_id = HashMap::new();

        for file in files {
            let metadata_json = serde_json::to_string(&file.metadata)?;
            let status = file.compute_status();
            #[allow(clippy::cast_possible_wrap)]
            let mtime = file
                .mtime
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| DbError::Other(format!("Invalid mtime: {e}")))?
                .as_secs() as i64;

            let normalized_path = normalize_path_for_db(&file.path);

            tx.execute(
                "INSERT INTO files (path, file_id, title, hash, mtime, status, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(path) DO UPDATE SET
                   file_id = excluded.file_id,
                   title = excluded.title,
                   hash = excluded.hash,
                   mtime = excluded.mtime,
                   status = excluded.status,
                   metadata = excluded.metadata",
                (
                    &normalized_path,
                    &file.id,
                    &file.title,
                    &file.hash,
                    mtime,
                    status.as_str(),
                    metadata_json,
                ),
            )?;

            // Get the file's database ID
            let file_id: i64 = tx.query_row(
                "SELECT id FROM files WHERE path = ?1",
                [&normalized_path],
                |row| row.get(0),
            )?;

            path_to_id.insert(file.path.clone(), file_id);
        }

        tx.commit()?;
        Ok(path_to_id)
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
    fn test_upsert_batch_insert() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        let repo = FileRepository::new(&conn);

        let files = vec![
            create_test_file("file1.md"),
            create_test_file("file2.md"),
            create_test_file("file3.md"),
        ];

        let path_to_id = repo.upsert_batch(&files).unwrap();

        assert_eq!(path_to_id.len(), 3);
        assert!(path_to_id.contains_key(&PathBuf::from("file1.md")));
        assert!(path_to_id.contains_key(&PathBuf::from("file2.md")));
        assert!(path_to_id.contains_key(&PathBuf::from("file3.md")));

        let all_files = repo.list_all().unwrap();
        assert_eq!(all_files.len(), 3);
    }

    #[test]
    fn test_upsert_batch_update() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        let repo = FileRepository::new(&conn);

        // First insert
        let mut file = create_test_file("test.md");
        file.title = "Original Title".to_string();
        repo.insert(&file).unwrap();

        // Now upsert with updated title
        file.title = "Updated Title".to_string();
        file.hash = "new_hash".to_string();
        let path_to_id = repo.upsert_batch(&[file]).unwrap();

        assert_eq!(path_to_id.len(), 1);

        // Verify it was updated, not duplicated
        let all_files = repo.list_all().unwrap();
        assert_eq!(all_files.len(), 1);
        assert_eq!(all_files[0].title, "Updated Title");
        assert_eq!(all_files[0].hash, "new_hash");
    }

    #[test]
    #[allow(clippy::similar_names)]
    fn test_upsert_batch_mixed() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        let repo = FileRepository::new(&conn);

        // Insert one file first
        let file1 = create_test_file("file1.md");
        repo.insert(&file1).unwrap();

        // Now upsert batch with one existing and two new
        let mut file1_updated = create_test_file("file1.md");
        file1_updated.title = "Updated".to_string();
        let batch_files = vec![
            file1_updated,
            create_test_file("file2.md"),
            create_test_file("file3.md"),
        ];

        let path_to_id = repo.upsert_batch(&batch_files).unwrap();

        assert_eq!(path_to_id.len(), 3);

        let all_files = repo.list_all().unwrap();
        assert_eq!(all_files.len(), 3);

        // Verify file1 was updated
        let file1_record = repo.get_by_path(Path::new("file1.md")).unwrap().unwrap();
        assert_eq!(file1_record.title, "Updated");
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

    #[test]
    fn test_path_normalization_cross_platform() {
        // Test that paths are stored with forward slashes regardless of platform
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        let repo = FileRepository::new(&conn);

        // Create a file with a subdirectory path
        // On Windows: PathBuf::from("features").join("auth.md") creates "features\auth.md"
        // On Unix: PathBuf::from("features").join("auth.md") creates "features/auth.md"
        let path = PathBuf::from("features").join("auth.md");
        let mut file = create_test_file("test.md");
        file.path = path.clone();

        repo.insert(&file).unwrap();

        // Verify the path is stored with forward slashes in the database
        let stored_path: String = conn
            .query_row(
                "SELECT path FROM files WHERE file_id = ?1",
                [&file.id],
                |row| row.get(0),
            )
            .unwrap();

        // Path should always be stored with forward slashes, regardless of platform
        assert_eq!(stored_path, "features/auth.md");

        // Verify we can retrieve the file using the normalized path
        let retrieved = repo.get_by_path(&path).unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.file_id, "test.md");
    }

    #[test]
    fn test_normalize_path_for_db_unix_style() {
        // Test that Unix-style paths remain unchanged
        let path = Path::new("features/auth.md");
        let normalized = normalize_path_for_db(path);
        assert_eq!(normalized, "features/auth.md");
    }

    #[test]
    fn test_normalize_path_for_db_windows_style() {
        // Test that Windows-style paths are converted to Unix-style
        // This test works on all platforms - we're just testing the string replacement logic
        #[cfg(windows)]
        {
            let path = PathBuf::from("features").join("auth.md");
            let normalized = normalize_path_for_db(&path);
            assert_eq!(normalized, "features/auth.md");
        }

        #[cfg(not(windows))]
        {
            // On Unix, PathBuf::from("features").join("auth.md") creates "features/auth.md"
            let path = PathBuf::from("features").join("auth.md");
            let normalized = normalize_path_for_db(&path);
            assert_eq!(normalized, "features/auth.md");
        }
    }
}
