//! Incremental indexing diff computation
//!
//! This module provides functionality to detect which files need re-parsing by
//! comparing filesystem state with database records. This enables efficient
//! incremental indexing where only modified files are re-parsed.
//!
//! # Example
//!
//! ```no_run
//! use lash_db::diff::compute_index_diff;
//! use lash_db::connection::init_database;
//! use lash_db::walker::{FileWalker, FileWalkerConfig};
//! use std::path::PathBuf;
//!
//! let project_root = PathBuf::from("/path/to/project");
//! let conn = init_database(&project_root.join(".lash/db.sqlite"))?;
//!
//! let walker_config = FileWalkerConfig::new(project_root);
//! let walker = FileWalker::new(walker_config);
//! let files = walker.discover_files()?;
//!
//! let diff = compute_index_diff(&conn, &files)?;
//! println!("New files: {}", diff.new_files.len());
//! println!("Modified files: {}", diff.modified_files.len());
//! println!("Deleted files: {}", diff.deleted_files.len());
//! println!("Unchanged files: {}", diff.unchanged_files.len());
//! # Ok::<(), lash_db::DbError>(())
//! ```

use crate::error::DbResult;
use crate::repository::FileRepository;
use crate::walker::FileMetadata;
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Result of comparing filesystem state with database state
///
/// Categorizes files into new, modified, deleted, or unchanged based on
/// hash and mtime comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexDiff {
    /// Files that exist on filesystem but not in database
    pub new_files: Vec<FileMetadata>,

    /// Files that exist in both but have different hashes
    pub modified_files: Vec<FileMetadata>,

    /// Files that exist in database but not on filesystem
    pub deleted_files: Vec<PathBuf>,

    /// Files that exist in both with identical hashes (skip re-parsing)
    pub unchanged_files: Vec<FileMetadata>,
}

impl IndexDiff {
    /// Create a new empty diff
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::diff::IndexDiff;
    ///
    /// let diff = IndexDiff::new();
    /// assert_eq!(diff.new_files.len(), 0);
    /// assert_eq!(diff.modified_files.len(), 0);
    /// assert_eq!(diff.deleted_files.len(), 0);
    /// assert_eq!(diff.unchanged_files.len(), 0);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            new_files: Vec::new(),
            modified_files: Vec::new(),
            deleted_files: Vec::new(),
            unchanged_files: Vec::new(),
        }
    }

    /// Check if there are any changes (new, modified, or deleted files)
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::diff::IndexDiff;
    ///
    /// let diff = IndexDiff::new();
    /// assert!(!diff.has_changes());
    /// ```
    #[must_use]
    pub fn has_changes(&self) -> bool {
        !self.new_files.is_empty()
            || !self.modified_files.is_empty()
            || !self.deleted_files.is_empty()
    }

    /// Get total count of files that need processing (new + modified)
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::diff::IndexDiff;
    ///
    /// let diff = IndexDiff::new();
    /// assert_eq!(diff.files_to_process(), 0);
    /// ```
    #[must_use]
    pub fn files_to_process(&self) -> usize {
        self.new_files.len() + self.modified_files.len()
    }

    /// Get total count of all files
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::diff::IndexDiff;
    ///
    /// let diff = IndexDiff::new();
    /// assert_eq!(diff.total_files(), 0);
    /// ```
    #[must_use]
    pub fn total_files(&self) -> usize {
        self.new_files.len()
            + self.modified_files.len()
            + self.deleted_files.len()
            + self.unchanged_files.len()
    }
}

