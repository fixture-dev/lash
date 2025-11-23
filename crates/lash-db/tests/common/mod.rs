//! Common test utilities for lash-db integration tests
//!
//! Provides database inspection utilities, test database creation,
//! and helper functions for testing database operations.

#![allow(dead_code)] // Test helpers may not be used by all test files

use lash_db::{init_database, open_database, DbResult};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Test database wrapper that provides both in-memory and file-based databases
pub struct TestDatabase {
    temp_dir: Option<TempDir>,
    db_path: PathBuf,
}

impl TestDatabase {
    /// Create a new in-memory test database
    ///
    /// This is fast and suitable for unit tests that don't require file persistence.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use common::TestDatabase;
    /// let db = TestDatabase::in_memory();
    /// let conn = db.connect().unwrap();
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the in-memory database cannot be initialized
    #[must_use]
    pub fn in_memory() -> Self {
        let db_path = PathBuf::from(":memory:");
        // init_database opens the connection and initializes it
        let _conn = init_database(&db_path).expect("Failed to initialize in-memory database");

        Self {
            temp_dir: None,
            db_path,
        }
    }

    /// Create a new file-based test database in a temporary directory
    ///
    /// This is suitable for integration tests that need file persistence.
    /// The database file is automatically cleaned up when the `TestDatabase` is dropped.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use common::TestDatabase;
    /// let db = TestDatabase::file_based();
    /// let conn = db.connect().unwrap();
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the temporary directory or database cannot be created
    #[must_use]
    pub fn file_based() -> Self {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
        let db_path = temp_dir.path().join(".lash.db");

        // init_database opens the connection and initializes it
        let _conn = init_database(&db_path).expect("Failed to initialize file database");

        Self {
            temp_dir: Some(temp_dir),
            db_path,
        }
    }

    /// Create a new file-based test database at a specific path
    ///
    /// The database is created at the given path but cleanup is manual.
    ///
    /// # Arguments
    ///
    /// * `path` - Path where the database should be created
    ///
    /// # Panics
    ///
    /// Panics if the database cannot be initialized
    #[must_use]
    pub fn at_path<P: AsRef<Path>>(path: P) -> Self {
        let db_path = path.as_ref().to_path_buf();
        // init_database opens the connection and initializes it
        let _conn = init_database(&db_path).expect("Failed to initialize database");

        Self {
            temp_dir: None,
            db_path,
        }
    }

    /// Get the path to the database file
    ///
    /// For in-memory databases, this returns ":memory:"
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.db_path
    }

    /// Connect to the test database
    ///
    /// For in-memory databases, this creates a new connection (which won't have the schema).
    /// For file-based databases, this opens the existing database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database connection cannot be established
    pub fn connect(&self) -> DbResult<Connection> {
        // For in-memory databases, we need to use init_database to get a schema
        // For file-based databases, we can use open_database
        if self.db_path.to_str() == Some(":memory:") {
            init_database(&self.db_path)
        } else {
            open_database(&self.db_path)
        }
    }

    /// Get a mutable connection to the database
    ///
    /// # Panics
    ///
    /// Panics if the connection cannot be established
    #[must_use]
    pub fn connection(&self) -> Connection {
        self.connect().expect("Failed to connect to test database")
    }
}

/// Database inspection utilities
pub struct DbInspector<'a> {
    conn: &'a Connection,
}

