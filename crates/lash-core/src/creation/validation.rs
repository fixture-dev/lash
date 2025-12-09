//! Validation layer for task creation requests
//!
//! This module provides comprehensive validation for task creation requests,
//! checking title, ID, labels, estimates, and parent references before tasks
//! are created in the markdown files.

use lash_types::config::LashConfig;
use lash_types::creation::{ParentRef, TaskCreationRequest};
use lash_types::creation_errors::TaskCreationError;
use lash_types::file::TaskFile;
use lash_types::task::Task;
use regex::Regex;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::OnceLock;

// Constants
const MAX_TITLE_LENGTH: usize = 200;

// Static regex patterns
static ID_REGEX: OnceLock<Regex> = OnceLock::new();
static LABEL_REGEX: OnceLock<Regex> = OnceLock::new();
static ESTIMATE_REGEX: OnceLock<Regex> = OnceLock::new();

/// Context built during validation, passed to subsequent stages
///
/// Contains all the information gathered during validation that will be
/// needed by placement and emitter stages.
#[derive(Debug, Clone)]
pub struct ValidationContext {
    /// Configuration used for validation
    pub config: LashConfig,
    /// Parsed file content
    pub resolved_file: TaskFile,
    /// Resolved parent task (if specified)
    pub parent_task: Option<Task>,
    /// Computed depth for the new task (0 for top-level)
    pub computed_depth: u8,
    /// All task IDs currently in the file
    pub existing_ids: HashSet<String>,
}

/// Validates task creation requests
///
/// The validator performs comprehensive checks on all aspects of a task
/// creation request, collecting all errors rather than stopping at the first one.
pub struct TaskValidator {
    config: LashConfig,
}

impl TaskValidator {
    /// Create a new task validator with the given configuration
    #[must_use]
    pub fn new(config: LashConfig) -> Self {
        Self { config }
    }

    /// Main entry point - validates a request against a file
    ///
    /// Performs comprehensive validation of the task creation request,
    /// collecting all validation errors before returning.
    ///
    /// # Arguments
    ///
    /// * `request` - The task creation request to validate
    /// * `file_content` - Optional parsed file content (if file exists)
    ///
    /// # Returns
    ///
    /// * `Ok(ValidationContext)` - Context for subsequent stages if validation passes
    /// * `Err(Vec<TaskCreationError>)` - All validation errors encountered
    ///
    /// # Errors
    ///
    /// Returns a vector of all validation errors found. Common errors include:
    /// - Empty or too-long titles
    /// - Invalid ID format or duplicate IDs
    /// - Invalid label or estimate formats
    /// - Parent task not found
    /// - Depth limit exceeded
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_core::creation::validation::TaskValidator;
    /// use lash_types::config::ConfigBuilder;
    /// use lash_types::creation::{TaskCreationRequest, FileTarget, ParentRef, InsertPosition};
    /// use std::path::PathBuf;
    ///
    /// let config = ConfigBuilder::new().build().unwrap();
    /// let validator = TaskValidator::new(config);
    ///
    /// let request = TaskCreationRequest {
    ///     title: "Test task".to_string(),
    ///     file_target: FileTarget::Path(PathBuf::from("tasks.md")),
    ///     parent: ParentRef::None,
    ///     position: InsertPosition::Append,
    ///     status: None,
    ///     id: Some("test-id".to_string()),
    ///     labels: vec![],
    ///     owner: None,
    ///     estimate: None,
    ///     depends_on: vec![],
    ///     agent_note: None,
    /// };
    ///
    /// // Validation without file content (for new files)
    /// let result = validator.validate(&request, None);
    /// ```
    pub fn validate(
        &self,
        request: &TaskCreationRequest,
        file_content: Option<&TaskFile>,
    ) -> Result<ValidationContext, Vec<TaskCreationError>> {
        let mut errors = Vec::new();

        // Collect existing IDs from file
        let existing_ids = if let Some(file) = file_content {
            file.tasks
                .tasks()
                .iter()
                .map(|t| t.id.clone())
                .collect::<HashSet<_>>()
        } else {
            HashSet::new()
        };

        // Validate title
        Self::validate_title(&request.title, &mut errors);

        // Validate ID if provided
        if let Some(ref id) = request.id {
            Self::validate_id(Some(id), &existing_ids, &mut errors);
        }

        // Validate labels
        for label in &request.labels {
            Self::validate_label(label, &mut errors);
        }

        // Validate estimate if provided
        if let Some(ref estimate) = request.estimate {
            Self::validate_estimate(estimate, &mut errors);
        }

        // Note: Owner validation is currently a no-op

        // Resolve parent and compute depth
        let (parent_task, computed_depth) = if let Some(file) = file_content {
            let parent = self.resolve_parent(&request.parent, file, &mut errors);
            let depth = if let Some(ref p) = parent {
                p.depth + 1
            } else {
                0
            };

            // Check depth limit
            if depth > self.config.max_depth {
                errors.push(TaskCreationError::DepthLimitExceeded {
                    depth,
                    max: self.config.max_depth,
                });
            }

            (parent, depth)
        } else {
            // New file - can only be top-level
            if !matches!(request.parent, ParentRef::None) {
                errors.push(TaskCreationError::ParentNotFound {
                    id: format!("{:?}", request.parent),
                });
            }
            (None, 0)
        };

        // Note: File target resolution is minimal for now

        // If we have errors, return them
        if !errors.is_empty() {
            return Err(errors);
        }

        // Build successful validation context
        Ok(ValidationContext {
            config: self.config.clone(),
            resolved_file: file_content.cloned().unwrap_or_else(|| {
                // Create a minimal TaskFile for new files
                use lash_types::file::{compute_hash, FileMetadata};
                use lash_types::task::TaskTree;
                use std::time::SystemTime;

                TaskFile {
                    path: PathBuf::new(),
                    title: String::new(),
                    id: String::new(),
                    metadata: FileMetadata::default(),
                    description: None,
                    description_agent_notes: Vec::new(),
                    tasks: TaskTree::new(),
                    hash: compute_hash(""),
                    mtime: SystemTime::now(),
                }
            }),
            parent_task,
            computed_depth,
            existing_ids,
        })
    }

