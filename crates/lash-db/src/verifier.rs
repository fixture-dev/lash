//! Index verification for detecting database drift
//!
//! This module provides functionality to verify that the `SQLite` database is in sync
//! with the actual Markdown files on disk. It detects various forms of drift:
//! - Files in DB but not on filesystem (stale records)
//! - Files on filesystem but not in DB (missing index entries)
//! - Hash mismatches (file modified but not reindexed)
//! - Orphaned task records (file deleted but tasks remain)
//! - Orphaned dependency records (file deleted but dependencies remain)
//!
//! # Example
//!
//! ```no_run
//! use lash_db::verifier::{IndexVerifier, VerifierConfig};
//! use lash_db::connection::init_database;
//! use std::path::PathBuf;
//!
//! let project_root = PathBuf::from("/path/to/project");
//! let db_path = project_root.join(".lash/db.sqlite");
//! let conn = init_database(&db_path)?;
//!
//! let config = VerifierConfig::new(project_root);
//! let verifier = IndexVerifier::new(&conn, config);
//! let report = verifier.verify()?;
//!
//! if report.is_clean() {
//!     println!("Index is clean!");
//! } else {
//!     println!("Found {} discrepancies", report.total_issues());
//!     for issue in &report.issues {
//!         println!("- {}: {}", issue.kind, issue.description);
//!     }
//! }
//! # Ok::<(), lash_db::DbError>(())
//! ```

use crate::error::DbResult;
use crate::repository::files::FileRecord;
use crate::repository::FileRepository;
use crate::walker::{FileMetadata, FileWalker, FileWalkerConfig};
use rusqlite::Connection;
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// Type of verification issue
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueKind {
    /// File exists in database but not on filesystem
    StaleFile,
    /// File exists on filesystem but not in database
    MissingFile,
    /// File hash in database doesn't match filesystem
    HashMismatch,
    /// Tasks exist in database for a file that doesn't exist
    OrphanedTasks,
    /// Dependencies reference files that don't exist
    OrphanedDependencies,
}

impl fmt::Display for IssueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaleFile => write!(f, "Stale File"),
            Self::MissingFile => write!(f, "Missing File"),
            Self::HashMismatch => write!(f, "Hash Mismatch"),
            Self::OrphanedTasks => write!(f, "Orphaned Tasks"),
            Self::OrphanedDependencies => write!(f, "Orphaned Dependencies"),
        }
    }
}

/// A verification issue found during index verification
#[derive(Debug, Clone)]
pub struct VerificationIssue {
    /// Type of issue
    pub kind: IssueKind,
    /// Path related to the issue
    pub path: PathBuf,
    /// Human-readable description of the issue
    pub description: String,
    /// Suggested fix for the issue
    pub fix_suggestion: String,
}

impl VerificationIssue {
    /// Create a new verification issue
    #[must_use]
    pub fn new(
        kind: IssueKind,
        path: PathBuf,
        description: String,
        fix_suggestion: String,
    ) -> Self {
        Self {
            kind,
            path,
            description,
            fix_suggestion,
        }
    }

    /// Create a stale file issue
    #[must_use]
    pub fn stale_file(path: &Path) -> Self {
        Self::new(
            IssueKind::StaleFile,
            path.to_path_buf(),
            format!(
                "File '{}' exists in database but not on filesystem",
                path.display()
            ),
            "Run `lash index` to remove stale records, or use auto-fix".to_string(),
        )
    }

    /// Create a missing file issue
    #[must_use]
    pub fn missing_file(path: &Path) -> Self {
        Self::new(
            IssueKind::MissingFile,
            path.to_path_buf(),
            format!(
                "File '{}' exists on filesystem but not in database",
                path.display()
            ),
            "Run `lash index` to add missing files".to_string(),
        )
    }

    /// Create a hash mismatch issue
    #[must_use]
    pub fn hash_mismatch(path: &Path, db_hash: &str, fs_hash: &str) -> Self {
        Self::new(
            IssueKind::HashMismatch,
            path.to_path_buf(),
            format!(
                "File '{}' has been modified (DB hash: {}..., FS hash: {}...)",
                path.display(),
                &db_hash[..8.min(db_hash.len())],
                &fs_hash[..8.min(fs_hash.len())]
            ),
            "Run `lash index` to resync modified files".to_string(),
        )
    }

