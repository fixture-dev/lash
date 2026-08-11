//! Error types for task creation operations
//!
//! This module provides specialized error types for task creation,
//! with user-friendly messages and actionable help text.

use std::fmt;
use std::path::PathBuf;

/// Errors that can occur during task creation
///
/// Each error variant includes methods for generating user-friendly
/// messages, help text, and stable error codes for tooling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskCreationError {
    /// Task title is empty or whitespace-only
    EmptyTitle,

    /// Task title exceeds maximum allowed length
    TitleTooLong {
        /// Actual length of the title
        len: usize,
        /// Maximum allowed length
        max: usize,
    },

    /// Target file does not exist
    FileNotFound(PathBuf),

    /// File exists but is not writable
    FileNotWritable(PathBuf),

    /// File exists but failed to parse
    FileParseFailed {
        /// Path to the file that failed to parse
        path: PathBuf,
        /// Error message from the parser
        error: String,
    },

    /// Parent task was not found in the file
    ParentNotFound {
        /// ID of the parent task that was not found
        id: String,
    },

    /// Task would exceed maximum nesting depth
    DepthLimitExceeded {
        /// Resulting depth if task were created
        depth: u8,
        /// Maximum allowed depth
        max: u8,
    },

    /// Task ID is already in use in the file
    DuplicateId {
        /// The duplicate task ID
        id: String,
    },

    /// Task ID format is invalid
    InvalidIdFormat {
        /// The invalid ID
        id: String,
        /// Explanation of why it's invalid
        reason: String,
    },

    /// Label format is invalid
    InvalidLabel {
        /// The invalid label
        label: String,
        /// Explanation of why it's invalid
        reason: String,
    },

    /// Time estimate format is invalid
    InvalidEstimate {
        /// The invalid estimate
        estimate: String,
    },

    /// Dependency reference was not found
    DependencyNotFound {
        /// The dependency reference that was not found
        reference: String,
    },

    /// Creating the task would create a circular dependency
    WouldCreateCycle {
        /// ID of the task being created
        task_id: String,
        /// ID of the dependency that would create the cycle
        dependency: String,
    },

    /// The agent note cannot be written and read back unchanged
    InvalidAgentNote {
        /// Explanation of which part of the note does not survive a round trip
        reason: String,
    },

    /// The specified insert position is invalid
    InvalidPosition {
        /// Explanation of why the position is invalid
        reason: String,
    },

    /// I/O error occurred during file operations
    IoError {
        /// Path involved in the I/O error
        path: PathBuf,
        /// Error message
        error: String,
    },
}