    /// Validate task title
    ///
    /// Ensures the title is non-empty and within maximum length.
    fn validate_title(title: &str, errors: &mut Vec<TaskCreationError>) {
        let trimmed = title.trim();

        if trimmed.is_empty() {
            errors.push(TaskCreationError::EmptyTitle);
            return;
        }

        if trimmed.len() > MAX_TITLE_LENGTH {
            errors.push(TaskCreationError::TitleTooLong {
                len: trimmed.len(),
                max: MAX_TITLE_LENGTH,
            });
        }
    }

    /// Validate task ID format and uniqueness
    ///
    /// IDs must:
    /// - Start with a lowercase letter
    /// - Contain only lowercase letters, numbers, and hyphens
    /// - Not already exist in the file
    fn validate_id(
        id: Option<&str>,
        existing_ids: &HashSet<String>,
        errors: &mut Vec<TaskCreationError>,
    ) {
        let Some(id_str) = id else {
            return;
        };

        // Check format using regex: ^[a-z][a-z0-9-]*$
        let regex = ID_REGEX.get_or_init(|| Regex::new(r"^[a-z][a-z0-9-]*$").unwrap());

        if !regex.is_match(id_str) {
            errors.push(TaskCreationError::InvalidIdFormat {
                id: id_str.to_string(),
                reason: "must start with lowercase letter and contain only lowercase letters, numbers, and hyphens".to_string(),
            });
        }

        // Check for duplicate
        if existing_ids.contains(id_str) {
            errors.push(TaskCreationError::DuplicateId {
                id: id_str.to_string(),
            });
        }
    }

    /// Validate label format
    ///
    /// Labels must contain only lowercase letters, numbers, and hyphens.
    fn validate_label(label: &str, errors: &mut Vec<TaskCreationError>) {
        // Check format using regex: ^[a-z][a-z0-9-]*$
        let regex = LABEL_REGEX.get_or_init(|| Regex::new(r"^[a-z][a-z0-9-]*$").unwrap());

        if !regex.is_match(label) {
            errors.push(TaskCreationError::InvalidLabel {
                label: label.to_string(),
                reason: "must start with lowercase letter and contain only lowercase letters, numbers, and hyphens".to_string(),
            });
        }
    }

    /// Validate time estimate format
    ///
    /// Estimates must match the pattern: `\d+[mhdw]` (e.g., 30m, 2h, 1d, 2w)
    fn validate_estimate(estimate: &str, errors: &mut Vec<TaskCreationError>) {
        // Check format using regex: ^\d+[mhdw]$
        let regex = ESTIMATE_REGEX.get_or_init(|| Regex::new(r"^\d+[mhdw]$").unwrap());

        if !regex.is_match(estimate) {
            errors.push(TaskCreationError::InvalidEstimate {
                estimate: estimate.to_string(),
            });
        }
    }

    // Note: Owner validation is currently a no-op.
    // Owner can be any string for now.

