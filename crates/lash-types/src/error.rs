//! Error types and diagnostic structures for Lash
//!
//! This module provides a comprehensive error taxonomy for all Lash operations.
//! Errors are designed to be:
//! - Expressive for humans (with rich formatting)
//! - Structured for machines (with stable codes and JSON output)
//! - Actionable (with suggestions and help text)
//!
//! # Error Categories
//!
//! - `Parse`: Markdown parsing failures
//! - `Lint`: Validation and linting errors
//! - `Index`: Database indexing errors
//! - `Dependency`: Dependency resolution errors
//! - `Query`: Search and query errors
//! - `Config`: Configuration errors
//! - `IO`: File system errors
//! - `Internal`: Internal/unexpected errors
//!
//! # Examples
//!
//! ```
//! use lash_types::error::{LashError, codes};
//! use std::path::PathBuf;
//!
//! // Create a parse error with location
//! let err = LashError::parse_invalid_checkbox(
//!     PathBuf::from("tasks.md"),
//!     5,
//!     3,
//!     "[*] invalid checkbox"
//! );
//!
//! // Get the stable error code
//! assert_eq!(err.code(), codes::E_PARSE_INVALID_CHECKBOX);
//!
//! // Convert to diagnostic for reporting
//! let diag = err.to_diagnostic();
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use thiserror::Error;

/// Standardized exit codes for the Lash CLI
///
/// These exit codes provide a consistent interface for scripts and agents
/// to detect different types of failures programmatically.
///
/// # Examples
///
/// ```
/// use lash_types::error::{ExitCode, LashError};
/// use std::path::PathBuf;
///
/// let err = LashError::lint_duplicate_id(
///     PathBuf::from("tasks.md"),
///     10,
///     5,
///     "task-id",
///     5
/// );
///
/// let exit_code = ExitCode::from(&err);
/// assert_eq!(exit_code as i32, 2); // LintError
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ExitCode {
    /// Command completed successfully
    Success = 0,
    /// General/unspecified error
    GeneralError = 1,
    /// Linting or validation error
    LintError = 2,
    /// Database indexing error
    IndexError = 3,
    /// Configuration error (including missing root)
    ConfigError = 4,
    /// Resource not found (file, task, etc.)
    NotFound = 5,
    /// Circular dependency detected
    CycleDetected = 6,
}

impl ExitCode {
    /// Convert exit code to i32 for process exit
    #[must_use]
    pub const fn as_i32(self) -> i32 {
        self as i32
    }
}

impl From<&LashError> for ExitCode {
    fn from(error: &LashError) -> Self {
        match error {
            // Lint and Parse errors -> LintError
            LashError::Lint { .. } | LashError::Parse { .. } => Self::LintError,

            // Index errors -> IndexError
            LashError::Index { .. } => Self::IndexError,

            // Dependency errors -> check for cycle vs not found
            LashError::Dependency { code, .. } => {
                if *code == codes::E_DEP_CYCLE {
                    Self::CycleDetected
                } else if *code == codes::E_DEP_NOT_FOUND {
                    Self::NotFound
                } else {
                    Self::GeneralError
                }
            }

            // Config errors -> ConfigError
            LashError::Config { .. } => Self::ConfigError,

            // Query errors -> NotFound if no results, otherwise GeneralError
            LashError::Query { code, .. } => {
                if *code == codes::E_QUERY_NO_RESULTS {
                    Self::NotFound
                } else {
                    Self::GeneralError
                }
            }

            // IO errors -> NotFound if file not found, otherwise GeneralError
            LashError::IO { code, .. } => {
                if *code == codes::E_IO_FILE_NOT_FOUND {
                    Self::NotFound
                } else {
                    Self::GeneralError
                }
            }

            // Internal errors -> GeneralError
            LashError::Internal { .. } => Self::GeneralError,
        }
    }
}

impl fmt::Display for ExitCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success => write!(f, "0 (success)"),
            Self::GeneralError => write!(f, "1 (general error)"),
            Self::LintError => write!(f, "2 (lint error)"),
            Self::IndexError => write!(f, "3 (index error)"),
            Self::ConfigError => write!(f, "4 (config error)"),
            Self::NotFound => write!(f, "5 (not found)"),
            Self::CycleDetected => write!(f, "6 (cycle detected)"),
        }
    }
}

/// Result type alias for Lash operations
///
/// Note: `LashError` is intentionally large (168 bytes) to provide rich context
/// for error reporting. This is acceptable for a CLI application where errors
/// are the exception, not the hot path. Boxing would add allocation overhead
/// for the common success case.
#[allow(clippy::result_large_err)]
pub type Result<T> = std::result::Result<T, LashError>;

/// Main error type for all Lash operations
#[derive(Error, Debug, Clone)]
pub enum LashError {
    /// Markdown parsing failures
    #[error("parse error: {message}")]
    Parse {
        /// Stable error code identifying the specific parse failure
        code: &'static str,
        /// Human-readable description of what went wrong
        message: String,
        /// File and line/column where the parse error occurred
        location: Option<Location>,
        /// The offending source text, for context
        snippet: Option<String>,
        /// Suggestion for how to fix the error
        help: Option<String>,
    },

    /// Validation/linting failures
    #[error("lint error: {message}")]
    Lint {
        /// Stable error code identifying the specific lint rule violated
        code: &'static str,
        /// Human-readable description of the violation
        message: String,
        /// File and line/column where the violation occurred
        location: Option<Location>,
        /// The offending source text, for context
        snippet: Option<String>,
        /// Suggestion for how to fix the violation
        help: Option<String>,
    },

    /// Database indexing errors
    #[error("index error: {message}")]
    Index {
        /// Stable error code identifying the specific indexing failure
        code: &'static str,
        /// Human-readable description of the indexing failure
        message: String,
        /// Additional context about what was being indexed when the error occurred
        context: Option<String>,
        /// Suggestion for how to resolve the error
        help: Option<String>,
    },

    /// Dependency resolution errors
    #[error("dependency error: {message}")]
    Dependency {
        /// Stable error code identifying the specific dependency issue
        code: &'static str,
        /// Human-readable description of the dependency problem
        message: String,
        /// File and line/column of the dependency reference
        location: Option<Location>,
        /// The dependency chain that led to this error (e.g., cycle path)
        chain: Option<Vec<String>>,
        /// Suggestion for how to resolve the dependency issue
        help: Option<String>,
    },

    /// Query/search errors
    #[error("query error: {message}")]
    Query {
        /// Stable error code identifying the specific query failure
        code: &'static str,
        /// Human-readable description of the query failure
        message: String,
        /// Suggestion for how to fix the query
        help: Option<String>,
    },

    /// Configuration errors
    #[error("configuration error: {message}")]
    Config {
        /// Stable error code identifying the specific configuration issue
        code: &'static str,
        /// Human-readable description of the configuration problem
        message: String,
        /// Path to the configuration file or project root that caused the error
        path: Option<PathBuf>,
        /// Suggestion for how to fix the configuration
        help: Option<String>,
    },

    /// File system errors
    #[error("I/O error: {message}")]
    IO {
        /// Stable error code identifying the specific I/O failure
        code: &'static str,
        /// Human-readable description of the I/O failure
        message: String,
        /// Path to the file or directory that caused the error
        path: Option<PathBuf>,
        /// The underlying OS error message, if available
        io_error: Option<String>,
    },

    /// Internal/unexpected errors
    #[error("internal error: {message}")]
    Internal {
        /// Stable error code identifying the specific internal failure
        code: &'static str,
        /// Human-readable description of the internal error
        message: String,
        /// Additional context about the internal state when the error occurred
        context: Option<String>,
    },
}