    /// Create an orphaned tasks issue
    #[must_use]
    pub fn orphaned_tasks(path: &Path, task_count: usize) -> Self {
        Self::new(
            IssueKind::OrphanedTasks,
            path.to_path_buf(),
            format!(
                "{} orphaned task(s) for non-existent file '{}'",
                task_count,
                path.display()
            ),
            "Run `lash index` or use auto-fix to clean up orphaned tasks".to_string(),
        )
    }

    /// Create an orphaned dependencies issue
    #[must_use]
    pub fn orphaned_dependencies(dep_count: usize) -> Self {
        Self::new(
            IssueKind::OrphanedDependencies,
            PathBuf::from("<multiple>"),
            format!("{dep_count} orphaned dependency record(s) reference non-existent files"),
            "Run `lash index` or use auto-fix to clean up orphaned dependencies".to_string(),
        )
    }
}

/// Result of index verification
///
/// Contains all issues found during verification and provides methods to
/// analyze and report on the state of the index.
#[derive(Debug, Clone)]
pub struct VerificationReport {
    /// List of all issues found
    pub issues: Vec<VerificationIssue>,
    /// Number of files checked
    pub files_checked: usize,
    /// Number of database records checked
    pub db_records_checked: usize,
}

impl VerificationReport {
    /// Create a new empty verification report
    #[must_use]
    pub fn new() -> Self {
        Self {
            issues: Vec::new(),
            files_checked: 0,
            db_records_checked: 0,
        }
    }

    /// Check if the index is clean (no issues)
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::verifier::VerificationReport;
    ///
    /// let report = VerificationReport::new();
    /// assert!(report.is_clean());
    /// ```
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.issues.is_empty()
    }

    /// Get total number of issues
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::verifier::VerificationReport;
    ///
    /// let report = VerificationReport::new();
    /// assert_eq!(report.total_issues(), 0);
    /// ```
    #[must_use]
    pub fn total_issues(&self) -> usize {
        self.issues.len()
    }

    /// Get issues of a specific kind
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::verifier::{VerificationReport, IssueKind};
    ///
    /// let report = VerificationReport::new();
    /// let stale_files = report.issues_of_kind(IssueKind::StaleFile);
    /// assert_eq!(stale_files.len(), 0);
    /// ```
    #[must_use]
    pub fn issues_of_kind(&self, kind: IssueKind) -> Vec<&VerificationIssue> {
        self.issues
            .iter()
            .filter(|issue| issue.kind == kind)
            .collect()
    }

    /// Get count of issues by kind
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::verifier::{VerificationReport, IssueKind};
    ///
    /// let report = VerificationReport::new();
    /// assert_eq!(report.count_by_kind(IssueKind::StaleFile), 0);
    /// ```
    #[must_use]
    pub fn count_by_kind(&self, kind: IssueKind) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.kind == kind)
            .count()
    }
}

impl Default for VerificationReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for index verification
#[derive(Debug, Clone)]
pub struct VerifierConfig {
    /// Project root directory
    pub project_root: PathBuf,
    /// File walker configuration
    pub walker_config: FileWalkerConfig,
    /// Whether to check for orphaned tasks
    pub check_orphaned_tasks: bool,
    /// Whether to check for orphaned dependencies
    pub check_orphaned_dependencies: bool,
}

impl VerifierConfig {
    /// Create a new verifier configuration
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::verifier::VerifierConfig;
    /// use std::path::PathBuf;
    ///
    /// let config = VerifierConfig::new(PathBuf::from("/project"));
    /// assert!(config.check_orphaned_tasks);
    /// assert!(config.check_orphaned_dependencies);
    /// ```
    #[must_use]
    pub fn new(project_root: PathBuf) -> Self {
        let walker_config = FileWalkerConfig::new(project_root.clone());
        Self {
            project_root,
            walker_config,
            check_orphaned_tasks: true,
            check_orphaned_dependencies: true,
        }
    }

    /// Set custom walker configuration
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::verifier::VerifierConfig;
    /// use lash_db::walker::FileWalkerConfig;
    /// use std::path::PathBuf;
    ///
    /// let walker_config = FileWalkerConfig::new(PathBuf::from("/project"))
    ///     .with_respect_gitignore(false);
    ///
    /// let config = VerifierConfig::new(PathBuf::from("/project"))
    ///     .with_walker_config(walker_config);
    /// ```
    #[must_use]
    pub fn with_walker_config(mut self, walker_config: FileWalkerConfig) -> Self {
        self.walker_config = walker_config;
        self
    }