impl<'a> DbInspector<'a> {
    /// Create a new database inspector
    #[must_use]
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Count the number of files in the database
    ///
    /// # Panics
    ///
    /// Panics if the query fails
    #[must_use]
    pub fn count_files(&self) -> usize {
        self.conn
            .query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))
            .expect("Failed to count files")
    }

    /// Count the number of tasks in the database
    ///
    /// # Panics
    ///
    /// Panics if the query fails
    #[must_use]
    pub fn count_tasks(&self) -> usize {
        self.conn
            .query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))
            .expect("Failed to count tasks")
    }

    /// Count the number of labels in the database
    ///
    /// # Panics
    ///
    /// Panics if the query fails
    #[must_use]
    pub fn count_labels(&self) -> usize {
        self.conn
            .query_row("SELECT COUNT(*) FROM labels", [], |row| row.get(0))
            .expect("Failed to count labels")
    }

    /// Count the number of dependencies in the database
    ///
    /// # Panics
    ///
    /// Panics if the query fails
    #[must_use]
    pub fn count_dependencies(&self) -> usize {
        self.conn
            .query_row("SELECT COUNT(*) FROM dependencies", [], |row| row.get(0))
            .expect("Failed to count dependencies")
    }

    /// Get all file paths in the database
    ///
    /// # Panics
    ///
    /// Panics if the query fails
    #[must_use]
    pub fn get_file_paths(&self) -> Vec<String> {
        let mut stmt = self
            .conn
            .prepare("SELECT path FROM files ORDER BY path")
            .expect("Failed to prepare statement");

        let paths = stmt
            .query_map([], |row| row.get(0))
            .expect("Failed to query file paths")
            .collect::<Result<Vec<String>, _>>()
            .expect("Failed to collect file paths");

        paths
    }

    /// Get all task IDs in the database
    ///
    /// # Panics
    ///
    /// Panics if the query fails
    #[must_use]
    pub fn get_task_ids(&self) -> Vec<String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM tasks ORDER BY id")
            .expect("Failed to prepare statement");

        let ids = stmt
            .query_map([], |row| row.get(0))
            .expect("Failed to query task IDs")
            .collect::<Result<Vec<String>, _>>()
            .expect("Failed to collect task IDs");

        ids
    }

    /// Get all label names in the database
    ///
    /// # Panics
    ///
    /// Panics if the query fails
    #[must_use]
    pub fn get_labels(&self) -> Vec<String> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT name FROM labels ORDER BY name")
            .expect("Failed to prepare statement");

        let labels = stmt
            .query_map([], |row| row.get(0))
            .expect("Failed to query labels")
            .collect::<Result<Vec<String>, _>>()
            .expect("Failed to collect labels");

        labels
    }

    /// Check if a specific file exists in the database
    ///
    /// # Arguments
    ///
    /// * `path` - File path to check
    ///
    /// # Panics
    ///
    /// Panics if the query fails
    #[must_use]
    pub fn has_file(&self, path: &str) -> bool {
        self.conn
            .query_row("SELECT COUNT(*) FROM files WHERE path = ?", [path], |row| {
                row.get::<_, i64>(0)
            })
            .expect("Failed to check file existence")
            > 0
    }

    /// Check if a specific task exists in the database
    ///
    /// # Arguments
    ///
    /// * `id` - Task ID to check
    ///
    /// # Panics
    ///
    /// Panics if the query fails
    #[must_use]
    pub fn has_task(&self, id: &str) -> bool {
        self.conn
            .query_row("SELECT COUNT(*) FROM tasks WHERE id = ?", [id], |row| {
                row.get::<_, i64>(0)
            })
            .expect("Failed to check task existence")
            > 0
    }

    /// Check if a specific label exists in the database
    ///
    /// # Arguments
    ///
    /// * `name` - Label name to check
    ///
    /// # Panics
    ///
    /// Panics if the query fails
    #[must_use]
    pub fn has_label(&self, name: &str) -> bool {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM labels WHERE name = ?",
                [name],
                |row| row.get::<_, i64>(0),
            )
            .expect("Failed to check label existence")
            > 0
    }

    /// Get the status of a task
    ///
    /// # Arguments
    ///
    /// * `id` - Task ID
    ///
    /// # Returns
    ///
    /// The task status as a string, or None if the task doesn't exist
    ///
    /// # Panics
    ///
    /// Panics if the query fails (other than not finding the task)
    #[must_use]
    pub fn get_task_status(&self, id: &str) -> Option<String> {
        self.conn
            .query_row("SELECT status FROM tasks WHERE id = ?", [id], |row| {
                row.get(0)
            })
            .ok()
    }

    /// Get labels for a specific task
    ///
    /// # Arguments
    ///
    /// * `task_id` - Task ID
    ///
    /// # Panics
    ///
    /// Panics if the query fails
    #[must_use]
    pub fn get_task_labels(&self, task_id: &str) -> Vec<String> {
        let mut stmt = self
            .conn
            .prepare("SELECT name FROM labels WHERE task_id = ? ORDER BY name")
            .expect("Failed to prepare statement");

        stmt.query_map([task_id], |row| row.get(0))
            .expect("Failed to query task labels")
            .collect::<Result<Vec<String>, _>>()
            .expect("Failed to collect task labels")
    }

    /// Print database statistics (useful for debugging tests)
    pub fn print_stats(&self) {
        println!("Database Statistics:");
        println!("  Files: {}", self.count_files());
        println!("  Tasks: {}", self.count_tasks());
        println!("  Labels: {}", self.count_labels());
        println!("  Dependencies: {}", self.count_dependencies());
    }
}

/// Assert that a database contains a specific number of files
///
/// # Panics
///
/// Panics if the count doesn't match
pub fn assert_file_count(conn: &Connection, expected: usize) {
    let inspector = DbInspector::new(conn);
    let actual = inspector.count_files();
    assert_eq!(
        actual, expected,
        "Expected {expected} files in database, but found {actual}"
    );
}

/// Assert that a database contains a specific number of tasks
///
/// # Panics
///
/// Panics if the count doesn't match
pub fn assert_task_count(conn: &Connection, expected: usize) {
    let inspector = DbInspector::new(conn);
    let actual = inspector.count_tasks();
    assert_eq!(
        actual, expected,
        "Expected {expected} tasks in database, but found {actual}"
    );
}

/// Assert that a database contains a specific file
///
/// # Panics
///
/// Panics if the file doesn't exist
pub fn assert_has_file(conn: &Connection, path: &str) {
    let inspector = DbInspector::new(conn);
    assert!(
        inspector.has_file(path),
        "Expected database to contain file '{path}'"
    );
}

/// Assert that a database contains a specific task
///
/// # Panics
///
/// Panics if the task doesn't exist
pub fn assert_has_task(conn: &Connection, id: &str) {
    let inspector = DbInspector::new(conn);
    assert!(
        inspector.has_task(id),
        "Expected database to contain task '{id}'"
    );
}

/// Assert that a database contains a specific label
///
/// # Panics
///
/// Panics if the label doesn't exist
pub fn assert_has_label(conn: &Connection, name: &str) {
    let inspector = DbInspector::new(conn);
    assert!(
        inspector.has_label(name),
        "Expected database to contain label '{name}'"
    );
}