    /// Resolve parent task reference
    ///
    /// Returns the parent task if found, or None for top-level tasks.
    /// Adds errors if parent cannot be resolved.
    fn resolve_parent(
        &self,
        parent: &ParentRef,
        file: &TaskFile,
        errors: &mut Vec<TaskCreationError>,
    ) -> Option<Task> {
        match parent {
            ParentRef::None => None,
            ParentRef::Id(id) => {
                // Find task in file
                if let Some(task) = file.tasks.get_task(id) {
                    Some(task.clone())
                } else {
                    errors.push(TaskCreationError::ParentNotFound { id: id.clone() });
                    None
                }
            }
            ParentRef::FullRef(_full_ref) => {
                // For now, we only support within-file parents
                // Full references across files will be implemented later
                errors.push(TaskCreationError::ParentNotFound {
                    id: "cross-file parent references not yet implemented".to_string(),
                });
                None
            }
            ParentRef::AppendAtDepth(depth) => {
                // Find the last task at depth - 1
                // This is a more complex operation that requires scanning the file
                // For now, we'll just validate the depth
                if *depth > self.config.max_depth {
                    errors.push(TaskCreationError::DepthLimitExceeded {
                        depth: *depth,
                        max: self.config.max_depth,
                    });
                }
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::config::ConfigBuilder;
    use lash_types::creation::TaskCreationRequestBuilder;
    use lash_types::file::{compute_hash, FileMetadata};
    use lash_types::task::{TaskBuilder, TaskTree};
    use std::time::SystemTime;

    fn create_test_file(tasks: TaskTree) -> TaskFile {
        TaskFile {
            path: PathBuf::from("test.md"),
            title: "Test File".to_string(),
            id: "test-file".to_string(),
            metadata: FileMetadata::default(),
            description: None,
            description_agent_notes: Vec::new(),
            tasks,
            hash: compute_hash("test content"),
            mtime: SystemTime::now(),
        }
    }

    #[test]
    fn test_valid_request_passes() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let request = TaskCreationRequestBuilder::new("Valid task title")
            .id("valid-task-id")
            .label("backend")
            .estimate("2h")
            .owner("alice")
            .build();

        let result = validator.validate(&request, None);
        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.computed_depth, 0);
        assert!(ctx.parent_task.is_none());
    }

    #[test]
    fn test_empty_title_error() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let request = TaskCreationRequestBuilder::new("   ").build();

        let result = validator.validate(&request, None);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| matches!(e, TaskCreationError::EmptyTitle)));
    }

    #[test]
    fn test_title_too_long_error() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let long_title = "a".repeat(201);
        let request = TaskCreationRequestBuilder::new(long_title).build();

        let result = validator.validate(&request, None);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| matches!(e, TaskCreationError::TitleTooLong { len: 201, max: 200 })));
    }

    #[test]
    fn test_invalid_id_format() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        // Test various invalid formats
        let invalid_ids = vec![
            "Invalid-ID", // Capital letters
            "invalid id", // Space
            "invalid_id", // Underscore
            "123invalid", // Starts with number
            "invalid@id", // Special character
            "invalid.id", // Dot
            "-invalid",   // Starts with hyphen
        ];

        for id in invalid_ids {
            let request = TaskCreationRequestBuilder::new("Test").id(id).build();

            let result = validator.validate(&request, None);
            assert!(
                result.is_err(),
                "ID '{id}' should be invalid but passed validation"
            );
            let errors = result.unwrap_err();
            assert!(
                errors
                    .iter()
                    .any(|e| matches!(e, TaskCreationError::InvalidIdFormat { .. })),
                "ID '{id}' should trigger InvalidIdFormat error"
            );
        }
    }

    #[test]
    fn test_valid_id_formats() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let valid_ids = vec!["valid", "valid-id", "valid-id-123", "v", "v123", "task-1"];

        for id in valid_ids {
            let request = TaskCreationRequestBuilder::new("Test").id(id).build();

            let result = validator.validate(&request, None);
            assert!(
                result.is_ok(),
                "ID '{id}' should be valid but failed: {result:?}"
            );
        }
    }

    #[test]
    fn test_duplicate_id_error() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let mut tasks = TaskTree::new();
        tasks
            .add_task(
                TaskBuilder::new("Existing")
                    .id("existing-id")
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let file = create_test_file(tasks);

        let request = TaskCreationRequestBuilder::new("New task")
            .id("existing-id")
            .build();

        let result = validator.validate(&request, Some(&file));
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            TaskCreationError::DuplicateId { id } if id == "existing-id"
        )));
    }

    #[test]
    fn test_invalid_label_format() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let request = TaskCreationRequestBuilder::new("Test")
            .label("Invalid-Label")
            .label("invalid label")
            .build();

        let result = validator.validate(&request, None);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(
            errors
                .iter()
                .filter(|e| matches!(e, TaskCreationError::InvalidLabel { .. }))
                .count(),
            2
        );
    }

    #[test]
    fn test_valid_label_formats() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let request = TaskCreationRequestBuilder::new("Test")
            .label("backend")
            .label("api-v2")
            .label("test123")
            .build();

        let result = validator.validate(&request, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_estimate_format() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let invalid_estimates = vec!["2hours", "2 h", "h2", "2H", "2", "invalid", "2hrs"];

        for estimate in invalid_estimates {
            let request = TaskCreationRequestBuilder::new("Test")
                .estimate(estimate)
                .build();

            let result = validator.validate(&request, None);
            assert!(result.is_err(), "Estimate '{estimate}' should be invalid");
            let errors = result.unwrap_err();
            assert!(errors
                .iter()
                .any(|e| matches!(e, TaskCreationError::InvalidEstimate { .. })));
        }
    }

    #[test]
    fn test_valid_estimate_formats() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let valid_estimates = vec!["30m", "2h", "1d", "2w", "0m", "999h"];

        for estimate in valid_estimates {
            let request = TaskCreationRequestBuilder::new("Test")
                .estimate(estimate)
                .build();

            let result = validator.validate(&request, None);
            assert!(
                result.is_ok(),
                "Estimate '{estimate}' should be valid: {result:?}"
            );
        }
    }

    #[test]
    fn test_parent_not_found() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let tasks = TaskTree::new();
        let file = create_test_file(tasks);

        let request = TaskCreationRequestBuilder::new("Child task")
            .parent_id("nonexistent-parent")
            .build();

        let result = validator.validate(&request, Some(&file));
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            TaskCreationError::ParentNotFound { id } if id == "nonexistent-parent"
        )));
    }

    #[test]
    fn test_parent_found() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let mut tasks = TaskTree::new();
        tasks
            .add_task(
                TaskBuilder::new("Parent")
                    .id("parent-task")
                    .depth(0)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let file = create_test_file(tasks);

        let request = TaskCreationRequestBuilder::new("Child task")
            .parent_id("parent-task")
            .build();

        let result = validator.validate(&request, Some(&file));
        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert!(ctx.parent_task.is_some());
        assert_eq!(ctx.parent_task.unwrap().id, "parent-task");
        assert_eq!(ctx.computed_depth, 1);
    }

    #[test]
    fn test_depth_limit_exceeded() {
        let config = ConfigBuilder::new().max_depth(3).build().unwrap();
        let validator = TaskValidator::new(config);

        let mut tasks = TaskTree::new();
        // Create a parent at depth 3 (max allowed)
        tasks
            .add_task(
                TaskBuilder::new("Deep parent")
                    .id("deep-parent")
                    .depth(3)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let file = create_test_file(tasks);

        let request = TaskCreationRequestBuilder::new("Too deep child")
            .parent_id("deep-parent")
            .build();

        let result = validator.validate(&request, Some(&file));
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            TaskCreationError::DepthLimitExceeded { depth: 4, max: 3 }
        )));
    }

    #[test]
    fn test_multiple_errors_collected() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let mut tasks = TaskTree::new();
        tasks
            .add_task(
                TaskBuilder::new("Existing")
                    .id("existing-id")
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let file = create_test_file(tasks);

        let request = TaskCreationRequestBuilder::new("") // Empty title
            .id("existing-id") // Duplicate ID
            .label("Invalid Label") // Invalid label
            .estimate("invalid") // Invalid estimate
            .parent_id("nonexistent-parent") // Parent not found
            .build();

        let result = validator.validate(&request, Some(&file));
        assert!(result.is_err());
        let errors = result.unwrap_err();

        // Should have collected multiple errors
        assert!(errors.len() >= 4);
        assert!(errors
            .iter()
            .any(|e| matches!(e, TaskCreationError::EmptyTitle)));
        assert!(errors
            .iter()
            .any(|e| matches!(e, TaskCreationError::DuplicateId { .. })));
        assert!(errors
            .iter()
            .any(|e| matches!(e, TaskCreationError::InvalidLabel { .. })));
        assert!(errors
            .iter()
            .any(|e| matches!(e, TaskCreationError::InvalidEstimate { .. })));
        assert!(errors
            .iter()
            .any(|e| matches!(e, TaskCreationError::ParentNotFound { .. })));
    }
}
