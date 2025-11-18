//! Error types and diagnostic structures for Lash

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for Lash operations
pub type Result<T> = std::result::Result<T, LashError>;

/// Main error type for all Lash operations
#[derive(Error, Debug, Clone)]
pub enum LashError {
    /// Markdown parsing failures
    #[error("Parse error: {message}")]
    ParseError {
        code: &'static str,
        message: String,
        location: Option<Location>,
    },

    /// Validation/linting failures
    #[error("Lint error: {message}")]
    LintError {
        code: &'static str,
        message: String,
        location: Option<Location>,
    },

    /// File system errors
    #[error("I/O error: {message}")]
    IoError {
        code: &'static str,
        message: String,
        path: Option<PathBuf>,
    },

    /// Database errors
    #[error("Database error: {message}")]
    DatabaseError { code: &'static str, message: String },

    /// Configuration issues
    #[error("Configuration error: {message}")]
    ConfigError { code: &'static str, message: String },

    /// Dependency errors (broken references, cycles)
    #[error("Dependency error: {message}")]
    DependencyError {
        code: &'static str,
        message: String,
        location: Option<Location>,
    },
}

impl LashError {
    /// Get the error code for this error
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::ParseError { code, .. }
            | Self::LintError { code, .. }
            | Self::IoError { code, .. }
            | Self::DatabaseError { code, .. }
            | Self::ConfigError { code, .. }
            | Self::DependencyError { code, .. } => code,
        }
    }

    /// Convert this error to a Diagnostic
    #[must_use]
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            Self::ParseError {
                code,
                message,
                location,
            }
            | Self::LintError {
                code,
                message,
                location,
            }
            | Self::DependencyError {
                code,
                message,
                location,
            } => Diagnostic {
                code,
                severity: Severity::Error,
                message: message.clone(),
                location: location.clone(),
                suggestion: None,
            },
            Self::IoError {
                code,
                message,
                path,
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
                suggestion: None,
            },
            Self::DatabaseError { code, message } | Self::ConfigError { code, message } => {
                Diagnostic {
                    code,
                    severity: Severity::Error,
                    message: message.clone(),
                    location: None,
                    suggestion: None,
                }
            }
        }
    }
}

/// Diagnostic structure for error reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Error code (e.g., `E_PARSE_BAD_CHECKBOX`)
    pub code: &'static str,

    /// Severity level
    pub severity: Severity,

    /// Human-readable error message
    pub message: String,

    /// Location where the error occurred
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,

    /// Optional suggestion for how to fix
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
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
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.code, self.severity, self.message)?;

        if let Some(location) = &self.location {
            write!(f, "\n  at {location}")?;
        }

        if let Some(suggestion) = &self.suggestion {
            write!(f, "\n  suggestion: {suggestion}")?;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Fatal error
    Error,
    /// Warning that should be addressed
    Warning,
    /// Informational message
    Info,
    /// Helpful suggestion
    Help,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
            Self::Info => write!(f, "info"),
            Self::Help => write!(f, "help"),
        }
    }
}

// Error code constants
pub mod codes {
    // Parse errors (E_PARSE_*)
    pub const E_PARSE_BAD_CHECKBOX: &str = "E_PARSE_BAD_CHECKBOX";
    pub const E_PARSE_INVALID_ANNOTATION: &str = "E_PARSE_INVALID_ANNOTATION";
    pub const E_PARSE_MALFORMED_HEADING: &str = "E_PARSE_MALFORMED_HEADING";
    pub const E_PARSE_INVALID_DATE: &str = "E_PARSE_INVALID_DATE";

    // Lint errors (E_LINT_*)
    pub const E_LINT_DEPTH_EXCEEDED: &str = "E_LINT_DEPTH_EXCEEDED";
    pub const E_LINT_DUPLICATE_ID: &str = "E_LINT_DUPLICATE_ID";
    pub const E_LINT_MISSING_ID: &str = "E_LINT_MISSING_ID";
    pub const E_LINT_INVALID_STATUS: &str = "E_LINT_INVALID_STATUS";
    pub const E_LINT_UNKNOWN_ANNOTATION: &str = "E_LINT_UNKNOWN_ANNOTATION";
    pub const E_LINT_BAD_INDENTATION: &str = "E_LINT_BAD_INDENTATION";