impl TaskCreationError {
    /// Get a user-friendly error message
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::EmptyTitle => "task title cannot be empty".to_string(),
            Self::TitleTooLong { len, max } => {
                format!("task title is too long ({len} characters, max {max})")
            }
            Self::FileNotFound(path) => {
                format!("target file not found: {}", path.display())
            }
            Self::FileNotWritable(path) => {
                format!("target file is not writable: {}", path.display())
            }
            Self::FileParseFailed { path, error } => {
                format!("failed to parse target file {}: {}", path.display(), error)
            }
            Self::ParentNotFound { id } => {
                format!("parent task not found: '{id}'")
            }
            Self::DepthLimitExceeded { depth, max } => {
                format!("task would exceed maximum depth ({depth} > {max})")
            }
            Self::DuplicateId { id } => {
                format!("task ID '{id}' is already in use in this file")
            }
            Self::InvalidIdFormat { id, reason } => {
                format!("invalid task ID '{id}': {reason}")
            }
            Self::InvalidLabel { label, reason } => {
                format!("invalid label '{label}': {reason}")
            }
            Self::InvalidEstimate { estimate } => {
                format!("invalid time estimate '{estimate}'")
            }
            Self::DependencyNotFound { reference } => {
                format!("dependency not found: {reference}")
            }
            Self::WouldCreateCycle {
                task_id,
                dependency,
            } => {
                format!(
                    "creating task '{task_id}' with dependency '{dependency}' would create a cycle"
                )
            }
            Self::InvalidAgentNote { reason } => {
                format!("agent note cannot be stored: {reason}")
            }
            Self::InvalidPosition { reason } => {
                format!("invalid insert position: {reason}")
            }
            Self::IoError { path, error } => {
                format!("I/O error for {}: {}", path.display(), error)
            }
        }
    }

    /// Get helpful suggestions for fixing the error
    #[must_use]
    pub fn help(&self) -> String {
        match self {
            Self::EmptyTitle => {
                "provide a non-empty title for the task".to_string()
            }
            Self::TitleTooLong { max, .. } => {
                format!("shorten the title to {max} characters or fewer")
            }
            Self::FileNotFound(path) => {
                format!(
                    "create the file first, or use a NewFile target to create it automatically: {}",
                    path.display()
                )
            }
            Self::FileNotWritable(path) => {
                format!(
                    "check file permissions and ensure {} is writable",
                    path.display()
                )
            }
            Self::FileParseFailed { path, .. } => {
                format!("run `lash lint {}` to see parsing errors and fix them", path.display())
            }
            Self::ParentNotFound { id } => {
                format!("ensure parent task '{id}' exists in the target file, or use ParentRef::None for a top-level task")
            }
            Self::DepthLimitExceeded { max, .. } => {
                format!("choose a parent task at a shallower depth (max depth is {max})")
            }
            Self::DuplicateId { id } => {
                format!("choose a different ID, or omit the ID to have one auto-generated ('{id}' is already used)")
            }
            Self::InvalidIdFormat { .. } => {
                "task IDs must contain only alphanumeric characters, dashes, underscores, and colons".to_string()
            }
            Self::InvalidLabel { .. } => {
                "labels must be alphanumeric with hyphens, no spaces or special characters".to_string()
            }
            Self::InvalidEstimate { .. } => {
                "estimates should be in format like '2h', '3d', '1w', etc.".to_string()
            }
            Self::DependencyNotFound { reference } => {
                format!("ensure the referenced task exists: {reference}")
            }
            Self::WouldCreateCycle { dependency, .. } => {
                format!("remove the dependency on '{dependency}' or restructure the task hierarchy")
            }
            Self::InvalidAgentNote { .. } => {
                "an agent note may span several lines, but each line must have non-whitespace content and must not begin with '@'".to_string()
            }
            Self::InvalidPosition { .. } => {
                "--before/--after take a task ID from the target file, either bare ('beta-task') or qualified with the file ('index#beta-task'); --at-index takes a 0-based position among siblings".to_string()
            }
            Self::IoError { .. } => {
                "check file permissions and disk space".to_string()
            }
        }
    }

    /// Get a stable error code for this error
    #[must_use]
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::EmptyTitle => "E_CREATE_EMPTY_TITLE",
            Self::TitleTooLong { .. } => "E_CREATE_TITLE_TOO_LONG",
            Self::FileNotFound(_) => "E_CREATE_FILE_NOT_FOUND",
            Self::FileNotWritable(_) => "E_CREATE_FILE_NOT_WRITABLE",
            Self::FileParseFailed { .. } => "E_CREATE_FILE_PARSE_FAILED",
            Self::ParentNotFound { .. } => "E_CREATE_PARENT_NOT_FOUND",
            Self::DepthLimitExceeded { .. } => "E_CREATE_DEPTH_LIMIT_EXCEEDED",
            Self::DuplicateId { .. } => "E_CREATE_DUPLICATE_ID",
            Self::InvalidIdFormat { .. } => "E_CREATE_INVALID_ID_FORMAT",
            Self::InvalidLabel { .. } => "E_CREATE_INVALID_LABEL",
            Self::InvalidEstimate { .. } => "E_CREATE_INVALID_ESTIMATE",
            Self::DependencyNotFound { .. } => "E_CREATE_DEPENDENCY_NOT_FOUND",
            Self::WouldCreateCycle { .. } => "E_CREATE_WOULD_CREATE_CYCLE",
            Self::InvalidAgentNote { .. } => "E_CREATE_INVALID_AGENT_NOTE",
            Self::InvalidPosition { .. } => "E_CREATE_INVALID_POSITION",
            Self::IoError { .. } => "E_CREATE_IO_ERROR",
        }
    }
}

impl fmt::Display for TaskCreationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.error_code(), self.message())
    }
}