impl LashError {
    /// Get the stable error code for this error
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Parse { code, .. }
            | Self::Lint { code, .. }
            | Self::Index { code, .. }
            | Self::Dependency { code, .. }
            | Self::Query { code, .. }
            | Self::Config { code, .. }
            | Self::IO { code, .. }
            | Self::Internal { code, .. } => code,
        }
    }

    /// Convert this error to a Diagnostic for rich formatting
    #[must_use]
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            Self::Parse {
                code,
                message,
                location,
                snippet,
                help,
            }
            | Self::Lint {
                code,
                message,
                location,
                snippet,
                help,
            } => Diagnostic {
                code,
                severity: Severity::Error,
                message: message.clone(),
                location: location.clone(),
                snippet: snippet.clone(),
                help: help.clone(),
                labels: None,
                recovery_command: None,
                fix_steps: None,
                explanation: None,
                docs_url: None,
            },
            Self::Dependency {
                code,
                message,
                location,
                chain,
                help,
            } => Diagnostic {
                code,
                severity: Severity::Error,
                message: message.clone(),
                location: location.clone(),
                snippet: None,
                help: help.clone(),
                labels: chain
                    .as_ref()
                    .map(|c| vec![("dependency chain".to_string(), c.join(" -> "))]),
                recovery_command: None,
                fix_steps: None,
                explanation: None,
                docs_url: None,
            },
            Self::Index {
                code,
                message,
                context,
                help,
            } => Diagnostic {
                code,
                severity: Severity::Error,
                message: message.clone(),
                location: None,
                snippet: None,
                help: help.clone(),
                labels: context
                    .as_ref()
                    .map(|c| vec![("context".to_string(), c.clone())]),
                recovery_command: None,
                fix_steps: None,
                explanation: None,
                docs_url: None,
            },
            Self::Query {
                code,
                message,
                help,
            } => Diagnostic {
                code,
                severity: Severity::Error,
                message: message.clone(),
                location: None,
                snippet: None,
                help: help.clone(),
                labels: None,
                recovery_command: None,
                fix_steps: None,
                explanation: None,
                docs_url: None,
            },
            Self::Config {
                code,
                message,
                path,
                help,
            } => Diagnostic {
                code,
                severity: Severity::Error,
                message: message.clone(),
                location: path.as_ref().map(|p| Location {
                    file_path: p.clone(),
                    line: None,
                    column: None,
                    span: None,
                }),
                snippet: None,
                help: help.clone(),
                labels: None,
                recovery_command: None,
                fix_steps: None,
                explanation: None,
                docs_url: None,
            },
            Self::IO {
                code,
                message,
                path,
                io_error,
            } => Diagnostic {
                code,
                severity: Severity::Error,
                message: message.clone(),
                location: path.as_ref().map(|p| Location {
                    file_path: p.clone(),
                    line: None,
                    column: None,
                    span: None,
                }),
                snippet: None,
                help: None,
                labels: io_error
                    .as_ref()
                    .map(|s| vec![("underlying error".to_string(), s.clone())]),
                recovery_command: None,
                fix_steps: None,
                explanation: None,
                docs_url: None,
            },
            Self::Internal {
                code,
                message,
                context,
            } => Diagnostic {
                code,
                severity: Severity::Error,
                message: message.clone(),
                location: None,
                snippet: None,
                help: Some("This is an internal error. Please report this as a bug.".to_string()),
                labels: context
                    .as_ref()
                    .map(|c| vec![("context".to_string(), c.clone())]),
                recovery_command: None,
                fix_steps: None,
                explanation: None,
                docs_url: None,
            },
        }
    }

    // ===== Parse Error Constructors =====

    /// Invalid checkbox syntax
    #[must_use]
    pub fn parse_invalid_checkbox(
        file: PathBuf,
        line: usize,
        column: usize,
        snippet: impl Into<String>,
    ) -> Self {
        Self::Parse {
            code: codes::E_PARSE_INVALID_CHECKBOX,
            message: "invalid checkbox syntax".to_string(),
            location: Some(Location::new(file, line, column)),
            snippet: Some(snippet.into()),
            help: Some("checkboxes must be one of: [ ], [-], [x], or [!]".to_string()),
        }
    }

    /// Malformed annotation
    #[must_use]
    pub fn parse_invalid_annotation(
        file: PathBuf,
        line: usize,
        column: usize,
        snippet: impl Into<String>,
        annotation_name: impl Into<String>,
    ) -> Self {
        Self::Parse {
            code: codes::E_PARSE_INVALID_ANNOTATION,
            message: format!("invalid annotation format: @{}", annotation_name.into()),
            location: Some(Location::new(file, line, column)),
            snippet: Some(snippet.into()),
            help: Some("annotations must be in format: @key: value".to_string()),
        }
    }

    /// Invalid header format
    #[must_use]
    pub fn parse_invalid_header(file: PathBuf, line: usize, snippet: impl Into<String>) -> Self {
        Self::Parse {
            code: codes::E_PARSE_INVALID_HEADER,
            message: "invalid header format".to_string(),
            location: Some(Location::new(file, line, 1)),
            snippet: Some(snippet.into()),
            help: Some("headers must start with # and be followed by a space".to_string()),
        }
    }

    /// Unexpected nesting depth
    #[must_use]
    pub fn parse_unexpected_depth(
        file: PathBuf,
        line: usize,
        column: usize,
        found_depth: usize,
    ) -> Self {
        Self::Parse {
            code: codes::E_PARSE_UNEXPECTED_DEPTH,
            message: format!("unexpected indentation depth: {found_depth}"),
            location: Some(Location::new(file, line, column)),
            snippet: None,
            help: Some("each level should be indented by exactly 2 spaces".to_string()),
        }
    }

    /// Invalid date format
    #[must_use]
    pub fn parse_invalid_date(
        file: PathBuf,
        line: usize,
        column: usize,
        date_str: impl Into<String>,
    ) -> Self {
        Self::Parse {
            code: codes::E_PARSE_INVALID_DATE,
            message: format!("invalid date format: {}", date_str.into()),
            location: Some(Location::new(file, line, column)),
            snippet: None,
            help: Some("dates must be in YYYY-MM-DD format".to_string()),
        }
    }

    // ===== Lint Error Constructors =====

    /// Duplicate task ID
    #[must_use]
    pub fn lint_duplicate_id(
        file: PathBuf,
        line: usize,
        column: usize,
        id: impl Into<String>,
        first_occurrence_line: usize,
    ) -> Self {
        let id_str = id.into();
        Self::Lint {
            code: codes::E_LINT_DUPLICATE_ID,
            message: format!("duplicate task ID: '{id_str}'"),
            location: Some(Location::new(file, line, column)),
            snippet: Some(format!("@id: {id_str}")),
            help: Some(format!(
                "task ID '{id_str}' was already defined on line {first_occurrence_line}"
            )),
        }
    }

    /// Unknown annotation
    #[must_use]
    pub fn lint_unknown_annotation(
        file: PathBuf,
        line: usize,
        column: usize,
        annotation: impl Into<String>,
    ) -> Self {
        let ann = annotation.into();
        Self::Lint {
            code: codes::E_LINT_UNKNOWN_ANNOTATION,
            message: format!("unknown annotation: @{ann}"),
            location: Some(Location::new(file, line, column)),
            snippet: Some(format!("@{ann}")),
            help: Some("valid annotations: @id, @labels, @owner, @estimate, @depends-on, @created, @agent-note".to_string()),
        }
    }

    /// Depth limit exceeded
    #[must_use]
    pub fn lint_depth_exceeded(
        file: PathBuf,
        line: usize,
        column: usize,
        depth: usize,
        max_depth: usize,
    ) -> Self {
        Self::Lint {
            code: codes::E_LINT_DEPTH_EXCEEDED,
            message: format!("task nesting exceeds maximum depth: {depth} > {max_depth}"),
            location: Some(Location::new(file, line, column)),
            snippet: None,
            help: Some(format!(
                "flatten the hierarchy to {max_depth} levels or fewer"
            )),
        }
    }

    /// Status inconsistency
    #[must_use]
    pub fn lint_status_inconsistency(
        file: PathBuf,
        line: usize,
        column: usize,
        parent_status: impl Into<String>,
    ) -> Self {
        Self::Lint {
            code: codes::E_LINT_STATUS_INCONSISTENCY,
            message: "parent task is marked done but has incomplete children".to_string(),
            location: Some(Location::new(file, line, column)),
            snippet: None,
            help: Some(format!(
                "parent with status '{}' cannot have incomplete child tasks",
                parent_status.into()
            )),
        }
    }

    /// Invalid label format
    #[must_use]
    pub fn lint_invalid_label(
        file: PathBuf,
        line: usize,
        column: usize,
        label: impl Into<String>,
    ) -> Self {
        let lbl = label.into();
        Self::Lint {
            code: codes::E_LINT_INVALID_LABEL,
            message: format!("invalid label format: '{lbl}'"),
            location: Some(Location::new(file, line, column)),
            snippet: Some(format!("@labels: {lbl}")),
            help: Some("labels must be alphanumeric with hyphens, separated by commas".to_string()),
        }
    }

    /// Missing required annotation
    #[must_use]
    pub fn lint_missing_annotation(
        file: PathBuf,
        line: usize,
        annotation: impl Into<String>,
    ) -> Self {
        let ann = annotation.into();
        Self::Lint {
            code: codes::E_LINT_MISSING_ANNOTATION,
            message: format!("missing required annotation: @{ann}"),
            location: Some(Location::new(file, line, 1)),
            snippet: None,
            help: Some(format!("add @{ann} annotation to this task")),
        }
    }

    /// Bad indentation
    #[must_use]
    pub fn lint_bad_indentation(
        file: PathBuf,
        line: usize,
        column: usize,
        found: usize,
        expected: usize,
    ) -> Self {
        Self::Lint {
            code: codes::E_LINT_BAD_INDENTATION,
            message: format!("incorrect indentation: found {found} spaces, expected {expected}"),
            location: Some(Location::new(file, line, column)),
            snippet: None,
            help: Some("run `lash format` to fix indentation automatically".to_string()),
        }
    }

    // ===== Dependency Error Constructors =====

    /// Broken reference (target not found)
    #[must_use]
    pub fn dep_not_found(
        file: PathBuf,
        line: usize,
        column: usize,
        reference: impl Into<String>,
    ) -> Self {
        let ref_str = reference.into();
        Self::Dependency {
            code: codes::E_DEP_NOT_FOUND,
            message: format!("dependency target not found: {ref_str}"),
            location: Some(Location::new(file, line, column)),
            chain: None,
            help: Some("check that the referenced file and task ID exist".to_string()),
        }
    }

    /// Circular dependency
    #[must_use]
    pub fn dep_cycle(chain: &[String]) -> Self {
        Self::Dependency {
            code: codes::E_DEP_CYCLE,
            message: "circular dependency detected".to_string(),
            location: None,
            chain: Some(chain.to_owned()),
            help: Some(format!(
                "break the cycle by removing one of these dependencies: {}",
                chain.join(" -> ")
            )),
        }
    }

    /// Invalid reference format
    #[must_use]
    pub fn dep_invalid_ref(
        file: PathBuf,
        line: usize,
        column: usize,
        reference: impl Into<String>,
    ) -> Self {
        let ref_str = reference.into();
        Self::Dependency {
            code: codes::E_DEP_INVALID_REF,
            message: format!("invalid dependency reference format: {ref_str}"),
            location: Some(Location::new(file, line, column)),
            chain: None,
            help: Some("dependencies must be in format: path/to/file.md#task:id".to_string()),
        }
    }

    // ===== Index Error Constructors =====

    /// Database corruption
    #[must_use]
    pub fn index_corrupted(details: impl Into<String>) -> Self {
        Self::Index {
            code: codes::E_INDEX_CORRUPTED,
            message: "database is corrupted".to_string(),
            context: Some(details.into()),
            help: Some(
                "run `lash index --rebuild` to rebuild the database from scratch".to_string(),
            ),
        }
    }

    /// Schema version mismatch
    #[must_use]
    pub fn index_version_mismatch(found: u32, expected: u32) -> Self {
        Self::Index {
            code: codes::E_INDEX_VERSION_MISMATCH,
            message: format!(
                "database schema version mismatch: found {found}, expected {expected}"
            ),
            context: None,
            help: Some("run `lash index --migrate` to update the database schema".to_string()),
        }
    }

    /// Index out of sync
    #[must_use]
    pub fn index_out_of_sync(files_changed: usize) -> Self {
        Self::Index {
            code: codes::E_INDEX_OUT_OF_SYNC,
            message: format!("index is out of sync ({files_changed} files changed)"),
            context: None,
            help: Some("run `lash index` to update the index".to_string()),
        }
    }

    // ===== Query Error Constructors =====

    /// Invalid query syntax
    #[must_use]
    pub fn query_invalid_syntax(query: impl Into<String>) -> Self {
        let q = query.into();
        Self::Query {
            code: codes::E_QUERY_INVALID_SYNTAX,
            message: format!("invalid query syntax: {q}"),
            help: Some("see `lash help search` for query syntax".to_string()),
        }
    }

    /// No results found
    #[must_use]
    pub fn query_no_results(query: impl Into<String>) -> Self {
        Self::Query {
            code: codes::E_QUERY_NO_RESULTS,
            message: format!("no results found for query: {}", query.into()),
            help: Some("try a broader search or check your filters".to_string()),
        }
    }

    // ===== Config Error Constructors =====

    /// Root directory not found
    #[must_use]
    pub fn config_root_not_found(search_path: PathBuf) -> Self {
        Self::Config {
            code: codes::E_CONFIG_ROOT_NOT_FOUND,
            message: "could not find lash root directory".to_string(),
            path: Some(search_path),
            help: Some("run `lash init` to create a new lash project".to_string()),
        }
    }

    /// Invalid configuration value
    #[must_use]
    pub fn config_invalid_value(key: impl Into<String>, value: impl Into<String>) -> Self {
        let k = key.into();
        let v = value.into();
        Self::Config {
            code: codes::E_CONFIG_INVALID_VALUE,
            message: format!("invalid configuration value for '{k}': {v}"),
            path: None,
            help: Some(format!(
                "check the documentation for valid values for '{k}'"
            )),
        }
    }

    /// Config parse error
    #[must_use]
    pub fn config_parse_error(path: PathBuf, error: impl Into<String>) -> Self {
        Self::Config {
            code: codes::E_CONFIG_PARSE_ERROR,
            message: format!("failed to parse configuration file: {}", error.into()),
            path: Some(path),
            help: Some("check that the configuration file is valid TOML".to_string()),
        }
    }

    /// Missing index file
    #[must_use]
    pub fn config_missing_index() -> Self {
        Self::Config {
            code: codes::E_CONFIG_MISSING_INDEX,
            message: "index file not found (lash.index.md or index.lash.md)".to_string(),
            path: None,
            help: Some("create an index file at the root of your project".to_string()),
        }
    }

    // ===== IO Error Constructors =====

    /// File not found
    #[must_use]
    pub fn io_file_not_found(path: PathBuf) -> Self {
        Self::IO {
            code: codes::E_IO_FILE_NOT_FOUND,
            message: format!("file not found: {}", path.display()),
            path: Some(path),
            io_error: None,
        }
    }

    /// Read error
    #[must_use]
    pub fn io_read_error(path: PathBuf, error: impl Into<String>) -> Self {
        Self::IO {
            code: codes::E_IO_READ_ERROR,
            message: format!("failed to read file: {}", path.display()),
            path: Some(path),
            io_error: Some(error.into()),
        }
    }

    /// Write error
    #[must_use]
    pub fn io_write_error(path: PathBuf, error: impl Into<String>) -> Self {
        Self::IO {
            code: codes::E_IO_WRITE_ERROR,
            message: format!("failed to write file: {}", path.display()),
            path: Some(path),
            io_error: Some(error.into()),
        }
    }

    /// Permission denied
    #[must_use]
    pub fn io_permission_denied(path: PathBuf) -> Self {
        Self::IO {
            code: codes::E_IO_PERMISSION_DENIED,
            message: format!("permission denied: {}", path.display()),
            path: Some(path),
            io_error: None,
        }
    }

    /// Invalid path
    #[must_use]
    pub fn io_invalid_path(path: PathBuf, reason: impl Into<String>) -> Self {
        Self::IO {
            code: codes::E_IO_INVALID_PATH,
            message: format!("invalid path: {}", path.display()),
            path: Some(path),
            io_error: Some(reason.into()),
        }
    }

    // ===== Internal Error Constructors =====

    /// Unexpected internal error
    #[must_use]
    pub fn internal(message: impl Into<String>, context: Option<String>) -> Self {
        Self::Internal {
            code: codes::E_INTERNAL,
            message: message.into(),
            context,
        }
    }
}