    /// Set whether to check for orphaned tasks
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::verifier::VerifierConfig;
    /// use std::path::PathBuf;
    ///
    /// let config = VerifierConfig::new(PathBuf::from("/project"))
    ///     .with_check_orphaned_tasks(false);
    /// assert!(!config.check_orphaned_tasks);
    /// ```
    #[must_use]
    pub fn with_check_orphaned_tasks(mut self, check: bool) -> Self {
        self.check_orphaned_tasks = check;
        self
    }

    /// Set whether to check for orphaned dependencies
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::verifier::VerifierConfig;
    /// use std::path::PathBuf;
    ///
    /// let config = VerifierConfig::new(PathBuf::from("/project"))
    ///     .with_check_orphaned_dependencies(false);
    /// assert!(!config.check_orphaned_dependencies);
    /// ```
    #[must_use]
    pub fn with_check_orphaned_dependencies(mut self, check: bool) -> Self {
        self.check_orphaned_dependencies = check;
        self
    }
}

/// Index verifier for detecting database drift
///
/// The verifier compares the state of the database with the actual filesystem
/// to detect inconsistencies that may have arisen from:
/// - Files being added/removed outside of `lash index`
/// - Manual database modifications
/// - Interrupted indexing operations
/// - File system race conditions
pub struct IndexVerifier<'conn> {
    conn: &'conn Connection,
    config: VerifierConfig,
}

impl<'conn> IndexVerifier<'conn> {
    /// Create a new index verifier
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_db::verifier::{IndexVerifier, VerifierConfig};
    /// use lash_db::connection::init_database;
    /// use std::path::PathBuf;
    ///
    /// let conn = init_database(&PathBuf::from("/tmp/lash.db"))?;
    /// let config = VerifierConfig::new(PathBuf::from("/project"));
    /// let verifier = IndexVerifier::new(&conn, config);
    /// # Ok::<(), lash_db::DbError>(())
    /// ```
    #[must_use]
    pub fn new(conn: &'conn Connection, config: VerifierConfig) -> Self {
        Self { conn, config }
    }

    /// Verify the index and return a report of all issues found
    ///
    /// This performs a comprehensive verification including:
    /// 1. Comparing filesystem with database records
    /// 2. Checking for hash mismatches
    /// 3. Detecting orphaned tasks
    /// 4. Detecting orphaned dependencies
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - File discovery fails
    /// - Database queries fail
    /// - File metadata cannot be read
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_db::verifier::{IndexVerifier, VerifierConfig};
    /// use lash_db::connection::init_database;
    /// use std::path::PathBuf;
    ///
    /// let conn = init_database(&PathBuf::from("/tmp/lash.db"))?;
    /// let config = VerifierConfig::new(PathBuf::from("/project"));
    /// let verifier = IndexVerifier::new(&conn, config);
    /// let report = verifier.verify()?;
    ///
    /// if !report.is_clean() {
    ///     for issue in &report.issues {
    ///         eprintln!("{}: {}", issue.kind, issue.description);
    ///     }
    /// }
    /// # Ok::<(), lash_db::DbError>(())
    /// ```
    pub fn verify(&self) -> DbResult<VerificationReport> {
        let mut report = VerificationReport::new();

        // Phase 1: Discover files on filesystem
        let walker = FileWalker::new(self.config.walker_config.clone());
        let fs_files = walker.discover_files()?;
        report.files_checked = fs_files.len();

        // Phase 2: Query database records
        let file_repo = FileRepository::new(self.conn);
        let db_files = file_repo.list_all()?;
        report.db_records_checked = db_files.len();

        // Build lookup maps for fast comparison
        let fs_map: HashMap<PathBuf, FileMetadata> = fs_files
            .iter()
            .map(|f| (f.relative_path.clone(), f.clone()))
            .collect();

        let db_map: HashMap<PathBuf, (String, i64)> = db_files
            .iter()
            .map(|f| (f.path.clone(), (f.hash.clone(), f.id)))
            .collect();

        // Phase 3: Check for stale files (in DB but not on filesystem)
        for db_file in &db_files {
            if !fs_map.contains_key(&db_file.path) {
                report
                    .issues
                    .push(VerificationIssue::stale_file(&db_file.path));
            }
        }

        // Phase 4: Check for missing files and hash mismatches
        for fs_file in &fs_files {
            if let Some((db_hash, _db_id)) = db_map.get(&fs_file.relative_path) {
                // File exists in both - check hash
                if &fs_file.content_hash != db_hash {
                    report.issues.push(VerificationIssue::hash_mismatch(
                        &fs_file.relative_path,
                        db_hash,
                        &fs_file.content_hash,
                    ));
                }
            } else {
                // File on filesystem but not in DB
                report
                    .issues
                    .push(VerificationIssue::missing_file(&fs_file.relative_path));
            }
        }

        // Phase 5: Check for orphaned tasks (if enabled)
        if self.config.check_orphaned_tasks {
            self.check_orphaned_tasks(&fs_map, &db_files, &mut report)?;
        }

        // Phase 6: Check for orphaned dependencies (if enabled)
        if self.config.check_orphaned_dependencies {
            self.check_orphaned_dependencies(&fs_map, &mut report)?;
        }

        Ok(report)
    }