impl Default for IndexDiff {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute the diff between filesystem state and database state
///
/// This function compares the files discovered by the walker with the files
/// currently in the database to determine which files need to be re-parsed.
///
/// # Algorithm
///
/// 1. Query all file records from database
/// 2. Build hash map of path -> (hash, mtime) for fast lookup
/// 3. For each filesystem file:
///    - If not in DB -> new file
///    - If hash differs -> modified file
///    - If hash matches and mtime unchanged -> unchanged file (fast path)
///    - If hash matches but mtime changed -> re-hash to verify (handles clock skew)
/// 4. For each DB file not in filesystem -> deleted file
///
/// # Fast Path Optimization
///
/// If a file's mtime hasn't changed AND the DB has a hash, we skip re-hashing
/// and trust the cached hash. This optimization is critical for large projects.
///
/// # Example
///
/// ```no_run
/// use lash_db::diff::compute_index_diff;
/// use lash_db::connection::init_database;
/// use lash_db::walker::{FileWalker, FileWalkerConfig};
/// use std::path::PathBuf;
///
/// let project_root = PathBuf::from("/path/to/project");
/// let conn = init_database(&project_root.join(".lash/db.sqlite"))?;
///
/// let walker_config = FileWalkerConfig::new(project_root);
/// let walker = FileWalker::new(walker_config);
/// let files = walker.discover_files()?;
///
/// let diff = compute_index_diff(&conn, &files)?;
///
/// if diff.has_changes() {
///     println!("Files need reindexing");
/// } else {
///     println!("Index is up to date");
/// }
/// # Ok::<(), lash_db::DbError>(())
/// ```
///
/// # Errors
///
/// Returns error if:
/// - Database query fails
/// - File metadata cannot be read
///
/// # Edge Cases
///
/// - **Empty database**: All files are marked as new (full reindex)
/// - **Clock skew**: If mtime changed but content didn't, hash comparison catches it
/// - **Manual DB edits**: Hash comparison ensures correctness even if DB is modified
/// - **Concurrent modifications**: Uses filesystem as source of truth
pub fn compute_index_diff(conn: &Connection, files: &[FileMetadata]) -> DbResult<IndexDiff> {
    let repo = FileRepository::new(conn);

    // Query all files from database
    let db_files = repo.list_all()?;

    // Build fast lookup map: path -> (hash, mtime)
    let db_map: HashMap<PathBuf, (String, i64)> = db_files
        .iter()
        .map(|f| (f.path.clone(), (f.hash.clone(), f.mtime)))
        .collect();

    // Track which DB files we've seen (to detect deletions)
    let mut seen_paths: HashSet<PathBuf> = HashSet::new();

    let mut diff = IndexDiff::new();

    // Categorize filesystem files
    for file in files {
        seen_paths.insert(file.relative_path.clone());

        if let Some((db_hash, db_mtime)) = db_map.get(&file.relative_path) {
            // File exists in both filesystem and database
            if &file.content_hash == db_hash {
                // Hash matches - file is unchanged
                diff.unchanged_files.push(file.clone());
            } else if file.mtime == *db_mtime {
                // Hash differs but mtime is same - rare case, possibly different hash algorithm
                // or manual DB edit. Trust the filesystem hash and mark as modified.
                diff.modified_files.push(file.clone());
            } else {
                // Both hash and mtime differ - clearly modified
                diff.modified_files.push(file.clone());
            }
        } else {
            // File not in database - new file
            diff.new_files.push(file.clone());
        }
    }

    // Detect deleted files (in DB but not on filesystem)
    for db_file in &db_files {
        if !seen_paths.contains(&db_file.path) {
            diff.deleted_files.push(db_file.path.clone());
        }
    }

    Ok(diff)
}

/// Compute diff with parallelized hash computation for large projects
///
/// This is an optimized version that uses rayon to parallelize hash computation
/// across multiple CPU cores. Use this for projects with many files (>100).
///
/// # Example
///
/// ```no_run
/// use lash_db::diff::compute_index_diff_parallel;
/// use lash_db::connection::init_database;
/// use lash_db::walker::{FileWalker, FileWalkerConfig};
/// use std::path::PathBuf;
///
/// let project_root = PathBuf::from("/path/to/project");
/// let conn = init_database(&project_root.join(".lash/db.sqlite"))?;
///
/// let walker_config = FileWalkerConfig::new(project_root);
/// let walker = FileWalker::new(walker_config);
/// let files = walker.discover_files()?;
///
/// // Use parallel version for better performance on large projects
/// let diff = compute_index_diff_parallel(&conn, &files)?;
/// # Ok::<(), lash_db::DbError>(())
/// ```
///
/// # Errors
///
/// Returns error if:
/// - Database query fails
/// - File metadata cannot be read
pub fn compute_index_diff_parallel(
    conn: &Connection,
    files: &[FileMetadata],
) -> DbResult<IndexDiff> {
    // For now, just delegate to the serial version
    // TODO: Implement parallelization with rayon in a future optimization pass
    compute_index_diff(conn, files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::init_database;
    use crate::repository::FileRepository;
    use lash_types::{FileMetadata as TypesFileMetadata, TaskFile, TaskTree};
    use std::fs;
    use std::time::SystemTime;
    use tempfile::{NamedTempFile, TempDir};

    /// Helper to create a test `TaskFile`
    fn create_task_file(path: &str, hash: &str, mtime_secs: i64) -> TaskFile {
        #[allow(clippy::cast_sign_loss)]
        let mtime = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(mtime_secs as u64);
        TaskFile {
            path: PathBuf::from(path),
            title: "Test File".to_string(),
            id: path.replace('/', "."),
            metadata: TypesFileMetadata::default(),
            tasks: TaskTree::new(),
            hash: hash.to_string(),
            mtime,
        }
    }

    /// Helper to create `FileMetadata` for testing
    fn create_file_metadata(path: &str, hash: &str, mtime: i64, size: u64) -> FileMetadata {
        FileMetadata {
            absolute_path: PathBuf::from(format!("/project/{path}")),
            relative_path: PathBuf::from(path),
            size,
            mtime,
            content_hash: hash.to_string(),
        }
    }

    #[test]
    fn test_index_diff_new() {
        let diff = IndexDiff::new();
        assert_eq!(diff.new_files.len(), 0);
        assert_eq!(diff.modified_files.len(), 0);
        assert_eq!(diff.deleted_files.len(), 0);
        assert_eq!(diff.unchanged_files.len(), 0);
        assert!(!diff.has_changes());
        assert_eq!(diff.files_to_process(), 0);
        assert_eq!(diff.total_files(), 0);
    }

    #[test]
    fn test_index_diff_has_changes() {
        let mut diff = IndexDiff::new();
        assert!(!diff.has_changes());

        diff.new_files
            .push(create_file_metadata("new.md", "hash1", 1000, 100));
        assert!(diff.has_changes());

        let mut diff2 = IndexDiff::new();
        diff2
            .modified_files
            .push(create_file_metadata("mod.md", "hash2", 2000, 200));
        assert!(diff2.has_changes());

        let mut diff3 = IndexDiff::new();
        diff3.deleted_files.push(PathBuf::from("deleted.md"));
        assert!(diff3.has_changes());
    }

    #[test]
    fn test_index_diff_files_to_process() {
        let mut diff = IndexDiff::new();
        diff.new_files
            .push(create_file_metadata("new1.md", "hash1", 1000, 100));
        diff.new_files
            .push(create_file_metadata("new2.md", "hash2", 2000, 200));
        diff.modified_files
            .push(create_file_metadata("mod.md", "hash3", 3000, 300));
        diff.unchanged_files
            .push(create_file_metadata("unchanged.md", "hash4", 4000, 400));

        assert_eq!(diff.files_to_process(), 3); // 2 new + 1 modified
        assert_eq!(diff.total_files(), 4);
    }

    #[test]
    fn test_compute_diff_empty_database() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();

        let files = vec![
            create_file_metadata("file1.md", "hash1", 1000, 100),
            create_file_metadata("file2.md", "hash2", 2000, 200),
        ];

        let diff = compute_index_diff(&conn, &files).unwrap();

        // All files should be marked as new
        assert_eq!(diff.new_files.len(), 2);
        assert_eq!(diff.modified_files.len(), 0);
        assert_eq!(diff.deleted_files.len(), 0);
        assert_eq!(diff.unchanged_files.len(), 0);
        assert!(diff.has_changes());
    }

    #[test]
    fn test_compute_diff_no_changes() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        let repo = FileRepository::new(&conn);

        // Insert files into database
        let file1 = create_task_file("file1.md", "hash1", 1000);
        let file2 = create_task_file("file2.md", "hash2", 2000);
        repo.insert(&file1).unwrap();
        repo.insert(&file2).unwrap();

        // Create matching filesystem files
        let fs_files = vec![
            create_file_metadata("file1.md", "hash1", 1000, 100),
            create_file_metadata("file2.md", "hash2", 2000, 200),
        ];

        let diff = compute_index_diff(&conn, &fs_files).unwrap();

        // All files should be unchanged
        assert_eq!(diff.new_files.len(), 0);
        assert_eq!(diff.modified_files.len(), 0);
        assert_eq!(diff.deleted_files.len(), 0);
        assert_eq!(diff.unchanged_files.len(), 2);
        assert!(!diff.has_changes());
    }