/// Diagnostic structure for error reporting
///
/// This is the primary structure used for formatting and displaying errors.
/// It can be serialized to JSON for machine consumption or formatted as
/// rich text for human consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Stable error code (e.g., `E_PARSE_INVALID_CHECKBOX`)
    pub code: &'static str,

    /// Severity level
    pub severity: Severity,

    /// Human-readable error message
    pub message: String,

    /// Location where the error occurred
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,

    /// Code snippet showing the error context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,

    /// Help text or suggestion for fixing the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,

    /// Additional labeled context (for dependency chains, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<(String, String)>>,

    // === Agent-friendly fields ===
    /// Exact CLI command to run for automated recovery
    /// e.g., `lash format path/to/file.md` or `lash lint --fix path/to/file.md`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_command: Option<String>,

    /// Step-by-step instructions for manually fixing the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_steps: Option<Vec<String>>,

    /// Detailed explanation of the error for agents/documentation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,

    /// URL to documentation for this error code (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs_url: Option<String>,
}

impl Diagnostic {
    /// Convert diagnostic to JSON string
    ///
    /// # Errors
    ///
    /// Returns error if serialization fails
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    /// Create a new diagnostic with a suggestion
    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Create a new diagnostic with a snippet
    #[must_use]
    pub fn with_snippet(mut self, snippet: impl Into<String>) -> Self {
        self.snippet = Some(snippet.into());
        self
    }

    /// Create a new diagnostic with labels
    #[must_use]
    pub fn with_labels(mut self, labels: Vec<(String, String)>) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Create a new diagnostic with a recovery command
    #[must_use]
    pub fn with_recovery_command(mut self, cmd: impl Into<String>) -> Self {
        self.recovery_command = Some(cmd.into());
        self
    }

    /// Create a new diagnostic with fix steps
    #[must_use]
    pub fn with_fix_steps(mut self, steps: Vec<String>) -> Self {
        self.fix_steps = Some(steps);
        self
    }

    /// Create a new diagnostic with an explanation
    #[must_use]
    pub fn with_explanation(mut self, explanation: impl Into<String>) -> Self {
        self.explanation = Some(explanation.into());
        self
    }

    /// Create a new diagnostic with a docs URL
    #[must_use]
    pub fn with_docs_url(mut self, url: impl Into<String>) -> Self {
        self.docs_url = Some(url.into());
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.code, self.severity, self.message)?;

        if let Some(location) = &self.location {
            write!(f, "\n  at {location}")?;
        }

        if let Some(snippet) = &self.snippet {
            write!(f, "\n  snippet: {snippet}")?;
        }

        if let Some(help) = &self.help {
            write!(f, "\n  help: {help}")?;
        }

        if let Some(labels) = &self.labels {
            for (key, value) in labels {
                write!(f, "\n  {key}: {value}")?;
            }
        }

        Ok(())
    }
}

/// Location of an error in a file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Location {
    /// Path to the file
    pub file_path: PathBuf,

    /// Line number (1-indexed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,

    /// Column number (1-indexed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,

    /// Character span (start, end) for highlighting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<(usize, usize)>,
}

impl Location {
    /// Create a new location
    #[must_use]
    pub fn new(file_path: PathBuf, line: usize, column: usize) -> Self {
        Self {
            file_path,
            line: Some(line),
            column: Some(column),
            span: None,
        }
    }

    /// Create a location with just a file path
    #[must_use]
    pub fn file_only(file_path: PathBuf) -> Self {
        Self {
            file_path,
            line: None,
            column: None,
            span: None,
        }
    }

    /// Add a span to this location
    #[must_use]
    pub fn with_span(mut self, start: usize, end: usize) -> Self {
        self.span = Some((start, end));
        self
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.file_path.display())?;
        if let Some(line) = self.line {
            write!(f, ":{line}")?;
            if let Some(col) = self.column {
                write!(f, ":{col}")?;
            }
        }
        Ok(())
    }
}

/// Severity level for diagnostics
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Fatal error
    Error,
    /// Warning that should be addressed
    Warning,
    /// Informational message
    Info,
    /// Helpful hint
    Hint,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
            Self::Info => write!(f, "info"),
            Self::Hint => write!(f, "hint"),
        }
    }
}

/// Stable error codes for all Lash errors
///
/// These codes are stable across versions and can be used for:
/// - Error documentation lookup
/// - Automated error handling
/// - Error filtering and reporting
/// - Agent integration
pub mod codes {
    // Parse errors (E_PARSE_*)
    /// Invalid checkbox syntax
    pub const E_PARSE_INVALID_CHECKBOX: &str = "E_PARSE_INVALID_CHECKBOX";
    /// Malformed annotation
    pub const E_PARSE_INVALID_ANNOTATION: &str = "E_PARSE_INVALID_ANNOTATION";
    /// Invalid header format
    pub const E_PARSE_INVALID_HEADER: &str = "E_PARSE_INVALID_HEADER";
    /// Unexpected indentation depth
    pub const E_PARSE_UNEXPECTED_DEPTH: &str = "E_PARSE_UNEXPECTED_DEPTH";
    /// Invalid date format
    pub const E_PARSE_INVALID_DATE: &str = "E_PARSE_INVALID_DATE";