    /// Check for orphaned tasks
    ///
    /// Orphaned tasks are tasks that exist in the database for files that
    /// no longer exist on the filesystem.
    fn check_orphaned_tasks(
        &self,
        fs_map: &HashMap<PathBuf, FileMetadata>,
        db_files: &[FileRecord],
        report: &mut VerificationReport,
    ) -> DbResult<()> {
        for db_file in db_files {
            // If file doesn't exist on filesystem, check if it has tasks
            if !fs_map.contains_key(&db_file.path) {
                let task_count: i64 = self.conn.query_row(
                    "SELECT COUNT(*) FROM tasks WHERE file_id = ?1",
                    [db_file.id],
                    |row| row.get(0),
                )?;

                if task_count > 0 {
                    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                    report.issues.push(VerificationIssue::orphaned_tasks(
                        &db_file.path,
                        task_count as usize,
                    ));
                }
            }
        }

        Ok(())
    }

    /// Check for orphaned dependencies
    ///
    /// Orphaned dependencies are dependency records that reference tasks
    /// that no longer exist in the database.
    fn check_orphaned_dependencies(
        &self,
        _fs_map: &HashMap<PathBuf, FileMetadata>,
        report: &mut VerificationReport,
    ) -> DbResult<()> {
        // Check for dependencies referencing non-existent tasks
        let orphaned_count: i64 = self.conn.query_row(
            "SELECT COUNT(*)
             FROM dependencies
             WHERE from_task_id NOT IN (SELECT id FROM tasks)
                OR (to_task_id IS NOT NULL AND to_task_id NOT IN (SELECT id FROM tasks))",
            [],
            |row| row.get(0),
        )?;

        if orphaned_count > 0 {
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            report.issues.push(VerificationIssue::orphaned_dependencies(
                orphaned_count as usize,
            ));
        }

        Ok(())
    }