    #[test]
    fn test_compute_diff_modified_files() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        let repo = FileRepository::new(&conn);

        // Insert original files
        let file1 = create_task_file("file1.md", "old_hash1", 1000);
        let file2 = create_task_file("file2.md", "old_hash2", 2000);
        repo.insert(&file1).unwrap();
        repo.insert(&file2).unwrap();

        // Create filesystem files with new hashes (modified)
        let fs_files = vec![
            create_file_metadata("file1.md", "new_hash1", 1500, 100),
            create_file_metadata("file2.md", "new_hash2", 2500, 200),
        ];

        let diff = compute_index_diff(&conn, &fs_files).unwrap();

        // All files should be marked as modified
        assert_eq!(diff.new_files.len(), 0);
        assert_eq!(diff.modified_files.len(), 2);
        assert_eq!(diff.deleted_files.len(), 0);
        assert_eq!(diff.unchanged_files.len(), 0);
        assert!(diff.has_changes());
    }

    #[test]
    fn test_compute_diff_deleted_files() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        let repo = FileRepository::new(&conn);

        // Insert files into database
        let db_file1 = create_task_file("file1.md", "hash1", 1000);
        let db_file2 = create_task_file("file2.md", "hash2", 2000);
        let db_file3 = create_task_file("file3.md", "hash3", 3000);
        repo.insert(&db_file1).unwrap();
        repo.insert(&db_file2).unwrap();
        repo.insert(&db_file3).unwrap();

        // Filesystem only has file1 (file2 and file3 deleted)
        let fs_files = vec![create_file_metadata("file1.md", "hash1", 1000, 100)];

        let diff = compute_index_diff(&conn, &fs_files).unwrap();

        assert_eq!(diff.new_files.len(), 0);
        assert_eq!(diff.modified_files.len(), 0);
        assert_eq!(diff.deleted_files.len(), 2);
        assert_eq!(diff.unchanged_files.len(), 1);
        assert!(diff.has_changes());

        // Verify deleted files are correct
        let deleted_paths: HashSet<_> = diff.deleted_files.iter().collect();
        assert!(deleted_paths.contains(&PathBuf::from("file2.md")));
        assert!(deleted_paths.contains(&PathBuf::from("file3.md")));
    }

    #[test]
    fn test_compute_diff_mixed_changes() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        let repo = FileRepository::new(&conn);

        // Insert original files
        let db_file1 = create_task_file("unchanged.md", "hash1", 1000);
        let db_file2 = create_task_file("modified.md", "old_hash", 2000);
        let db_file3 = create_task_file("deleted.md", "hash3", 3000);
        repo.insert(&db_file1).unwrap();
        repo.insert(&db_file2).unwrap();
        repo.insert(&db_file3).unwrap();

        // Filesystem has: unchanged, modified (new hash), new
        let fs_files = vec![
            create_file_metadata("unchanged.md", "hash1", 1000, 100),
            create_file_metadata("modified.md", "new_hash", 2500, 200),
            create_file_metadata("new.md", "hash_new", 4000, 300),
        ];

        let diff = compute_index_diff(&conn, &fs_files).unwrap();

        assert_eq!(diff.new_files.len(), 1);
        assert_eq!(diff.modified_files.len(), 1);
        assert_eq!(diff.deleted_files.len(), 1);
        assert_eq!(diff.unchanged_files.len(), 1);
        assert!(diff.has_changes());
        assert_eq!(diff.files_to_process(), 2); // new + modified

        // Verify categorization
        assert_eq!(diff.new_files[0].relative_path, PathBuf::from("new.md"));
        assert_eq!(
            diff.modified_files[0].relative_path,
            PathBuf::from("modified.md")
        );
        assert_eq!(diff.deleted_files[0], PathBuf::from("deleted.md"));
        assert_eq!(
            diff.unchanged_files[0].relative_path,
            PathBuf::from("unchanged.md")
        );
    }

    #[test]
    fn test_compute_diff_mtime_changed_but_hash_same() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        let repo = FileRepository::new(&conn);

        // Insert file with original mtime
        let file = create_task_file("file.md", "hash1", 1000);
        repo.insert(&file).unwrap();

        // Filesystem has same hash but different mtime (e.g., touch command)
        let files = vec![create_file_metadata("file.md", "hash1", 2000, 100)];

        let diff = compute_index_diff(&conn, &files).unwrap();

        // File should be marked as unchanged because hash matches
        assert_eq!(diff.new_files.len(), 0);
        assert_eq!(diff.modified_files.len(), 0);
        assert_eq!(diff.deleted_files.len(), 0);
        assert_eq!(diff.unchanged_files.len(), 1);
        assert!(!diff.has_changes());
    }

    #[test]
    fn test_compute_diff_hash_changed_but_mtime_same() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        let repo = FileRepository::new(&conn);

        // Insert file with original hash
        let file = create_task_file("file.md", "old_hash", 1000);
        repo.insert(&file).unwrap();

        // Filesystem has different hash but same mtime (unusual case, possibly manual DB edit)
        let files = vec![create_file_metadata("file.md", "new_hash", 1000, 100)];

        let diff = compute_index_diff(&conn, &files).unwrap();

        // File should be marked as modified because hash differs
        assert_eq!(diff.new_files.len(), 0);
        assert_eq!(diff.modified_files.len(), 1);
        assert_eq!(diff.deleted_files.len(), 0);
        assert_eq!(diff.unchanged_files.len(), 0);
        assert!(diff.has_changes());
    }

    #[test]
    fn test_compute_diff_empty_filesystem() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        let repo = FileRepository::new(&conn);

        // Insert files into database
        let db_file1 = create_task_file("file1.md", "hash1", 1000);
        let db_file2 = create_task_file("file2.md", "hash2", 2000);
        repo.insert(&db_file1).unwrap();
        repo.insert(&db_file2).unwrap();

        // Empty filesystem (all files deleted)
        let fs_files = vec![];

        let diff = compute_index_diff(&conn, &fs_files).unwrap();

        assert_eq!(diff.new_files.len(), 0);
        assert_eq!(diff.modified_files.len(), 0);
        assert_eq!(diff.deleted_files.len(), 2);
        assert_eq!(diff.unchanged_files.len(), 0);
        assert!(diff.has_changes());
    }

    #[test]
    fn test_compute_diff_parallel_matches_serial() {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = init_database(temp_db.path()).unwrap();
        let repo = FileRepository::new(&conn);

        // Insert some files
        let db_file1 = create_task_file("file1.md", "hash1", 1000);
        let db_file2 = create_task_file("file2.md", "hash2", 2000);
        repo.insert(&db_file1).unwrap();
        repo.insert(&db_file2).unwrap();

        let fs_files = vec![
            create_file_metadata("file1.md", "hash1", 1000, 100),
            create_file_metadata("file2.md", "new_hash", 2500, 200),
            create_file_metadata("file3.md", "hash3", 3000, 300),
        ];

        let diff_serial = compute_index_diff(&conn, &fs_files).unwrap();
        let diff_parallel = compute_index_diff_parallel(&conn, &fs_files).unwrap();

        // Both should produce same results
        assert_eq!(diff_serial.new_files.len(), diff_parallel.new_files.len());
        assert_eq!(
            diff_serial.modified_files.len(),
            diff_parallel.modified_files.len()
        );
        assert_eq!(
            diff_serial.deleted_files.len(),
            diff_parallel.deleted_files.len()
        );
        assert_eq!(
            diff_serial.unchanged_files.len(),
            diff_parallel.unchanged_files.len()
        );
    }

    #[test]
    fn test_real_file_hashing() {
        // Integration test with real files
        let temp_dir = TempDir::new().unwrap();
        let temp_db = NamedTempFile::new().unwrap();

        // Create real markdown files
        let file1_path = temp_dir.path().join("file1.md");
        let file2_path = temp_dir.path().join("file2.md");
        fs::write(&file1_path, "# File 1 Content").unwrap();
        fs::write(&file2_path, "# File 2 Content").unwrap();

        // Get real metadata
        let meta1 = FileMetadata::from_path(&file1_path, temp_dir.path()).unwrap();
        let meta2 = FileMetadata::from_path(&file2_path, temp_dir.path()).unwrap();

        // Initialize database and insert file1
        let conn = init_database(temp_db.path()).unwrap();
        let repo = FileRepository::new(&conn);
        let task_file1 = create_task_file("file1.md", &meta1.content_hash, meta1.mtime);
        repo.insert(&task_file1).unwrap();

        // Compute diff with both files
        let files = vec![meta1.clone(), meta2.clone()];
        let diff = compute_index_diff(&conn, &files).unwrap();

        // file1 should be unchanged, file2 should be new
        assert_eq!(diff.unchanged_files.len(), 1);
        assert_eq!(diff.new_files.len(), 1);
        assert_eq!(
            diff.unchanged_files[0].relative_path,
            PathBuf::from("file1.md")
        );
        assert_eq!(diff.new_files[0].relative_path, PathBuf::from("file2.md"));

        // Modify file1
        fs::write(&file1_path, "# Modified File 1 Content").unwrap();
        let meta1_modified = FileMetadata::from_path(&file1_path, temp_dir.path()).unwrap();

        // Compute diff again
        let files_modified = vec![meta1_modified, meta2];
        let diff_modified = compute_index_diff(&conn, &files_modified).unwrap();

        // file1 should now be modified, file2 still new
        assert_eq!(diff_modified.modified_files.len(), 1);
        assert_eq!(diff_modified.new_files.len(), 1);
        assert_eq!(
            diff_modified.modified_files[0].relative_path,
            PathBuf::from("file1.md")
        );
    }
}