    // Lint errors (E_LINT_*)
    /// Task nesting exceeds maximum depth
    pub const E_LINT_DEPTH_EXCEEDED: &str = "E_LINT_DEPTH_EXCEEDED";
    /// Duplicate task ID within file
    pub const E_LINT_DUPLICATE_ID: &str = "E_LINT_DUPLICATE_ID";
    /// Missing required annotation
    pub const E_LINT_MISSING_ANNOTATION: &str = "E_LINT_MISSING_ANNOTATION";
    /// Invalid task status
    pub const E_LINT_STATUS_INCONSISTENCY: &str = "E_LINT_STATUS_INCONSISTENCY";
    /// Unknown annotation
    pub const E_LINT_UNKNOWN_ANNOTATION: &str = "E_LINT_UNKNOWN_ANNOTATION";
    /// Incorrect indentation
    pub const E_LINT_BAD_INDENTATION: &str = "E_LINT_BAD_INDENTATION";
    /// Invalid label format
    pub const E_LINT_INVALID_LABEL: &str = "E_LINT_INVALID_LABEL";
    /// `@doc:` fragment does not match any heading in the target document
    pub const W_SEM_DOC_FRAGMENT: &str = "W_SEM_DOC_FRAGMENT";

    // Dependency errors (E_DEP_*)
    /// Dependency target not found
    pub const E_DEP_NOT_FOUND: &str = "E_DEP_NOT_FOUND";
    /// Circular dependency detected
    pub const E_DEP_CYCLE: &str = "E_DEP_CYCLE";
    /// Invalid dependency reference format
    pub const E_DEP_INVALID_REF: &str = "E_DEP_INVALID_REF";

    // Index errors (E_INDEX_*)
    /// Database is corrupted
    pub const E_INDEX_CORRUPTED: &str = "E_INDEX_CORRUPTED";
    /// Database schema version mismatch
    pub const E_INDEX_VERSION_MISMATCH: &str = "E_INDEX_VERSION_MISMATCH";
    /// Index is out of sync with files
    pub const E_INDEX_OUT_OF_SYNC: &str = "E_INDEX_OUT_OF_SYNC";

    // Query errors (E_QUERY_*)
    /// Invalid query syntax
    pub const E_QUERY_INVALID_SYNTAX: &str = "E_QUERY_INVALID_SYNTAX";
    /// No results found
    pub const E_QUERY_NO_RESULTS: &str = "E_QUERY_NO_RESULTS";

    // I/O errors (E_IO_*)
    /// File not found
    pub const E_IO_FILE_NOT_FOUND: &str = "E_IO_FILE_NOT_FOUND";
    /// Failed to read file
    pub const E_IO_READ_ERROR: &str = "E_IO_READ_ERROR";
    /// Failed to write file
    pub const E_IO_WRITE_ERROR: &str = "E_IO_WRITE_ERROR";
    /// Permission denied
    pub const E_IO_PERMISSION_DENIED: &str = "E_IO_PERMISSION_DENIED";
    /// Invalid path
    pub const E_IO_INVALID_PATH: &str = "E_IO_INVALID_PATH";

    // Configuration errors (E_CONFIG_*)
    /// Root directory not found
    pub const E_CONFIG_ROOT_NOT_FOUND: &str = "E_CONFIG_ROOT_NOT_FOUND";
    /// Invalid configuration value
    pub const E_CONFIG_INVALID_VALUE: &str = "E_CONFIG_INVALID_VALUE";
    /// Configuration parse error
    pub const E_CONFIG_PARSE_ERROR: &str = "E_CONFIG_PARSE_ERROR";
    /// Missing index file
    pub const E_CONFIG_MISSING_INDEX: &str = "E_CONFIG_MISSING_INDEX";

    // Internal errors (E_INTERNAL_*)
    /// Unexpected internal error
    pub const E_INTERNAL: &str = "E_INTERNAL";

    // Task creation errors (E_CREATE_*)
    /// Task title is empty or whitespace-only
    pub const E_CREATE_EMPTY_TITLE: &str = "E_CREATE_EMPTY_TITLE";
    /// Task title exceeds maximum length
    pub const E_CREATE_TITLE_TOO_LONG: &str = "E_CREATE_TITLE_TOO_LONG";
    /// Target file does not exist
    pub const E_CREATE_FILE_NOT_FOUND: &str = "E_CREATE_FILE_NOT_FOUND";
    /// Target file is not writable
    pub const E_CREATE_FILE_NOT_WRITABLE: &str = "E_CREATE_FILE_NOT_WRITABLE";
    /// Target file failed to parse
    pub const E_CREATE_FILE_PARSE_FAILED: &str = "E_CREATE_FILE_PARSE_FAILED";
    /// Parent task not found
    pub const E_CREATE_PARENT_NOT_FOUND: &str = "E_CREATE_PARENT_NOT_FOUND";
    /// Task would exceed maximum nesting depth
    pub const E_CREATE_DEPTH_LIMIT_EXCEEDED: &str = "E_CREATE_DEPTH_LIMIT_EXCEEDED";
    /// Task ID is already in use
    pub const E_CREATE_DUPLICATE_ID: &str = "E_CREATE_DUPLICATE_ID";
    /// Task ID format is invalid
    pub const E_CREATE_INVALID_ID_FORMAT: &str = "E_CREATE_INVALID_ID_FORMAT";
    /// Label format is invalid
    pub const E_CREATE_INVALID_LABEL: &str = "E_CREATE_INVALID_LABEL";
    /// Time estimate format is invalid
    pub const E_CREATE_INVALID_ESTIMATE: &str = "E_CREATE_INVALID_ESTIMATE";
    /// Dependency reference not found
    pub const E_CREATE_DEPENDENCY_NOT_FOUND: &str = "E_CREATE_DEPENDENCY_NOT_FOUND";
    /// Would create circular dependency
    pub const E_CREATE_WOULD_CREATE_CYCLE: &str = "E_CREATE_WOULD_CREATE_CYCLE";
    /// Insert position is invalid
    pub const E_CREATE_INVALID_POSITION: &str = "E_CREATE_INVALID_POSITION";
    /// I/O error during task creation
    pub const E_CREATE_IO_ERROR: &str = "E_CREATE_IO_ERROR";

    // Legacy error code aliases (for backward compatibility with existing code)
    // These should be removed once all code is updated to use the new naming
    /// Deprecated alias for [`E_PARSE_INVALID_CHECKBOX`]
    #[deprecated(note = "Use E_PARSE_INVALID_CHECKBOX instead")]
    pub const E_PARSE_BAD_CHECKBOX: &str = E_PARSE_INVALID_CHECKBOX;
    /// Deprecated alias for [`E_PARSE_INVALID_HEADER`]
    #[deprecated(note = "Use E_PARSE_INVALID_HEADER instead")]
    pub const E_PARSE_MALFORMED_HEADING: &str = E_PARSE_INVALID_HEADER;
    /// Deprecated alias for [`E_LINT_MISSING_ANNOTATION`]
    #[deprecated(note = "Use E_LINT_MISSING_ANNOTATION instead")]
    pub const E_LINT_MISSING_ID: &str = E_LINT_MISSING_ANNOTATION;
    /// Deprecated alias for [`E_LINT_STATUS_INCONSISTENCY`]
    #[deprecated(note = "Use E_LINT_STATUS_INCONSISTENCY instead")]
    pub const E_LINT_INVALID_STATUS: &str = E_LINT_STATUS_INCONSISTENCY;
    /// Deprecated alias for [`E_INDEX_CORRUPTED`]
    #[deprecated(note = "Use E_INDEX_CORRUPTED instead")]
    pub const E_DB_CONNECTION: &str = E_INDEX_CORRUPTED;
    /// Deprecated alias for [`E_INDEX_CORRUPTED`]
    #[deprecated(note = "Use E_INDEX_CORRUPTED instead")]
    pub const E_DB_QUERY: &str = E_INDEX_CORRUPTED;
    /// Deprecated alias for [`E_INDEX_CORRUPTED`]
    #[deprecated(note = "Use E_INDEX_CORRUPTED instead")]
    pub const E_DB_CONSTRAINT: &str = E_INDEX_CORRUPTED;
    /// Deprecated alias for [`E_INDEX_VERSION_MISMATCH`]
    #[deprecated(note = "Use E_INDEX_VERSION_MISMATCH instead")]
    pub const E_DB_MIGRATION: &str = E_INDEX_VERSION_MISMATCH;
    /// Deprecated alias for [`E_CONFIG_ROOT_NOT_FOUND`]
    #[deprecated(note = "Use E_CONFIG_ROOT_NOT_FOUND instead")]
    pub const E_CFG_ROOT_NOT_FOUND: &str = E_CONFIG_ROOT_NOT_FOUND;
    /// Deprecated alias for [`E_CONFIG_INVALID_VALUE`]
    #[deprecated(note = "Use E_CONFIG_INVALID_VALUE instead")]
    pub const E_CFG_INVALID_VALUE: &str = E_CONFIG_INVALID_VALUE;
    /// Deprecated alias for [`E_CONFIG_PARSE_ERROR`]
    #[deprecated(note = "Use E_CONFIG_PARSE_ERROR instead")]
    pub const E_CFG_PARSE_ERROR: &str = E_CONFIG_PARSE_ERROR;
    /// Deprecated alias for [`E_CONFIG_MISSING_INDEX`]
    #[deprecated(note = "Use E_CONFIG_MISSING_INDEX instead")]
    pub const E_CFG_MISSING_INDEX: &str = E_CONFIG_MISSING_INDEX;
    /// Deprecated alias for [`E_LINT_INVALID_LABEL`]
    #[deprecated(note = "Use E_LINT_INVALID_LABEL instead")]
    pub const E_LINT_INVALID_LABEL_FORMAT: &str = E_LINT_INVALID_LABEL;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_extraction() {
        let err = LashError::parse_invalid_checkbox(PathBuf::from("test.md"), 5, 3, "[*] invalid");
        assert_eq!(err.code(), codes::E_PARSE_INVALID_CHECKBOX);
    }