    // Dependency errors (E_DEP_*)
    pub const E_DEP_NOT_FOUND: &str = "E_DEP_NOT_FOUND";
    pub const E_DEP_CYCLE: &str = "E_DEP_CYCLE";
    pub const E_DEP_INVALID_REF: &str = "E_DEP_INVALID_REF";

    // I/O errors (E_IO_*)
    pub const E_IO_FILE_NOT_FOUND: &str = "E_IO_FILE_NOT_FOUND";
    pub const E_IO_READ_ERROR: &str = "E_IO_READ_ERROR";
    pub const E_IO_WRITE_ERROR: &str = "E_IO_WRITE_ERROR";
    pub const E_IO_PERMISSION_DENIED: &str = "E_IO_PERMISSION_DENIED";

    // Database errors (E_DB_*)
    pub const E_DB_CONNECTION: &str = "E_DB_CONNECTION";
    pub const E_DB_QUERY: &str = "E_DB_QUERY";
    pub const E_DB_CONSTRAINT: &str = "E_DB_CONSTRAINT";
    pub const E_DB_MIGRATION: &str = "E_DB_MIGRATION";

    // Configuration errors (E_CFG_*)
    pub const E_CFG_ROOT_NOT_FOUND: &str = "E_CFG_ROOT_NOT_FOUND";
    pub const E_CFG_INVALID_VALUE: &str = "E_CFG_INVALID_VALUE";
    pub const E_CFG_PARSE_ERROR: &str = "E_CFG_PARSE_ERROR";
    pub const E_CFG_MISSING_INDEX: &str = "E_CFG_MISSING_INDEX";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_extraction() {
        let err = LashError::ParseError {
            code: codes::E_PARSE_BAD_CHECKBOX,
            message: "Invalid checkbox syntax".to_string(),
            location: None,
        };
        assert_eq!(err.code(), codes::E_PARSE_BAD_CHECKBOX);
    }

    #[test]
    fn test_diagnostic_json_serialization() {
        let diag = Diagnostic {
            code: codes::E_LINT_DEPTH_EXCEEDED,
            severity: Severity::Error,
            message: "Task nesting exceeds maximum depth".to_string(),
            location: Some(Location {
                file_path: PathBuf::from("tasks.md"),
                line: Some(42),
                column: Some(5),
                span: None,
            }),
            suggestion: Some("Reduce nesting level to 3 or fewer".to_string()),
        };

        let json = diag.to_json().unwrap();
        assert!(json.contains("E_LINT_DEPTH_EXCEEDED"));
        assert!(json.contains("tasks.md"));
    }

    #[test]
    fn test_location_display() {
        let loc = Location {
            file_path: PathBuf::from("/path/to/file.md"),
            line: Some(10),
            column: Some(5),
            span: None,
        };
        let display = format!("{loc}");
        assert!(display.contains("file.md"));
        assert!(display.contains("10"));
        assert!(display.contains('5'));
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(format!("{}", Severity::Error), "error");
        assert_eq!(format!("{}", Severity::Warning), "warning");
        assert_eq!(format!("{}", Severity::Info), "info");
        assert_eq!(format!("{}", Severity::Help), "help");
    }

    #[test]
    fn test_diagnostic_with_suggestion() {
        let diag = Diagnostic {
            code: codes::E_LINT_DEPTH_EXCEEDED,
            severity: Severity::Error,
            message: "Too deep".to_string(),
            location: None,
            suggestion: None,
        };

        let with_sugg = diag.with_suggestion("Flatten hierarchy");
        assert_eq!(with_sugg.suggestion, Some("Flatten hierarchy".to_string()));
    }
}
