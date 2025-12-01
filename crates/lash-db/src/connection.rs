//! Database connection management and initialization

use rusqlite::Connection;
use std::path::Path;

use crate::error::{DbError, DbResult};

/// Initialize a new database at the given path
///
/// Creates the database file if it doesn't exist, sets up PRAGMAs for optimal
/// performance, runs the schema DDL to create all tables and indexes, and applies
/// all pending migrations to bring the database to the current schema version.
///
/// # Example
///
/// ```no_run
/// use lash_db::connection::init_database;
/// use std::path::Path;
///
/// let conn = init_database(Path::new("/tmp/lash.db")).unwrap();
/// ```
///
/// # Errors
///
/// Returns error if:
/// - Database file cannot be created
/// - Schema DDL execution fails
/// - PRAGMA settings fail
/// - Migrations fail to apply
pub fn init_database(path: &Path) -> DbResult<Connection> {
    let conn = Connection::open(path)?;

    // Set PRAGMAs for optimal performance and safety
    configure_pragmas(&conn)?;

    // Create schema if it doesn't exist
    create_schema(&conn)?;

    // Run any pending migrations to bring schema to current version
    crate::migrations::run_migrations(&conn)?;

    Ok(conn)
}

/// Open an existing database at the given path
///
/// Opens the database, sets PRAGMAs, and runs any pending migrations.
/// Does not create schema if missing.
///
/// # Example
///
/// ```no_run
/// use lash_db::connection::open_database;
/// use std::path::Path;
///
/// let conn = open_database(Path::new("/tmp/lash.db")).unwrap();
/// ```
///
/// # Errors
///
/// Returns error if:
/// - Database file doesn't exist
/// - Connection cannot be established
/// - PRAGMA settings fail
/// - Migrations fail to apply
pub fn open_database(path: &Path) -> DbResult<Connection> {
    let conn = Connection::open(path)?;
    configure_pragmas(&conn)?;

    // Run any pending migrations to bring schema to current version
    crate::migrations::run_migrations(&conn)?;

    Ok(conn)
}

/// Configure `SQLite` `PRAGMA`s for optimal performance
///
/// Sets:
/// - `foreign_keys = ON` - Enforce foreign key constraints
/// - `journal_mode = WAL` - Write-Ahead Logging for better concurrency
/// - `synchronous = NORMAL` - Balance safety and speed
/// - `temp_store = MEMORY` - Use memory for temporary tables
fn configure_pragmas(conn: &Connection) -> DbResult<()> {
    conn.execute_batch(
        "
        PRAGMA foreign_keys = ON;
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA temp_store = MEMORY;
        ",
    )?;
    Ok(())
}

/// Create the database schema
///
/// Executes the full schema DDL from schema.sql. This is idempotent - if tables
/// already exist, this is a no-op due to CREATE TABLE IF NOT EXISTS semantics.
fn create_schema(conn: &Connection) -> DbResult<()> {
    // Include the schema SQL at compile time
    const SCHEMA_SQL: &str = include_str!("../schema.sql");

    conn.execute_batch(SCHEMA_SQL)?;

    Ok(())
}

/// Get the current schema version from the metadata table
///
/// # Example
///
/// ```no_run
/// # use lash_db::connection::{init_database, get_schema_version};
/// # use std::path::Path;
/// # let conn = init_database(Path::new("/tmp/lash.db")).unwrap();
/// let version = get_schema_version(&conn).unwrap();
/// assert_eq!(version, 1);
/// ```
///
/// # Errors
///
/// Returns error if:
/// - Metadata table doesn't exist
/// - Schema version key is missing
/// - Version value is not a valid integer
pub fn get_schema_version(conn: &Connection) -> DbResult<i32> {
    let version: String = conn.query_row(
        "SELECT value FROM metadata WHERE key = 'schema_version'",
        [],
        |row| row.get(0),
    )?;

    version
        .parse()
        .map_err(|_| DbError::InvalidState("Invalid schema version in metadata table".to_string()))
}

/// Set the schema version in the metadata table
///
/// # Errors
///
/// Returns error if update fails
pub fn set_schema_version(conn: &Connection, version: i32) -> DbResult<()> {
    conn.execute(
        "UPDATE metadata SET value = ?1 WHERE key = 'schema_version'",
        [version.to_string()],
    )?;
    Ok(())
}

/// Get a metadata value by key
///
/// # Errors
///
/// Returns error if query fails or key doesn't exist
pub fn get_metadata(conn: &Connection, key: &str) -> DbResult<Option<String>> {
    match conn.query_row("SELECT value FROM metadata WHERE key = ?1", [key], |row| {
        row.get(0)
    }) {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Set a metadata value
///
/// # Errors
///
/// Returns error if insert/update fails
pub fn set_metadata(conn: &Connection, key: &str, value: &str) -> DbResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
        [key, value],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_init_database() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let conn = init_database(path).unwrap();

        // Verify schema version is set
        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, 3);

        // Verify tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();

        assert!(tables.contains(&"metadata".to_string()));
        assert!(tables.contains(&"files".to_string()));
        assert!(tables.contains(&"tasks".to_string()));
        assert!(tables.contains(&"dependencies".to_string()));
        assert!(tables.contains(&"dependency_closure".to_string()));
        assert!(tables.contains(&"labels".to_string()));
        assert!(tables.contains(&"task_labels".to_string()));
        assert!(tables.contains(&"file_labels".to_string()));
    }

    #[test]
    fn test_open_existing_database() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Create database first
        init_database(path).unwrap();

        // Open existing database
        let conn = open_database(path).unwrap();
        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, 3);
    }

    #[test]
    fn test_foreign_keys_enabled() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();

        let fk_enabled: i32 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();

        assert_eq!(fk_enabled, 1);
    }

    #[test]
    fn test_wal_mode_enabled() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();

        let journal_mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();

        assert_eq!(journal_mode.to_lowercase(), "wal");
    }

    #[test]
    fn test_metadata_operations() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();

        // Set a metadata value
        set_metadata(&conn, "test_key", "test_value").unwrap();

        // Get it back
        let value = get_metadata(&conn, "test_key").unwrap();
        assert_eq!(value, Some("test_value".to_string()));

        // Update it
        set_metadata(&conn, "test_key", "new_value").unwrap();
        let value = get_metadata(&conn, "test_key").unwrap();
        assert_eq!(value, Some("new_value".to_string()));

        // Get non-existent key
        let value = get_metadata(&conn, "nonexistent").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn test_schema_version_operations() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();

        // Initial version should be 3 (current schema version)
        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, 3);

        // Update version
        set_schema_version(&conn, 3).unwrap();
        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, 3);
    }

    #[test]
    fn test_fts_table_exists() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();

        // Check FTS virtual table exists
        let fts_exists: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='tasks_fts'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(fts_exists, 1);
    }
}