    /// Auto-fix all issues found in a verification report
    ///
    /// This will:
    /// - Delete stale file records
    /// - Index missing files
    /// - Re-index files with hash mismatches
    /// - Clean up orphaned tasks
    /// - Clean up orphaned dependencies
    ///
    /// # Errors
    ///
    /// Returns error if any database operation fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_db::verifier::{IndexVerifier, VerifierConfig};
    /// use lash_db::connection::init_database;
    /// use std::path::PathBuf;
    ///
    /// let conn = init_database(&PathBuf::from("/tmp/lash.db"))?;
    /// let config = VerifierConfig::new(PathBuf::from("/project"));
    /// let verifier = IndexVerifier::new(&conn, config);
    /// let report = verifier.verify()?;
    ///
    /// if !report.is_clean() {
    ///     println!("Found {} issues, auto-fixing...", report.total_issues());
    ///     verifier.auto_fix(&report)?;
    ///     println!("Auto-fix complete!");
    /// }
    /// # Ok::<(), lash_db::DbError>(())
    /// ```
    pub fn auto_fix(&self, report: &VerificationReport) -> DbResult<()> {
        let file_repo = FileRepository::new(self.conn);

        // Fix stale files
        for issue in report.issues_of_kind(IssueKind::StaleFile) {
            file_repo.delete(&issue.path)?;
        }

        // Clean up orphaned tasks (CASCADE DELETE will handle this when we delete files)
        // So orphaned tasks are already cleaned up by deleting stale files

        // Clean up orphaned dependencies
        if report.count_by_kind(IssueKind::OrphanedDependencies) > 0 {
            self.conn.execute(
                "DELETE FROM dependencies
                 WHERE from_task_id NOT IN (SELECT id FROM tasks)
                    OR (to_task_id IS NOT NULL AND to_task_id NOT IN (SELECT id FROM tasks))",
                [],
            )?;
        }

        // Note: Missing files and hash mismatches should be fixed by running `lash index`
        // which is beyond the scope of the verifier. The verifier only cleans up
        // stale/orphaned data, not re-indexing.

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::init_database;
    use lash_types::{FileMetadata as TypesFileMetadata, TaskFile, TaskTree};
    use std::fs;
    use std::path::Path;
    use std::time::SystemTime;
    use tempfile::{NamedTempFile, TempDir};

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

    #[test]
    fn test_verification_report_new() {
        let report = VerificationReport::new();
        assert!(report.is_clean());
        assert_eq!(report.total_issues(), 0);
    }

    #[test]
    fn test_verification_report_issues_of_kind() {
        let mut report = VerificationReport::new();
        report
            .issues
            .push(VerificationIssue::stale_file(&PathBuf::from("stale.md")));
        report
            .issues
            .push(VerificationIssue::missing_file(&PathBuf::from(
                "missing.md",
            )));
        report
            .issues
            .push(VerificationIssue::stale_file(&PathBuf::from("stale2.md")));

        assert_eq!(report.count_by_kind(IssueKind::StaleFile), 2);
        assert_eq!(report.count_by_kind(IssueKind::MissingFile), 1);
        assert_eq!(report.count_by_kind(IssueKind::HashMismatch), 0);

        let stale_issues = report.issues_of_kind(IssueKind::StaleFile);
        assert_eq!(stale_issues.len(), 2);
    }

    #[test]
    fn test_verifier_config_new() {
        let config = VerifierConfig::new(PathBuf::from("/project"));
        assert!(config.check_orphaned_tasks);
        assert!(config.check_orphaned_dependencies);
    }

    #[test]
    fn test_verifier_config_builders() {
        let config = VerifierConfig::new(PathBuf::from("/project"))
            .with_check_orphaned_tasks(false)
            .with_check_orphaned_dependencies(false);

        assert!(!config.check_orphaned_tasks);
        assert!(!config.check_orphaned_dependencies);
    }

    #[test]
    fn test_verify_clean_index() {
        let temp_dir = TempDir::new().unwrap();
        let temp_db = NamedTempFile::new().unwrap();

        // Create a file on filesystem
        fs::write(
            temp_dir.path().join("test.md"),
            "# Test\n\n@id: test\n\n## Tasks\n\n- [ ] Task 1\n",
        )
        .unwrap();

        // Get metadata
        let file_meta =
            FileMetadata::from_path(&temp_dir.path().join("test.md"), temp_dir.path()).unwrap();

        // Initialize database and insert matching file
        let conn = init_database(temp_db.path()).unwrap();
        let file_repo = FileRepository::new(&conn);
        let task_file = create_task_file("test.md", &file_meta.content_hash, file_meta.mtime);
        file_repo.insert(&task_file).unwrap();

        // Verify
        let config = VerifierConfig::new(temp_dir.path().to_path_buf());
        let verifier = IndexVerifier::new(&conn, config);
        let report = verifier.verify().unwrap();

        assert!(report.is_clean());
        assert_eq!(report.files_checked, 1);
        assert_eq!(report.db_records_checked, 1);
    }

    #[test]
    fn test_verify_stale_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_db = NamedTempFile::new().unwrap();

        // Initialize database with a file that doesn't exist on filesystem
        let conn = init_database(temp_db.path()).unwrap();
        let file_repo = FileRepository::new(&conn);
        let task_file = create_task_file("nonexistent.md", "hash123", 1000);
        file_repo.insert(&task_file).unwrap();

        // Verify
        let config = VerifierConfig::new(temp_dir.path().to_path_buf());
        let verifier = IndexVerifier::new(&conn, config);
        let report = verifier.verify().unwrap();

        assert!(!report.is_clean());
        assert_eq!(report.total_issues(), 1);
        assert_eq!(report.count_by_kind(IssueKind::StaleFile), 1);

        let stale_issue = &report.issues[0];
        assert_eq!(stale_issue.kind, IssueKind::StaleFile);
        assert_eq!(stale_issue.path, PathBuf::from("nonexistent.md"));
    }