impl std::error::Error for TaskCreationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_title_error() {
        let err = TaskCreationError::EmptyTitle;
        assert_eq!(err.error_code(), "E_CREATE_EMPTY_TITLE");
        assert!(err.message().contains("empty"));
        assert!(err.help().contains("non-empty"));
    }

    #[test]
    fn test_title_too_long_error() {
        let err = TaskCreationError::TitleTooLong { len: 300, max: 256 };
        assert_eq!(err.error_code(), "E_CREATE_TITLE_TOO_LONG");
        assert!(err.message().contains("300"));
        assert!(err.message().contains("256"));
        assert!(err.help().contains("shorten"));
    }

    #[test]
    fn test_file_not_found_error() {
        let err = TaskCreationError::FileNotFound(PathBuf::from("missing.md"));
        assert_eq!(err.error_code(), "E_CREATE_FILE_NOT_FOUND");
        assert!(err.message().contains("missing.md"));
        assert!(err.help().contains("create"));
    }

    #[test]
    fn test_file_not_writable_error() {
        let err = TaskCreationError::FileNotWritable(PathBuf::from("readonly.md"));
        assert_eq!(err.error_code(), "E_CREATE_FILE_NOT_WRITABLE");
        assert!(err.message().contains("readonly.md"));
        assert!(err.help().contains("permissions"));
    }

    #[test]
    fn test_file_parse_failed_error() {
        let err = TaskCreationError::FileParseFailed {
            path: PathBuf::from("bad.md"),
            error: "invalid syntax".to_string(),
        };
        assert_eq!(err.error_code(), "E_CREATE_FILE_PARSE_FAILED");
        assert!(err.message().contains("bad.md"));
        assert!(err.message().contains("invalid syntax"));
        assert!(err.help().contains("lash lint"));
    }

    #[test]
    fn test_parent_not_found_error() {
        let err = TaskCreationError::ParentNotFound {
            id: "parent-task".to_string(),
        };
        assert_eq!(err.error_code(), "E_CREATE_PARENT_NOT_FOUND");
        assert!(err.message().contains("parent-task"));
        assert!(err.help().contains("ensure"));
    }

    #[test]
    fn test_depth_limit_exceeded_error() {
        let err = TaskCreationError::DepthLimitExceeded { depth: 5, max: 4 };
        assert_eq!(err.error_code(), "E_CREATE_DEPTH_LIMIT_EXCEEDED");
        assert!(err.message().contains('5'));
        assert!(err.message().contains('4'));
        assert!(err.help().contains("shallower"));
    }

    #[test]
    fn test_duplicate_id_error() {
        let err = TaskCreationError::DuplicateId {
            id: "task-1".to_string(),
        };
        assert_eq!(err.error_code(), "E_CREATE_DUPLICATE_ID");
        assert!(err.message().contains("task-1"));
        assert!(err.help().contains("different"));
    }

    #[test]
    fn test_invalid_id_format_error() {
        let err = TaskCreationError::InvalidIdFormat {
            id: "invalid id".to_string(),
            reason: "contains spaces".to_string(),
        };
        assert_eq!(err.error_code(), "E_CREATE_INVALID_ID_FORMAT");
        assert!(err.message().contains("invalid id"));
        assert!(err.message().contains("contains spaces"));
        assert!(err.help().contains("alphanumeric"));
    }

    #[test]
    fn test_invalid_label_error() {
        let err = TaskCreationError::InvalidLabel {
            label: "bad label!".to_string(),
            reason: "contains special characters".to_string(),
        };
        assert_eq!(err.error_code(), "E_CREATE_INVALID_LABEL");
        assert!(err.message().contains("bad label!"));
        assert!(err.message().contains("special characters"));
        assert!(err.help().contains("alphanumeric"));
    }

    #[test]
    fn test_invalid_estimate_error() {
        let err = TaskCreationError::InvalidEstimate {
            estimate: "invalid".to_string(),
        };
        assert_eq!(err.error_code(), "E_CREATE_INVALID_ESTIMATE");
        assert!(err.message().contains("invalid"));
        assert!(err.help().contains("2h"));
    }

    #[test]
    fn test_dependency_not_found_error() {
        let err = TaskCreationError::DependencyNotFound {
            reference: "path/to/task.md#task:id".to_string(),
        };
        assert_eq!(err.error_code(), "E_CREATE_DEPENDENCY_NOT_FOUND");
        assert!(err.message().contains("path/to/task.md#task:id"));
        assert!(err.help().contains("ensure"));
    }

    #[test]
    fn test_would_create_cycle_error() {
        let err = TaskCreationError::WouldCreateCycle {
            task_id: "task-1".to_string(),
            dependency: "task-2".to_string(),
        };
        assert_eq!(err.error_code(), "E_CREATE_WOULD_CREATE_CYCLE");
        assert!(err.message().contains("task-1"));
        assert!(err.message().contains("task-2"));
        assert!(err.help().contains("remove"));
    }

    #[test]
    fn test_invalid_position_error() {
        let err = TaskCreationError::InvalidPosition {
            reason: "referenced task not found".to_string(),
        };
        assert_eq!(err.error_code(), "E_CREATE_INVALID_POSITION");
        assert!(err.message().contains("referenced task not found"));
        // The help names both accepted spellings of a position ID, because
        // the qualified one is what `lash show` prints (GitHub issue #53).
        assert!(err.help().contains("--before/--after"));
        assert!(err.help().contains("index#beta-task"));
    }

    #[test]
    fn test_io_error() {
        let err = TaskCreationError::IoError {
            path: PathBuf::from("test.md"),
            error: "permission denied".to_string(),
        };
        assert_eq!(err.error_code(), "E_CREATE_IO_ERROR");
        assert!(err.message().contains("test.md"));
        assert!(err.message().contains("permission denied"));
        assert!(err.help().contains("permissions"));
    }

    #[test]
    fn test_display_formatting() {
        let err = TaskCreationError::EmptyTitle;
        let display = format!("{err}");
        assert!(display.contains("E_CREATE_EMPTY_TITLE"));
        assert!(display.contains("empty"));
    }

    #[test]
    fn test_error_code_stability() {
        // These codes must never change - they're part of the stable API
        assert_eq!(
            TaskCreationError::EmptyTitle.error_code(),
            "E_CREATE_EMPTY_TITLE"
        );
        assert_eq!(
            TaskCreationError::TitleTooLong { len: 100, max: 50 }.error_code(),
            "E_CREATE_TITLE_TOO_LONG"
        );
        assert_eq!(
            TaskCreationError::FileNotFound(PathBuf::from("test.md")).error_code(),
            "E_CREATE_FILE_NOT_FOUND"
        );
        assert_eq!(
            TaskCreationError::FileNotWritable(PathBuf::from("test.md")).error_code(),
            "E_CREATE_FILE_NOT_WRITABLE"
        );
        assert_eq!(
            TaskCreationError::FileParseFailed {
                path: PathBuf::from("test.md"),
                error: "error".to_string()
            }
            .error_code(),
            "E_CREATE_FILE_PARSE_FAILED"
        );
        assert_eq!(
            TaskCreationError::ParentNotFound {
                id: "id".to_string()
            }
            .error_code(),
            "E_CREATE_PARENT_NOT_FOUND"
        );
        assert_eq!(
            TaskCreationError::DepthLimitExceeded { depth: 5, max: 4 }.error_code(),
            "E_CREATE_DEPTH_LIMIT_EXCEEDED"
        );
        assert_eq!(
            TaskCreationError::DuplicateId {
                id: "id".to_string()
            }
            .error_code(),
            "E_CREATE_DUPLICATE_ID"
        );
        assert_eq!(
            TaskCreationError::InvalidIdFormat {
                id: "id".to_string(),
                reason: "reason".to_string()
            }
            .error_code(),
            "E_CREATE_INVALID_ID_FORMAT"
        );
        assert_eq!(
            TaskCreationError::InvalidLabel {
                label: "label".to_string(),
                reason: "reason".to_string()
            }
            .error_code(),
            "E_CREATE_INVALID_LABEL"
        );
        assert_eq!(
            TaskCreationError::InvalidEstimate {
                estimate: "est".to_string()
            }
            .error_code(),
            "E_CREATE_INVALID_ESTIMATE"
        );
        assert_eq!(
            TaskCreationError::DependencyNotFound {
                reference: "ref".to_string()
            }
            .error_code(),
            "E_CREATE_DEPENDENCY_NOT_FOUND"
        );
        assert_eq!(
            TaskCreationError::WouldCreateCycle {
                task_id: "t1".to_string(),
                dependency: "t2".to_string()
            }
            .error_code(),
            "E_CREATE_WOULD_CREATE_CYCLE"
        );
        assert_eq!(
            TaskCreationError::InvalidPosition {
                reason: "reason".to_string()
            }
            .error_code(),
            "E_CREATE_INVALID_POSITION"
        );
        assert_eq!(
            TaskCreationError::IoError {
                path: PathBuf::from("test.md"),
                error: "error".to_string()
            }
            .error_code(),
            "E_CREATE_IO_ERROR"
        );
    }

    #[test]
    fn test_equality() {
        let err1 = TaskCreationError::EmptyTitle;
        let err2 = TaskCreationError::EmptyTitle;
        assert_eq!(err1, err2);

        let err3 = TaskCreationError::TitleTooLong { len: 100, max: 50 };
        let err4 = TaskCreationError::TitleTooLong { len: 100, max: 50 };
        assert_eq!(err3, err4);

        let err5 = TaskCreationError::TitleTooLong { len: 100, max: 50 };
        let err6 = TaskCreationError::TitleTooLong { len: 200, max: 50 };
        assert_ne!(err5, err6);
    }

    #[test]
    fn test_clone() {
        let err = TaskCreationError::DuplicateId {
            id: "task-1".to_string(),
        };
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_debug() {
        let err = TaskCreationError::EmptyTitle;
        let debug = format!("{err:?}");
        assert!(debug.contains("EmptyTitle"));
    }
}
