//! Error types for TUI operations

use std::io;
use thiserror::Error;

/// Result type for TUI operations
pub type TuiResult<T> = Result<T, TuiError>;

/// Errors that can occur during TUI operations
#[derive(Debug, Error)]
pub enum TuiError {
    /// IO error during terminal operations
    #[error("Terminal IO error: {0}")]
    Io(#[from] io::Error),

    /// Database error
    #[error("Database error: {0}")]
    Database(#[from] lash_db::DbError),

    /// Crossterm error
    #[error("Crossterm error: {0}")]
    Crossterm(String),

    /// Application logic error
    #[error("Application error: {0}")]
    App(String),
}