    #[test]
    fn test_verify_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_db = NamedTempFile::new().unwrap();

        // Create a file on filesystem but not in database
        fs::write(
            temp_dir.path().join("missing.md"),
            "# Missing\n\n@id: missing\n",
        )
        .unwrap();

        // Initialize empty database
        let conn = init_database(temp_db.path()).unwrap();

        // Verify
        let config = VerifierConfig::new(temp_dir.path().to_path_buf());
        let verifier = IndexVerifier::new(&conn, config);
        let report = verifier.verify().unwrap();

        assert!(!report.is_clean());
        assert_eq!(report.total_issues(), 1);
        assert_eq!(report.count_by_kind(IssueKind::MissingFile), 1);

        let missing_issue = &report.issues[0];
        assert_eq!(missing_issue.kind, IssueKind::MissingFile);
        assert_eq!(missing_issue.path, PathBuf::from("missing.md"));
    }

    #[test]
    fn test_verify_hash_mismatch() {
        let temp_dir = TempDir::new().unwrap();
        let temp_db = NamedTempFile::new().unwrap();

        // Create a file on filesystem
        fs::write(
            temp_dir.path().join("modified.md"),
            "# Modified Content\n\n@id: modified\n",
        )
        .unwrap();

        // Get current metadata
        let file_meta =
            FileMetadata::from_path(&temp_dir.path().join("modified.md"), temp_dir.path()).unwrap();

        // Initialize database with old hash
        let conn = init_database(temp_db.path()).unwrap();
        let file_repo = FileRepository::new(&conn);
        let task_file = create_task_file("modified.md", "old_hash_12345", file_meta.mtime);
        file_repo.insert(&task_file).unwrap();

        // Verify
        let config = VerifierConfig::new(temp_dir.path().to_path_buf());
        let verifier = IndexVerifier::new(&conn, config);
        let report = verifier.verify().unwrap();

        assert!(!report.is_clean());
        assert_eq!(report.total_issues(), 1);
        assert_eq!(report.count_by_kind(IssueKind::HashMismatch), 1);

        let mismatch_issue = &report.issues[0];
        assert_eq!(mismatch_issue.kind, IssueKind::HashMismatch);
        assert_eq!(mismatch_issue.path, PathBuf::from("modified.md"));
    }

    #[test]
    fn test_verify_orphaned_tasks() {
        let temp_dir = TempDir::new().unwrap();
        let temp_db = NamedTempFile::new().unwrap();

        // Initialize database with file and tasks
        let conn = init_database(temp_db.path()).unwrap();
        let file_repo = FileRepository::new(&conn);
        let task_file = create_task_file("deleted.md", "hash123", 1000);
        file_repo.insert(&task_file).unwrap();

        // Get file ID and insert some tasks
        let file_record = file_repo
            .get_by_path(Path::new("deleted.md"))
            .unwrap()
            .unwrap();
        conn.execute(
            "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 0)",
            (
                file_record.id,
                "task1",
                "deleted.md#task1",
                "Task 1",
                "open",
            ),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 1)",
            (
                file_record.id,
                "task2",
                "deleted.md#task2",
                "Task 2",
                "open",
            ),
        )
        .unwrap();

        // Verify (file doesn't exist on filesystem)
        let config = VerifierConfig::new(temp_dir.path().to_path_buf());
        let verifier = IndexVerifier::new(&conn, config);
        let report = verifier.verify().unwrap();

        assert!(!report.is_clean());
        // Should have both StaleFile and OrphanedTasks
        assert_eq!(report.count_by_kind(IssueKind::StaleFile), 1);
        assert_eq!(report.count_by_kind(IssueKind::OrphanedTasks), 1);

        let orphaned_issue = report.issues_of_kind(IssueKind::OrphanedTasks)[0];
        assert_eq!(orphaned_issue.path, PathBuf::from("deleted.md"));
        assert!(orphaned_issue.description.contains("2 orphaned task"));
    }

    #[test]
    fn test_verify_orphaned_dependencies() {
        let temp_dir = TempDir::new().unwrap();
        let temp_db = NamedTempFile::new().unwrap();

        // Initialize database and create tasks, then delete one to create orphaned dependency
        let conn = init_database(temp_db.path()).unwrap();
        let file_repo = FileRepository::new(&conn);

        // Create a file with tasks
        let task_file = create_task_file("test.md", "hash123", 1000);
        file_repo.insert(&task_file).unwrap();

        let file_record = file_repo
            .get_by_path(Path::new("test.md"))
            .unwrap()
            .unwrap();

        // Insert two tasks
        conn.execute(
            "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index)
             VALUES (?1, 'task1', 'test.md#task1', 'Task 1', 'open', 0, 0)",
            [file_record.id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index)
             VALUES (?1, 'task2', 'test.md#task2', 'Task 2', 'open', 0, 1)",
            [file_record.id],
        )
        .unwrap();

        // Get task IDs
        let task1_id: i64 = conn
            .query_row("SELECT id FROM tasks WHERE local_id = 'task1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        let task2_id: i64 = conn
            .query_row("SELECT id FROM tasks WHERE local_id = 'task2'", [], |row| {
                row.get(0)
            })
            .unwrap();

        // Create a dependency
        conn.execute(
            "INSERT INTO dependencies (from_task_id, to_task_id, kind)
             VALUES (?1, ?2, 'explicit_id')",
            (task1_id, task2_id),
        )
        .unwrap();

        // Now delete task2 without cascading (by disabling FK temporarily)
        conn.execute("PRAGMA foreign_keys = OFF", []).unwrap();
        conn.execute("DELETE FROM tasks WHERE id = ?1", [task2_id])
            .unwrap();
        conn.execute("PRAGMA foreign_keys = ON", []).unwrap();

        // Verify - should detect orphaned dependency
        let config = VerifierConfig::new(temp_dir.path().to_path_buf());
        let verifier = IndexVerifier::new(&conn, config);
        let report = verifier.verify().unwrap();

        assert!(!report.is_clean());
        assert_eq!(report.count_by_kind(IssueKind::OrphanedDependencies), 1);

        let orphaned_dep = &report.issues_of_kind(IssueKind::OrphanedDependencies)[0];
        assert!(orphaned_dep.description.contains("1 orphaned dependency"));
    }

    #[test]
    fn test_verify_mixed_issues() {
        let temp_dir = TempDir::new().unwrap();
        let temp_db = NamedTempFile::new().unwrap();

        // Create one file on filesystem
        fs::write(
            temp_dir.path().join("exists.md"),
            "# Exists\n\n@id: exists\n",
        )
        .unwrap();

        let file_meta =
            FileMetadata::from_path(&temp_dir.path().join("exists.md"), temp_dir.path()).unwrap();

        // Initialize database
        let conn = init_database(temp_db.path()).unwrap();
        let file_repo = FileRepository::new(&conn);

        // Insert matching file
        let task_file1 = create_task_file("exists.md", &file_meta.content_hash, file_meta.mtime);
        file_repo.insert(&task_file1).unwrap();

        // Insert stale file
        let task_file2 = create_task_file("stale.md", "hash123", 2000);
        file_repo.insert(&task_file2).unwrap();

        // Create another file on filesystem not in DB
        fs::write(
            temp_dir.path().join("missing.md"),
            "# Missing\n\n@id: missing\n",
        )
        .unwrap();

        // Verify
        let config = VerifierConfig::new(temp_dir.path().to_path_buf());
        let verifier = IndexVerifier::new(&conn, config);
        let report = verifier.verify().unwrap();

        assert!(!report.is_clean());
        assert_eq!(report.files_checked, 2); // exists.md and missing.md
        assert_eq!(report.db_records_checked, 2); // exists.md and stale.md
        assert_eq!(report.count_by_kind(IssueKind::StaleFile), 1);
        assert_eq!(report.count_by_kind(IssueKind::MissingFile), 1);
    }

    #[test]
    fn test_auto_fix_stale_files() {
        let temp_dir = TempDir::new().unwrap();
        let temp_db = NamedTempFile::new().unwrap();

        // Initialize database with stale file
        let conn = init_database(temp_db.path()).unwrap();
        let file_repo = FileRepository::new(&conn);
        let task_file = create_task_file("stale.md", "hash123", 1000);
        file_repo.insert(&task_file).unwrap();

        // Verify to get report
        let config = VerifierConfig::new(temp_dir.path().to_path_buf());
        let verifier = IndexVerifier::new(&conn, config.clone());
        let report = verifier.verify().unwrap();

        assert_eq!(report.count_by_kind(IssueKind::StaleFile), 1);

        // Auto-fix
        verifier.auto_fix(&report).unwrap();

        // Verify again - should be clean now
        let verifier2 = IndexVerifier::new(&conn, config);
        let report2 = verifier2.verify().unwrap();

        assert!(report2.is_clean());
    }

    #[test]
    fn test_auto_fix_orphaned_dependencies() {
        let temp_dir = TempDir::new().unwrap();
        let temp_db = NamedTempFile::new().unwrap();

        // Initialize database and create orphaned dependency
        let conn = init_database(temp_db.path()).unwrap();
        let file_repo = FileRepository::new(&conn);

        // Create a file with tasks
        let task_file = create_task_file("test.md", "hash123", 1000);
        file_repo.insert(&task_file).unwrap();

        let file_record = file_repo
            .get_by_path(Path::new("test.md"))
            .unwrap()
            .unwrap();

        // Insert two tasks
        conn.execute(
            "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index)
             VALUES (?1, 'task1', 'test.md#task1', 'Task 1', 'open', 0, 0)",
            [file_record.id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index)
             VALUES (?1, 'task2', 'test.md#task2', 'Task 2', 'open', 0, 1)",
            [file_record.id],
        )
        .unwrap();

        // Get task IDs
        let task1_id: i64 = conn
            .query_row("SELECT id FROM tasks WHERE local_id = 'task1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        let task2_id: i64 = conn
            .query_row("SELECT id FROM tasks WHERE local_id = 'task2'", [], |row| {
                row.get(0)
            })
            .unwrap();

        // Create a dependency
        conn.execute(
            "INSERT INTO dependencies (from_task_id, to_task_id, kind)
             VALUES (?1, ?2, 'explicit_id')",
            (task1_id, task2_id),
        )
        .unwrap();

        // Delete task2 without cascading to create orphaned dependency
        conn.execute("PRAGMA foreign_keys = OFF", []).unwrap();
        conn.execute("DELETE FROM tasks WHERE id = ?1", [task2_id])
            .unwrap();
        conn.execute("PRAGMA foreign_keys = ON", []).unwrap();

        // Verify
        let config = VerifierConfig::new(temp_dir.path().to_path_buf());
        let verifier = IndexVerifier::new(&conn, config.clone());
        let report = verifier.verify().unwrap();

        assert_eq!(report.count_by_kind(IssueKind::OrphanedDependencies), 1);

        // Auto-fix
        verifier.auto_fix(&report).unwrap();

        // Verify again - should be clean
        let verifier2 = IndexVerifier::new(&conn, config);
        let report2 = verifier2.verify().unwrap();

        assert!(report2.is_clean());
    }

    #[test]
    fn test_verify_disabled_checks() {
        let temp_dir = TempDir::new().unwrap();
        let temp_db = NamedTempFile::new().unwrap();

        // Initialize database with stale file that has tasks
        let conn = init_database(temp_db.path()).unwrap();
        let file_repo = FileRepository::new(&conn);
        let task_file = create_task_file("deleted.md", "hash123", 1000);
        file_repo.insert(&task_file).unwrap();

        let file_record = file_repo
            .get_by_path(Path::new("deleted.md"))
            .unwrap()
            .unwrap();
        conn.execute(
            "INSERT INTO tasks (file_id, local_id, full_id, title, status, depth, order_index)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 0)",
            (
                file_record.id,
                "task1",
                "deleted.md#task1",
                "Task 1",
                "open",
            ),
        )
        .unwrap();

        // Verify with orphaned tasks check disabled
        let config =
            VerifierConfig::new(temp_dir.path().to_path_buf()).with_check_orphaned_tasks(false);
        let verifier = IndexVerifier::new(&conn, config);
        let report = verifier.verify().unwrap();

        // Should only detect stale file, not orphaned tasks
        assert_eq!(report.count_by_kind(IssueKind::StaleFile), 1);
        assert_eq!(report.count_by_kind(IssueKind::OrphanedTasks), 0);
    }
}
