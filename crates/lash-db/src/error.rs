//! Database-specific error types

use thiserror::Error;

/// Result type for database operations
pub type DbResult<T> = Result<T, DbError>;

/// Database operation errors
#[derive(Error, Debug)]
pub enum DbError {
    /// `SQLite` error
    #[error("Database error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// Schema version mismatch
    #[error("Schema version mismatch: expected {expected}, found {found}")]
    SchemaMismatch { expected: i32, found: i32 },

    /// Migration failed
    #[error("Migration to version {version} failed: {reason}")]
    MigrationFailed { version: i32, reason: String },

    /// Serialization/deserialization error
    #[error("JSON serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// File not found in database
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// Task not found in database
    #[error("Task not found: {0}")]
    TaskNotFound(String),

    /// Label not found in database
    #[error("Label not found: {0}")]
    LabelNotFound(String),

    /// Dependency cycle detected
    #[error("Dependency cycle detected: {0}")]
    CycleDetected(String),

    /// Invalid state
    #[error("Invalid database state: {0}")]
    InvalidState(String),

    /// Project root not found
    #[error("Project root not found: {0}")]
    ProjectRootNotFound(String),

    /// I/O error during file operations
    #[error("I/O error: {0}")]
    IoError(String),

    /// Generic database error
    #[error("Database operation failed: {0}")]
    Other(String),
}

impl From<String> for DbError {
    fn from(msg: String) -> Self {
        DbError::Other(msg)
    }
}

impl From<&str> for DbError {
    fn from(msg: &str) -> Self {
        DbError::Other(msg.to_string())
    }
}