    #[test]
    fn test_parse_error_constructor() {
        let err =
            LashError::parse_invalid_checkbox(PathBuf::from("test.md"), 10, 5, "[*] bad checkbox");

        assert_eq!(err.code(), codes::E_PARSE_INVALID_CHECKBOX);
        let diag = err.to_diagnostic();
        assert_eq!(diag.severity, Severity::Error);
        assert!(diag.help.is_some());
        assert!(diag.snippet.is_some());
    }

    #[test]
    fn test_lint_duplicate_id() {
        let err = LashError::lint_duplicate_id(PathBuf::from("tasks.md"), 15, 3, "setup-db", 10);

        assert_eq!(err.code(), codes::E_LINT_DUPLICATE_ID);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains("setup-db"));
        assert!(diag.help.as_ref().unwrap().contains("10"));
    }

    #[test]
    fn test_dependency_cycle() {
        let chain = vec![
            "task1".to_string(),
            "task2".to_string(),
            "task3".to_string(),
            "task1".to_string(),
        ];
        let err = LashError::dep_cycle(&chain);

        assert_eq!(err.code(), codes::E_DEP_CYCLE);
        let diag = err.to_diagnostic();
        assert!(diag.labels.is_some());
    }

    #[test]
    fn test_diagnostic_json_serialization() {
        let diag = Diagnostic {
            code: codes::E_LINT_DEPTH_EXCEEDED,
            severity: Severity::Error,
            message: "Task nesting exceeds maximum depth".to_string(),
            location: Some(Location::new(PathBuf::from("tasks.md"), 42, 5)),
            snippet: None,
            help: Some("Reduce nesting level to 3 or fewer".to_string()),
            labels: None,
            recovery_command: None,
            fix_steps: None,
            explanation: None,
            docs_url: None,
        };

        let json = diag.to_json().unwrap();
        assert!(json.contains("E_LINT_DEPTH_EXCEEDED"));
        assert!(json.contains("tasks.md"));
        assert!(json.contains("42"));
    }

    #[test]
    fn test_location_display() {
        let loc = Location::new(PathBuf::from("/path/to/file.md"), 10, 5);
        let display = format!("{loc}");
        assert!(display.contains("file.md"));
        assert!(display.contains("10"));
        assert!(display.contains('5'));
    }

    #[test]
    fn test_location_file_only() {
        let loc = Location::file_only(PathBuf::from("test.md"));
        assert_eq!(loc.line, None);
        assert_eq!(loc.column, None);
    }

    #[test]
    fn test_severity_ordering() {
        // With derived Ord, earlier variants are < later variants
        // But semantically, Error is more severe than Warning
        assert!(Severity::Error < Severity::Warning);
        assert!(Severity::Warning < Severity::Info);
        assert!(Severity::Info < Severity::Hint);

        // Test that we can still compare them
        assert_ne!(Severity::Error, Severity::Warning);
        assert_eq!(Severity::Error, Severity::Error);
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(format!("{}", Severity::Error), "error");
        assert_eq!(format!("{}", Severity::Warning), "warning");
        assert_eq!(format!("{}", Severity::Info), "info");
        assert_eq!(format!("{}", Severity::Hint), "hint");
    }

    #[test]
    fn test_diagnostic_builder_methods() {
        let diag = Diagnostic {
            code: codes::E_LINT_DEPTH_EXCEEDED,
            severity: Severity::Error,
            message: "Too deep".to_string(),
            location: None,
            snippet: None,
            help: None,
            labels: None,
            recovery_command: None,
            fix_steps: None,
            explanation: None,
            docs_url: None,
        };

        let with_help = diag.clone().with_help("Flatten hierarchy");
        assert_eq!(with_help.help, Some("Flatten hierarchy".to_string()));

        let with_snippet = diag.clone().with_snippet("- [ ] task");
        assert_eq!(with_snippet.snippet, Some("- [ ] task".to_string()));

        let with_labels = diag.with_labels(vec![("key".to_string(), "value".to_string())]);
        assert!(with_labels.labels.is_some());
    }

    #[test]
    fn test_io_errors() {
        let err = LashError::io_file_not_found(PathBuf::from("missing.md"));
        assert_eq!(err.code(), codes::E_IO_FILE_NOT_FOUND);

        let err = LashError::io_read_error(PathBuf::from("test.md"), "disk error");
        assert_eq!(err.code(), codes::E_IO_READ_ERROR);

        let err = LashError::io_permission_denied(PathBuf::from("locked.md"));
        assert_eq!(err.code(), codes::E_IO_PERMISSION_DENIED);
    }

    #[test]
    fn test_config_errors() {
        let err = LashError::config_root_not_found(PathBuf::from("/tmp"));
        assert_eq!(err.code(), codes::E_CONFIG_ROOT_NOT_FOUND);

        let err = LashError::config_invalid_value("max_depth", "invalid");
        assert_eq!(err.code(), codes::E_CONFIG_INVALID_VALUE);

        let err = LashError::config_missing_index();
        assert_eq!(err.code(), codes::E_CONFIG_MISSING_INDEX);
    }

    #[test]
    fn test_index_errors() {
        let err = LashError::index_corrupted("integrity check failed");
        assert_eq!(err.code(), codes::E_INDEX_CORRUPTED);

        let err = LashError::index_version_mismatch(1, 2);
        assert_eq!(err.code(), codes::E_INDEX_VERSION_MISMATCH);

        let err = LashError::index_out_of_sync(5);
        assert_eq!(err.code(), codes::E_INDEX_OUT_OF_SYNC);
    }

    #[test]
    fn test_query_errors() {
        let err = LashError::query_invalid_syntax("@invalid");
        assert_eq!(err.code(), codes::E_QUERY_INVALID_SYNTAX);

        let err = LashError::query_no_results("nonexistent");
        assert_eq!(err.code(), codes::E_QUERY_NO_RESULTS);
    }

    #[test]
    fn test_internal_error() {
        let err = LashError::internal("unexpected state", Some("debug info".to_string()));
        assert_eq!(err.code(), codes::E_INTERNAL);
        let diag = err.to_diagnostic();
        assert!(diag.help.as_ref().unwrap().contains("bug"));
    }

    #[test]
    fn test_error_code_stability() {
        // These codes must never change - they're part of the stable API
        assert_eq!(codes::E_PARSE_INVALID_CHECKBOX, "E_PARSE_INVALID_CHECKBOX");
        assert_eq!(codes::E_LINT_DUPLICATE_ID, "E_LINT_DUPLICATE_ID");
        assert_eq!(codes::E_DEP_CYCLE, "E_DEP_CYCLE");
        assert_eq!(codes::E_INDEX_CORRUPTED, "E_INDEX_CORRUPTED");
        assert_eq!(codes::E_CONFIG_ROOT_NOT_FOUND, "E_CONFIG_ROOT_NOT_FOUND");
        assert_eq!(codes::E_IO_FILE_NOT_FOUND, "E_IO_FILE_NOT_FOUND");
        assert_eq!(codes::E_QUERY_INVALID_SYNTAX, "E_QUERY_INVALID_SYNTAX");
    }

    // ===== Exit Code Tests =====

    #[test]
    fn test_exit_code_values() {
        // Exit codes must remain stable - they're part of the CLI contract
        assert_eq!(ExitCode::Success as i32, 0);
        assert_eq!(ExitCode::GeneralError as i32, 1);
        assert_eq!(ExitCode::LintError as i32, 2);
        assert_eq!(ExitCode::IndexError as i32, 3);
        assert_eq!(ExitCode::ConfigError as i32, 4);
        assert_eq!(ExitCode::NotFound as i32, 5);
        assert_eq!(ExitCode::CycleDetected as i32, 6);
    }

    #[test]
    fn test_exit_code_as_i32() {
        assert_eq!(ExitCode::Success.as_i32(), 0);
        assert_eq!(ExitCode::LintError.as_i32(), 2);
        assert_eq!(ExitCode::CycleDetected.as_i32(), 6);
    }

    #[test]
    fn test_exit_code_display() {
        assert_eq!(format!("{}", ExitCode::Success), "0 (success)");
        assert_eq!(format!("{}", ExitCode::LintError), "2 (lint error)");
        assert_eq!(format!("{}", ExitCode::NotFound), "5 (not found)");
    }

    #[test]
    fn test_exit_code_from_parse_error() {
        let err = LashError::parse_invalid_checkbox(PathBuf::from("test.md"), 5, 3, "[*] invalid");
        assert_eq!(ExitCode::from(&err), ExitCode::LintError);
    }

    #[test]
    fn test_exit_code_from_lint_error() {
        let err = LashError::lint_duplicate_id(PathBuf::from("test.md"), 10, 5, "task-id", 5);
        assert_eq!(ExitCode::from(&err), ExitCode::LintError);
    }

    #[test]
    fn test_exit_code_from_index_error() {
        let err = LashError::index_corrupted("corruption details");
        assert_eq!(ExitCode::from(&err), ExitCode::IndexError);

        let err = LashError::index_version_mismatch(1, 2);
        assert_eq!(ExitCode::from(&err), ExitCode::IndexError);

        let err = LashError::index_out_of_sync(5);
        assert_eq!(ExitCode::from(&err), ExitCode::IndexError);
    }

    #[test]
    fn test_exit_code_from_config_error() {
        let err = LashError::config_root_not_found(PathBuf::from("/tmp"));
        assert_eq!(ExitCode::from(&err), ExitCode::ConfigError);

        let err = LashError::config_invalid_value("key", "value");
        assert_eq!(ExitCode::from(&err), ExitCode::ConfigError);

        let err = LashError::config_missing_index();
        assert_eq!(ExitCode::from(&err), ExitCode::ConfigError);
    }

    #[test]
    fn test_exit_code_from_dependency_cycle() {
        let chain = vec![
            "task1".to_string(),
            "task2".to_string(),
            "task1".to_string(),
        ];
        let err = LashError::dep_cycle(&chain);
        assert_eq!(ExitCode::from(&err), ExitCode::CycleDetected);
    }

    #[test]
    fn test_exit_code_from_dependency_not_found() {
        let err =
            LashError::dep_not_found(PathBuf::from("test.md"), 5, 3, "path/to/task.md#task:id");
        assert_eq!(ExitCode::from(&err), ExitCode::NotFound);
    }

    #[test]
    fn test_exit_code_from_dependency_invalid_ref() {
        let err = LashError::dep_invalid_ref(PathBuf::from("test.md"), 5, 3, "invalid-ref");
        assert_eq!(ExitCode::from(&err), ExitCode::GeneralError);
    }

    #[test]
    fn test_exit_code_from_io_file_not_found() {
        let err = LashError::io_file_not_found(PathBuf::from("missing.md"));
        assert_eq!(ExitCode::from(&err), ExitCode::NotFound);
    }

    #[test]
    fn test_exit_code_from_io_other_errors() {
        let err = LashError::io_read_error(PathBuf::from("test.md"), "disk error");
        assert_eq!(ExitCode::from(&err), ExitCode::GeneralError);

        let err = LashError::io_permission_denied(PathBuf::from("locked.md"));
        assert_eq!(ExitCode::from(&err), ExitCode::GeneralError);
    }

    #[test]
    fn test_exit_code_from_query_no_results() {
        let err = LashError::query_no_results("search term");
        assert_eq!(ExitCode::from(&err), ExitCode::NotFound);
    }

    #[test]
    fn test_exit_code_from_query_invalid_syntax() {
        let err = LashError::query_invalid_syntax("@invalid");
        assert_eq!(ExitCode::from(&err), ExitCode::GeneralError);
    }

    #[test]
    fn test_exit_code_from_internal_error() {
        let err = LashError::internal("unexpected state", Some("context".to_string()));
        assert_eq!(ExitCode::from(&err), ExitCode::GeneralError);
    }

    // ===== Parse Error Constructor Tests =====

    #[test]
    fn test_parse_invalid_annotation() {
        let err = LashError::parse_invalid_annotation(
            PathBuf::from("test.md"),
            10,
            5,
            "@invalid: value",
            "invalid",
        );
        assert_eq!(err.code(), codes::E_PARSE_INVALID_ANNOTATION);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains("invalid"));
        assert!(diag.help.as_ref().unwrap().contains("@key: value"));
        assert_eq!(diag.severity, Severity::Error);
    }

    #[test]
    fn test_parse_invalid_header() {
        let err = LashError::parse_invalid_header(PathBuf::from("test.md"), 1, "##Invalid");
        assert_eq!(err.code(), codes::E_PARSE_INVALID_HEADER);
        let diag = err.to_diagnostic();
        assert_eq!(diag.message, "invalid header format");
        assert!(diag.snippet.is_some());
        assert!(diag.help.as_ref().unwrap().contains("space"));
    }

    #[test]
    fn test_parse_unexpected_depth() {
        let err = LashError::parse_unexpected_depth(PathBuf::from("test.md"), 15, 7, 4);
        assert_eq!(err.code(), codes::E_PARSE_UNEXPECTED_DEPTH);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains('4'));
        assert!(diag.help.as_ref().unwrap().contains("2 spaces"));
    }

    #[test]
    fn test_parse_invalid_date() {
        let err = LashError::parse_invalid_date(PathBuf::from("test.md"), 5, 10, "2024-13-45");
        assert_eq!(err.code(), codes::E_PARSE_INVALID_DATE);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains("2024-13-45"));
        assert!(diag.help.as_ref().unwrap().contains("YYYY-MM-DD"));
    }

    // ===== Lint Error Constructor Tests =====

    #[test]
    fn test_lint_unknown_annotation() {
        let err =
            LashError::lint_unknown_annotation(PathBuf::from("test.md"), 8, 1, "unknown-anno");
        assert_eq!(err.code(), codes::E_LINT_UNKNOWN_ANNOTATION);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains("@unknown-anno"));
        assert!(diag.help.as_ref().unwrap().contains("@id"));
    }

    #[test]
    fn test_lint_depth_exceeded() {
        let err = LashError::lint_depth_exceeded(PathBuf::from("test.md"), 20, 9, 5, 4);
        assert_eq!(err.code(), codes::E_LINT_DEPTH_EXCEEDED);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains('5'));
        assert!(diag.message.contains('4'));
        assert!(diag.help.as_ref().unwrap().contains('4'));
    }

    #[test]
    fn test_lint_status_inconsistency() {
        let err = LashError::lint_status_inconsistency(PathBuf::from("test.md"), 12, 1, "done");
        assert_eq!(err.code(), codes::E_LINT_STATUS_INCONSISTENCY);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains("parent task"));
        assert!(diag.help.as_ref().unwrap().contains("done"));
    }

    #[test]
    fn test_lint_invalid_label() {
        let err = LashError::lint_invalid_label(PathBuf::from("test.md"), 7, 8, "invalid label!");
        assert_eq!(err.code(), codes::E_LINT_INVALID_LABEL);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains("invalid label!"));
        assert!(diag.snippet.as_ref().unwrap().contains("@labels"));
        assert!(diag.help.as_ref().unwrap().contains("alphanumeric"));
    }

    #[test]
    fn test_lint_missing_annotation() {
        let err = LashError::lint_missing_annotation(PathBuf::from("test.md"), 5, "id");
        assert_eq!(err.code(), codes::E_LINT_MISSING_ANNOTATION);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains("@id"));
        assert!(diag.help.as_ref().unwrap().contains("add @id"));
    }

    #[test]
    fn test_lint_bad_indentation() {
        let err = LashError::lint_bad_indentation(PathBuf::from("test.md"), 10, 1, 3, 4);
        assert_eq!(err.code(), codes::E_LINT_BAD_INDENTATION);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains('3'));
        assert!(diag.message.contains('4'));
        assert!(diag.help.as_ref().unwrap().contains("lash format"));
    }

    // ===== Dependency Error Constructor Tests =====

    #[test]
    fn test_dep_not_found() {
        let err = LashError::dep_not_found(
            PathBuf::from("test.md"),
            10,
            5,
            "path/to/missing.md#task:id",
        );
        assert_eq!(err.code(), codes::E_DEP_NOT_FOUND);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains("missing.md"));
        assert!(diag.help.is_some());
    }

    #[test]
    fn test_dep_invalid_ref() {
        let err = LashError::dep_invalid_ref(PathBuf::from("test.md"), 8, 3, "invalid-ref-format");
        assert_eq!(err.code(), codes::E_DEP_INVALID_REF);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains("invalid-ref-format"));
        assert!(diag
            .help
            .as_ref()
            .unwrap()
            .contains("path/to/file.md#task:id"));
    }

    #[test]
    fn test_dep_cycle_empty_chain() {
        let chain: Vec<String> = vec![];
        let err = LashError::dep_cycle(&chain);
        assert_eq!(err.code(), codes::E_DEP_CYCLE);
        let diag = err.to_diagnostic();
        assert_eq!(diag.message, "circular dependency detected");
    }

    // ===== Index Error Constructor Tests =====

    #[test]
    fn test_index_corrupted() {
        let err = LashError::index_corrupted("SQLite integrity check failed");
        assert_eq!(err.code(), codes::E_INDEX_CORRUPTED);
        let diag = err.to_diagnostic();
        assert!(diag.labels.is_some());
        let labels = diag.labels.unwrap();
        assert_eq!(labels[0].0, "context");
        assert!(labels[0].1.contains("integrity"));
    }

    #[test]
    fn test_index_version_mismatch_message() {
        let err = LashError::index_version_mismatch(2, 5);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains('2'));
        assert!(diag.message.contains('5'));
        assert!(diag.help.as_ref().unwrap().contains("migrate"));
    }

    #[test]
    fn test_index_out_of_sync_message() {
        let err = LashError::index_out_of_sync(42);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains("42"));
        assert!(diag.help.as_ref().unwrap().contains("lash index"));
    }

    // ===== Query Error Constructor Tests =====

    #[test]
    fn test_query_invalid_syntax() {
        let err = LashError::query_invalid_syntax("@invalid::syntax");
        assert_eq!(err.code(), codes::E_QUERY_INVALID_SYNTAX);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains("@invalid::syntax"));
        assert!(diag.help.as_ref().unwrap().contains("help search"));
    }

    #[test]
    fn test_query_no_results() {
        let err = LashError::query_no_results("nonexistent-term");
        assert_eq!(err.code(), codes::E_QUERY_NO_RESULTS);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains("nonexistent-term"));
        assert!(diag.help.is_some());
    }

    // ===== Config Error Constructor Tests =====

    #[test]
    fn test_config_root_not_found() {
        let err = LashError::config_root_not_found(PathBuf::from("/home/user/project"));
        assert_eq!(err.code(), codes::E_CONFIG_ROOT_NOT_FOUND);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains("root directory"));
        assert!(diag.location.is_some());
        assert_eq!(
            diag.location.as_ref().unwrap().file_path,
            PathBuf::from("/home/user/project")
        );
    }

    #[test]
    fn test_config_invalid_value() {
        let err = LashError::config_invalid_value("max_depth", "not-a-number");
        assert_eq!(err.code(), codes::E_CONFIG_INVALID_VALUE);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains("max_depth"));
        assert!(diag.message.contains("not-a-number"));
        assert!(diag.help.as_ref().unwrap().contains("max_depth"));
    }

    #[test]
    fn test_config_parse_error() {
        let err = LashError::config_parse_error(
            PathBuf::from("lash.config.toml"),
            "unexpected token at line 5",
        );
        assert_eq!(err.code(), codes::E_CONFIG_PARSE_ERROR);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains("unexpected token"));
        assert!(diag.help.as_ref().unwrap().contains("TOML"));
        assert_eq!(
            diag.location.as_ref().unwrap().file_path,
            PathBuf::from("lash.config.toml")
        );
    }

    #[test]
    fn test_config_missing_index() {
        let err = LashError::config_missing_index();
        assert_eq!(err.code(), codes::E_CONFIG_MISSING_INDEX);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains("lash.index.md"));
        assert!(diag.help.is_some());
    }

    // ===== IO Error Constructor Tests =====

    #[test]
    fn test_io_file_not_found() {
        let err = LashError::io_file_not_found(PathBuf::from("nonexistent.md"));
        assert_eq!(err.code(), codes::E_IO_FILE_NOT_FOUND);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains("nonexistent.md"));
    }

    #[test]
    fn test_io_read_error() {
        let err = LashError::io_read_error(PathBuf::from("test.md"), "Permission denied");
        assert_eq!(err.code(), codes::E_IO_READ_ERROR);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains("test.md"));
        assert!(diag.labels.is_some());
        let labels = diag.labels.unwrap();
        assert_eq!(labels[0].0, "underlying error");
        assert!(labels[0].1.contains("Permission denied"));
    }

    #[test]
    fn test_io_write_error() {
        let err = LashError::io_write_error(PathBuf::from("output.md"), "Disk full");
        assert_eq!(err.code(), codes::E_IO_WRITE_ERROR);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains("output.md"));
        assert!(diag.labels.is_some());
    }

    #[test]
    fn test_io_permission_denied() {
        let err = LashError::io_permission_denied(PathBuf::from("readonly.md"));
        assert_eq!(err.code(), codes::E_IO_PERMISSION_DENIED);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains("permission denied"));
        assert!(diag.message.contains("readonly.md"));
    }

    #[test]
    fn test_io_invalid_path() {
        let err =
            LashError::io_invalid_path(PathBuf::from("../../../etc/passwd"), "path traversal");
        assert_eq!(err.code(), codes::E_IO_INVALID_PATH);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains("invalid path"));
        assert!(diag.labels.is_some());
    }

    // ===== Internal Error Constructor Tests =====

    #[test]
    fn test_internal_error_with_context() {
        let err = LashError::internal("panic in worker thread", Some("thread_id: 42".to_string()));
        assert_eq!(err.code(), codes::E_INTERNAL);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains("panic"));
        assert!(diag.help.as_ref().unwrap().contains("bug"));
        assert!(diag.labels.is_some());
    }

    #[test]
    fn test_internal_error_without_context() {
        let err = LashError::internal("unexpected null pointer", None);
        assert_eq!(err.code(), codes::E_INTERNAL);
        let diag = err.to_diagnostic();
        assert_eq!(diag.labels, None);
        assert!(diag.help.is_some());
    }

    // ===== Display Trait Tests =====

    #[test]
    fn test_parse_error_display() {
        let err = LashError::parse_invalid_checkbox(PathBuf::from("test.md"), 5, 3, "[*] invalid");
        let display = format!("{err}");
        assert!(display.contains("parse error"));
        assert!(display.contains("invalid checkbox syntax"));
    }

    #[test]
    fn test_lint_error_display() {
        let err = LashError::lint_duplicate_id(PathBuf::from("test.md"), 10, 5, "task-id", 5);
        let display = format!("{err}");
        assert!(display.contains("lint error"));
        assert!(display.contains("task-id"));
    }

    #[test]
    fn test_index_error_display() {
        let err = LashError::index_corrupted("details");
        let display = format!("{err}");
        assert!(display.contains("index error"));
        assert!(display.contains("corrupted"));
    }

    #[test]
    fn test_dependency_error_display() {
        let err = LashError::dep_not_found(PathBuf::from("test.md"), 5, 3, "path/to/missing.md#id");
        let display = format!("{err}");
        assert!(display.contains("dependency error"));
    }

    #[test]
    fn test_query_error_display() {
        let err = LashError::query_invalid_syntax("@bad");
        let display = format!("{err}");
        assert!(display.contains("query error"));
    }

    #[test]
    fn test_config_error_display() {
        let err = LashError::config_missing_index();
        let display = format!("{err}");
        assert!(display.contains("configuration error"));
    }

    #[test]
    fn test_io_error_display() {
        let err = LashError::io_file_not_found(PathBuf::from("missing.md"));
        let display = format!("{err}");
        assert!(display.contains("I/O error"));
    }

    #[test]
    fn test_internal_error_display() {
        let err = LashError::internal("unexpected", None);
        let display = format!("{err}");
        assert!(display.contains("internal error"));
    }

    // ===== Location Tests =====

    #[test]
    fn test_location_with_span() {
        let loc = Location::new(PathBuf::from("test.md"), 5, 10).with_span(20, 30);
        assert_eq!(loc.span, Some((20, 30)));
        assert_eq!(loc.line, Some(5));
        assert_eq!(loc.column, Some(10));
    }

    #[test]
    fn test_location_display_without_column() {
        let mut loc = Location::new(PathBuf::from("test.md"), 10, 5);
        loc.column = None;
        let display = format!("{loc}");
        assert!(display.contains("test.md"));
        assert!(display.contains("10"));
        assert!(!display.contains(":5"));
    }

    #[test]
    fn test_location_display_file_only() {
        let loc = Location::file_only(PathBuf::from("test.md"));
        let display = format!("{loc}");
        assert_eq!(display, "test.md");
    }

    // ===== Diagnostic Tests =====

    #[test]
    fn test_diagnostic_display_full() {
        let diag = Diagnostic {
            code: codes::E_LINT_DEPTH_EXCEEDED,
            severity: Severity::Error,
            message: "Too deep".to_string(),
            location: Some(Location::new(PathBuf::from("test.md"), 10, 5)),
            snippet: Some("- [ ] nested task".to_string()),
            help: Some("Flatten hierarchy".to_string()),
            labels: Some(vec![("context".to_string(), "level 5".to_string())]),
            recovery_command: None,
            fix_steps: None,
            explanation: None,
            docs_url: None,
        };

        let display = format!("{diag}");
        assert!(display.contains("E_LINT_DEPTH_EXCEEDED"));
        assert!(display.contains("error"));
        assert!(display.contains("Too deep"));
        assert!(display.contains("test.md:10:5"));
        assert!(display.contains("snippet"));
        assert!(display.contains("help"));
        assert!(display.contains("context"));
    }

    #[test]
    fn test_diagnostic_display_minimal() {
        let diag = Diagnostic {
            code: codes::E_INTERNAL,
            severity: Severity::Error,
            message: "Internal error".to_string(),
            location: None,
            snippet: None,
            help: None,
            labels: None,
            recovery_command: None,
            fix_steps: None,
            explanation: None,
            docs_url: None,
        };

        let display = format!("{diag}");
        assert!(display.contains("E_INTERNAL"));
        assert!(display.contains("Internal error"));
        assert!(!display.contains("at "));
        assert!(!display.contains("snippet:"));
        assert!(!display.contains("help:"));
    }

    #[test]
    fn test_diagnostic_json_serialization_full() {
        let diag = Diagnostic {
            code: codes::E_PARSE_INVALID_CHECKBOX,
            severity: Severity::Error,
            message: "Invalid checkbox".to_string(),
            location: Some(Location::new(PathBuf::from("test.md"), 5, 3)),
            snippet: Some("[*] invalid".to_string()),
            help: Some("Use [ ], [-], [x], or [!]".to_string()),
            labels: None,
            recovery_command: None,
            fix_steps: None,
            explanation: None,
            docs_url: None,
        };

        let json = diag.to_json().unwrap();
        assert!(json.contains("E_PARSE_INVALID_CHECKBOX"));
        assert!(json.contains("test.md"));
        assert!(json.contains("\"line\": 5"));
        assert!(json.contains("\"column\": 3"));
        assert!(json.contains("[*] invalid"));
    }

    #[test]
    fn test_diagnostic_json_omits_none_fields() {
        let diag = Diagnostic {
            code: codes::E_INTERNAL,
            severity: Severity::Warning,
            message: "Warning message".to_string(),
            location: None,
            snippet: None,
            help: None,
            labels: None,
            recovery_command: None,
            fix_steps: None,
            explanation: None,
            docs_url: None,
        };

        let json = diag.to_json().unwrap();
        // None fields should not appear in JSON due to skip_serializing_if
        assert!(!json.contains("\"location\""));
        assert!(!json.contains("\"snippet\""));
        assert!(!json.contains("\"help\""));
        assert!(!json.contains("\"labels\""));
    }

    // ===== Edge Cases Tests =====

    #[test]
    fn test_empty_message_handling() {
        let err = LashError::internal("", None);
        let diag = err.to_diagnostic();
        assert_eq!(diag.message, "");
    }

    #[test]
    fn test_very_long_message() {
        let long_msg = "x".repeat(10000);
        let err = LashError::internal(&long_msg, None);
        let diag = err.to_diagnostic();
        assert_eq!(diag.message.len(), 10000);
    }

    #[test]
    fn test_special_characters_in_messages() {
        let err = LashError::lint_unknown_annotation(
            PathBuf::from("test.md"),
            1,
            1,
            "anno-with-émojis-🚀-and-symbols-©®™",
        );
        let diag = err.to_diagnostic();
        assert!(diag.message.contains("🚀"));
        assert!(diag.message.contains("©"));
    }

    #[test]
    fn test_newlines_in_messages() {
        let err = LashError::internal("Line 1\nLine 2\nLine 3", None);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains('\n'));
    }

    #[test]
    fn test_path_with_unicode() {
        let path = PathBuf::from("/path/to/файл.md"); // Cyrillic characters
        let err = LashError::io_file_not_found(path);
        let diag = err.to_diagnostic();
        assert!(diag.message.contains("файл.md"));
    }

    #[test]
    fn test_multiple_labels_in_diagnostic() {
        let labels = vec![
            ("label1".to_string(), "value1".to_string()),
            ("label2".to_string(), "value2".to_string()),
            ("label3".to_string(), "value3".to_string()),
        ];
        let diag = Diagnostic {
            code: codes::E_INTERNAL,
            severity: Severity::Error,
            message: "Test".to_string(),
            location: None,
            snippet: None,
            help: None,
            labels: None,
            recovery_command: None,
            fix_steps: None,
            explanation: None,
            docs_url: None,
        }
        .with_labels(labels);

        let display = format!("{diag}");
        assert!(display.contains("label1: value1"));
        assert!(display.contains("label2: value2"));
        assert!(display.contains("label3: value3"));
    }

    #[test]
    fn test_diagnostic_builder_chaining() {
        let diag = Diagnostic {
            code: codes::E_PARSE_INVALID_CHECKBOX,
            severity: Severity::Error,
            message: "Test".to_string(),
            location: None,
            snippet: None,
            help: None,
            labels: None,
            recovery_command: None,
            fix_steps: None,
            explanation: None,
            docs_url: None,
        }
        .with_help("Help text")
        .with_snippet("Code snippet")
        .with_labels(vec![("key".to_string(), "value".to_string())]);

        assert_eq!(diag.help, Some("Help text".to_string()));
        assert_eq!(diag.snippet, Some("Code snippet".to_string()));
        assert!(diag.labels.is_some());
    }

    // ===== Exit Code Equality and Copy Tests =====

    #[test]
    fn test_exit_code_equality() {
        assert_eq!(ExitCode::Success, ExitCode::Success);
        assert_ne!(ExitCode::Success, ExitCode::GeneralError);
        assert_eq!(ExitCode::LintError, ExitCode::LintError);
    }

    #[test]
    fn test_exit_code_clone_and_copy() {
        let code = ExitCode::LintError;
        let cloned = code;
        assert_eq!(code, cloned);
        assert_eq!(code.as_i32(), cloned.as_i32());
    }

    // ===== Severity Serialization Tests =====

    #[test]
    fn test_severity_serialization() {
        let json = serde_json::to_string(&Severity::Error).unwrap();
        assert_eq!(json, "\"error\"");

        let json = serde_json::to_string(&Severity::Warning).unwrap();
        assert_eq!(json, "\"warning\"");

        let json = serde_json::to_string(&Severity::Info).unwrap();
        assert_eq!(json, "\"info\"");

        let json = serde_json::to_string(&Severity::Hint).unwrap();
        assert_eq!(json, "\"hint\"");
    }

    #[test]
    fn test_severity_deserialization() {
        let sev: Severity = serde_json::from_str("\"error\"").unwrap();
        assert_eq!(sev, Severity::Error);

        let sev: Severity = serde_json::from_str("\"warning\"").unwrap();
        assert_eq!(sev, Severity::Warning);
    }

    // ===== Location Equality Tests =====

    #[test]
    fn test_location_equality() {
        let loc1 = Location::new(PathBuf::from("test.md"), 10, 5);
        let loc2 = Location::new(PathBuf::from("test.md"), 10, 5);
        assert_eq!(loc1, loc2);

        let loc3 = Location::new(PathBuf::from("test.md"), 10, 6);
        assert_ne!(loc1, loc3);
    }

    #[test]
    fn test_location_with_different_spans() {
        let loc1 = Location::new(PathBuf::from("test.md"), 5, 3).with_span(10, 20);
        let loc2 = Location::new(PathBuf::from("test.md"), 5, 3).with_span(10, 30);
        assert_ne!(loc1, loc2);
    }

    // ===== Error Code Access Tests =====

    #[test]
    fn test_all_error_variants_return_correct_codes() {
        let test_cases = vec![
            (
                LashError::Parse {
                    code: codes::E_PARSE_INVALID_CHECKBOX,
                    message: "test".to_string(),
                    location: None,
                    snippet: None,
                    help: None,
                },
                codes::E_PARSE_INVALID_CHECKBOX,
            ),
            (
                LashError::Lint {
                    code: codes::E_LINT_DUPLICATE_ID,
                    message: "test".to_string(),
                    location: None,
                    snippet: None,
                    help: None,
                },
                codes::E_LINT_DUPLICATE_ID,
            ),
            (
                LashError::Index {
                    code: codes::E_INDEX_CORRUPTED,
                    message: "test".to_string(),
                    context: None,
                    help: None,
                },
                codes::E_INDEX_CORRUPTED,
            ),
            (
                LashError::Dependency {
                    code: codes::E_DEP_CYCLE,
                    message: "test".to_string(),
                    location: None,
                    chain: None,
                    help: None,
                },
                codes::E_DEP_CYCLE,
            ),
            (
                LashError::Query {
                    code: codes::E_QUERY_INVALID_SYNTAX,
                    message: "test".to_string(),
                    help: None,
                },
                codes::E_QUERY_INVALID_SYNTAX,
            ),
            (
                LashError::Config {
                    code: codes::E_CONFIG_INVALID_VALUE,
                    message: "test".to_string(),
                    path: None,
                    help: None,
                },
                codes::E_CONFIG_INVALID_VALUE,
            ),
            (
                LashError::IO {
                    code: codes::E_IO_READ_ERROR,
                    message: "test".to_string(),
                    path: None,
                    io_error: None,
                },
                codes::E_IO_READ_ERROR,
            ),
            (
                LashError::Internal {
                    code: codes::E_INTERNAL,
                    message: "test".to_string(),
                    context: None,
                },
                codes::E_INTERNAL,
            ),
        ];

        for (error, expected_code) in test_cases {
            assert_eq!(error.code(), expected_code);
        }
    }

    // ===== Diagnostic to_diagnostic Coverage Tests =====

    #[test]
    fn test_dependency_diagnostic_with_chain() {
        let chain = vec![
            "task1".to_string(),
            "task2".to_string(),
            "task3".to_string(),
        ];
        let err = LashError::Dependency {
            code: codes::E_DEP_CYCLE,
            message: "cycle detected".to_string(),
            location: Some(Location::new(PathBuf::from("test.md"), 5, 3)),
            chain: Some(chain.clone()),
            help: Some("break the cycle".to_string()),
        };

        let diag = err.to_diagnostic();
        assert!(diag.labels.is_some());
        let labels = diag.labels.unwrap();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].0, "dependency chain");
        assert_eq!(labels[0].1, "task1 -> task2 -> task3");
    }

    #[test]
    fn test_dependency_diagnostic_without_chain() {
        let err = LashError::Dependency {
            code: codes::E_DEP_NOT_FOUND,
            message: "not found".to_string(),
            location: None,
            chain: None,
            help: None,
        };

        let diag = err.to_diagnostic();
        assert_eq!(diag.labels, None);
    }

    #[test]
    fn test_index_diagnostic_with_context() {
        let err = LashError::Index {
            code: codes::E_INDEX_CORRUPTED,
            message: "corrupted".to_string(),
            context: Some("integrity check failed".to_string()),
            help: Some("rebuild".to_string()),
        };

        let diag = err.to_diagnostic();
        assert!(diag.labels.is_some());
        let labels = diag.labels.unwrap();
        assert_eq!(labels[0].0, "context");
    }

    #[test]
    fn test_index_diagnostic_without_context() {
        let err = LashError::Index {
            code: codes::E_INDEX_OUT_OF_SYNC,
            message: "out of sync".to_string(),
            context: None,
            help: None,
        };

        let diag = err.to_diagnostic();
        assert_eq!(diag.labels, None);
    }

    #[test]
    fn test_config_diagnostic_with_path() {
        let err = LashError::Config {
            code: codes::E_CONFIG_PARSE_ERROR,
            message: "parse error".to_string(),
            path: Some(PathBuf::from("config.toml")),
            help: None,
        };

        let diag = err.to_diagnostic();
        assert!(diag.location.is_some());
        let loc = diag.location.unwrap();
        assert_eq!(loc.file_path, PathBuf::from("config.toml"));
        assert_eq!(loc.line, None);
        assert_eq!(loc.column, None);
    }

    #[test]
    fn test_config_diagnostic_without_path() {
        let err = LashError::Config {
            code: codes::E_CONFIG_INVALID_VALUE,
            message: "invalid value".to_string(),
            path: None,
            help: None,
        };

        let diag = err.to_diagnostic();
        assert_eq!(diag.location, None);
    }

    #[test]
    fn test_io_diagnostic_with_path_and_error() {
        let err = LashError::IO {
            code: codes::E_IO_READ_ERROR,
            message: "read failed".to_string(),
            path: Some(PathBuf::from("file.md")),
            io_error: Some("permission denied".to_string()),
        };

        let diag = err.to_diagnostic();
        assert!(diag.location.is_some());
        assert!(diag.labels.is_some());
        let labels = diag.labels.unwrap();
        assert_eq!(labels[0].0, "underlying error");
        assert_eq!(labels[0].1, "permission denied");
    }

    #[test]
    fn test_io_diagnostic_without_io_error() {
        let err = LashError::IO {
            code: codes::E_IO_FILE_NOT_FOUND,
            message: "not found".to_string(),
            path: Some(PathBuf::from("missing.md")),
            io_error: None,
        };

        let diag = err.to_diagnostic();
        assert_eq!(diag.labels, None);
    }

    #[test]
    fn test_internal_diagnostic_always_has_help() {
        let err = LashError::Internal {
            code: codes::E_INTERNAL,
            message: "internal error".to_string(),
            context: None,
        };

        let diag = err.to_diagnostic();
        assert!(diag.help.is_some());
        assert!(diag.help.unwrap().contains("bug"));
    }
}
