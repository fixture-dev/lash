//! Task creation service
//!
//! This module provides the `TaskCreationService` that orchestrates the
//! complete task creation workflow, tying together validation, placement,
//! and markdown emission.

use crate::parser::parse_file;
use lash_types::config::LashConfig;
use lash_types::creation::{FileTarget, TaskCreationRequest, TaskCreationResult};
use lash_types::creation_errors::TaskCreationError;
use lash_types::file::TaskFile;
use std::path::{Path, PathBuf};

use super::emitter::MarkdownEmitter;
use super::placement::{PlacementInfo, PlacementResolver};
use super::validation::{TaskValidator, ValidationContext};

/// Everything decided about a task before anything is written
///
/// Produced by [`TaskCreationService::plan_task`]. `create_task` turns a plan
/// into a file write; `lash add --dry-run` reports it and stops. Both go
/// through the same code so a dry run cannot pass on a request the real add
/// would reject.
#[derive(Debug, Clone)]
pub struct TaskCreationPlan {
    /// Validation context: resolved file, parent, depth, existing IDs
    pub context: ValidationContext,

    /// Where the task would be inserted
    pub placement: PlacementInfo,
}

/// Service that orchestrates task creation
///
/// This is the main entry point for creating tasks programmatically via
/// CLI and TUI interfaces. It coordinates validation, placement resolution,
/// and markdown emission.
///
/// # Examples
///
/// ```
/// use lash_core::creation::service::TaskCreationService;
/// use lash_types::config::ConfigBuilder;
/// use lash_types::creation::{TaskCreationRequestBuilder, FileTarget};
/// use std::path::PathBuf;
///
/// let config = ConfigBuilder::new().build().unwrap();
/// let service = TaskCreationService::new(config);
///
/// let request = TaskCreationRequestBuilder::new("Implement feature X")
///     .file_path(PathBuf::from("tasks.md"))
///     .label("backend")
///     .build();
///
/// // Note: This would perform actual file I/O in practice
/// // let result = service.create_task(&request)?;
/// ```
pub struct TaskCreationService {
    config: LashConfig,
}

impl TaskCreationService {
    /// Create a new task creation service with the given configuration
    #[must_use]
    pub fn new(config: LashConfig) -> Self {
        Self { config }
    }

    /// Main entry point - create a task from a request
    ///
    /// This method orchestrates the complete task creation workflow:
    /// 1. Load target file (if it exists)
    /// 2. Validate the request
    /// 3. Resolve placement
    /// 4. Emit markdown
    ///
    /// # Arguments
    ///
    /// * `request` - The task creation request containing all task details
    ///
    /// # Returns
    ///
    /// * `Ok(TaskCreationResult)` - Information about the created task
    /// * `Err(Vec<TaskCreationError>)` - All validation/creation errors encountered
    ///
    /// # Errors
    ///
    /// Returns errors for:
    /// - Invalid request fields (title, ID, labels, etc.)
    /// - File not found or not readable
    /// - Parent task not found
    /// - Depth limits exceeded
    /// - Invalid placement positions
    /// - File I/O failures
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lash_core::creation::service::TaskCreationService;
    /// use lash_types::config::ConfigBuilder;
    /// use lash_types::creation::TaskCreationRequestBuilder;
    /// use std::path::PathBuf;
    ///
    /// let config = ConfigBuilder::new().build().unwrap();
    /// let service = TaskCreationService::new(config);
    ///
    /// let request = TaskCreationRequestBuilder::new("New task")
    ///     .file_path(PathBuf::from("tasks.md"))
    ///     .build();
    ///
    /// match service.create_task(&request) {
    ///     Ok(result) => {
    ///         println!("Created task '{}' at {}:{}",
    ///             result.task_id, result.file_path.display(), result.line_number);
    ///     }
    ///     Err(errors) => {
    ///         for error in errors {
    ///             eprintln!("Error: {}", error.message());
    ///         }
    ///     }
    /// }
    /// ```
    pub fn create_task(
        &self,
        request: &TaskCreationRequest,
    ) -> Result<TaskCreationResult, Vec<TaskCreationError>> {
        // Steps 1-3: load the target file, validate, resolve placement
        let plan = self.plan_task(request)?;
        let TaskCreationPlan { context, placement } = plan;

        // Step 4: Emit to markdown
        let mut result =
            MarkdownEmitter::emit(request, &context, &placement).map_err(|e| vec![e])?;

        // Step 5: Report the ID the parser gives the task, not the one the
        // emitter guessed. The two agree on the slug now that both go through
        // `synthesize_task_id`, but only the parser can apply the numeric
        // suffix it uses to break a collision, and it is the parser's answer
        // that ends up in the index and in `lash show`.
        if let Some(parsed_id) =
            Self::parsed_id_at(&result.file_path, result.line_number, &self.config)
        {
            result.task_id = parsed_id;
        }

        Ok(result)
    }

    /// Work out what would be created, without writing anything
    ///
    /// Runs every check `create_task` runs — loading and parsing the target
    /// file, validating the request against it, resolving the insert position
    /// — and stops short of the write. This is what backs
    /// `lash add --dry-run`, which previously printed the request back
    /// unexamined and so reported success for positions that did not exist
    /// (GitHub issue #53).
    ///
    /// # Arguments
    ///
    /// * `request` - The task creation request to plan
    ///
    /// # Returns
    ///
    /// * `Ok(TaskCreationPlan)` - The validated context and resolved placement
    /// * `Err(Vec<TaskCreationError>)` - Every error the real add would raise
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::create_task`], minus the ones that
    /// can only arise from the write itself (I/O failures).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lash_core::creation::service::TaskCreationService;
    /// use lash_types::config::ConfigBuilder;
    /// use lash_types::creation::TaskCreationRequestBuilder;
    /// use std::path::PathBuf;
    ///
    /// let config = ConfigBuilder::new().build().unwrap();
    /// let service = TaskCreationService::new(config);
    ///
    /// let request = TaskCreationRequestBuilder::new("New task")
    ///     .file_path(PathBuf::from("tasks.md"))
    ///     .before("existing-task")
    ///     .build();
    ///
    /// match service.plan_task(&request) {
    ///     Ok(plan) => println!("would insert at index {}", plan.placement.order_index),
    ///     Err(errors) => eprintln!("{} problem(s)", errors.len()),
    /// }
    /// ```
    pub fn plan_task(
        &self,
        request: &TaskCreationRequest,
    ) -> Result<TaskCreationPlan, Vec<TaskCreationError>> {
        // Step 1: Load target file (if existing file)
        let file_content = self.load_target_file(&request.file_target)?;

        // Step 2: Validate request
        let validator = TaskValidator::new(self.config.clone());
        let context = validator.validate(request, file_content.as_ref())?;

        // Step 3: Resolve placement
        let placement = PlacementResolver::resolve(&context, request).map_err(|e| vec![e])?;

        Ok(TaskCreationPlan { context, placement })
    }

    /// The ID the parser assigns to the task written at `line_number`
    ///
    /// Returns `None` if the file cannot be re-read or holds no task on that
    /// line, in which case the caller keeps the emitter's ID rather than
    /// failing a write that already succeeded.
    fn parsed_id_at(path: &Path, line_number: usize, config: &LashConfig) -> Option<String> {
        parse_file(path, config)
            .ok()?
            .tasks
            .tasks()
            .iter()
            .find(|task| task.line_number == line_number)
            .map(|task| task.id.clone())
    }

    /// Load and parse the target file if it exists
    ///
    /// Handles different file target types and returns the parsed file content
    /// if the file exists, or None if it should be created.
    ///
    /// # Arguments
    ///
    /// * `target` - The file target specification
    ///
    /// # Returns
    ///
    /// * `Ok(Some(TaskFile))` - File exists and was parsed successfully
    /// * `Ok(None)` - File doesn't exist or should be created
    /// * `Err(Vec<TaskCreationError>)` - File parse errors or not found errors
    fn load_target_file(
        &self,
        target: &FileTarget,
    ) -> Result<Option<TaskFile>, Vec<TaskCreationError>> {
        match target {
            FileTarget::Current => {
                // This is typically handled at CLI/TUI level
                // Return None to indicate "use current context"
                Ok(None)
            }
            FileTarget::Path(path) => {
                if path.exists() {
                    let file = parse_file(path, &self.config).map_err(|e| {
                        vec![TaskCreationError::FileParseFailed {
                            path: path.clone(),
                            error: e.to_string(),
                        }]
                    })?;
                    Ok(Some(file))
                } else {
                    Ok(None) // Will create new file
                }
            }
            FileTarget::NewFile { .. } => {
                Ok(None) // New file, nothing to load
            }
            FileTarget::ContainingTask(reference) => {
                // Parse reference to extract file path
                let path = Self::extract_file_path(reference)?;
                if path.exists() {
                    let file = parse_file(&path, &self.config).map_err(|e| {
                        vec![TaskCreationError::FileParseFailed {
                            path: path.clone(),
                            error: e.to_string(),
                        }]
                    })?;
                    Ok(Some(file))
                } else {
                    Err(vec![TaskCreationError::FileNotFound(path)])
                }
            }
        }
    }

    /// Extract file path from a task reference like "path/to/file.md#task:id"
    ///
    /// Parses a full task reference and returns just the file path portion.
    ///
    /// # Arguments
    ///
    /// * `reference` - Full task reference string
    ///
    /// # Returns
    ///
    /// * `Ok(PathBuf)` - Extracted file path
    /// * `Err(Vec<TaskCreationError>)` - Invalid reference format
    fn extract_file_path(reference: &str) -> Result<PathBuf, Vec<TaskCreationError>> {
        // Split on '#' to get the file path part
        let file_part = reference
            .split('#')
            .next()
            .ok_or_else(|| {
                vec![TaskCreationError::InvalidPosition {
                    reason: format!("invalid task reference format: '{reference}'"),
                }]
            })?
            .trim();

        if file_part.is_empty() {
            return Err(vec![TaskCreationError::InvalidPosition {
                reason: format!("empty file path in task reference: '{reference}'"),
            }]);
        }

        Ok(PathBuf::from(file_part))
    }

    /// Generate an ID from a task title (slug format)
    ///
    /// Converts a task title to a valid slug-style ID by:
    /// - Converting to lowercase
    /// - Replacing non-alphanumeric characters with hyphens
    /// - Collapsing multiple hyphens
    /// - Trimming leading/trailing hyphens
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_core::creation::service::TaskCreationService;
    ///
    /// assert_eq!(
    ///     TaskCreationService::generate_id("Implement OAuth2 Flow"),
    ///     "implement-oauth2-flow"
    /// );
    ///
    /// assert_eq!(
    ///     TaskCreationService::generate_id("Fix bug #123"),
    ///     "fix-bug-123"
    /// );
    ///
    /// assert_eq!(
    ///     TaskCreationService::generate_id("Multiple   Spaces   Here"),
    ///     "multiple-spaces-here"
    /// );
    /// ```
    #[must_use]
    pub fn generate_id(title: &str) -> String {
        title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::config::ConfigBuilder;
    use lash_types::creation::TaskCreationRequestBuilder;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    // Helper to create a test file
    fn create_test_file(temp_dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = temp_dir.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_service_creation() {
        let config = ConfigBuilder::new().build().unwrap();
        // Just verify it constructs - if we get here without panicking, it works
        let _service = TaskCreationService::new(config);
    }

    #[test]
    fn test_generate_id_basic() {
        assert_eq!(
            TaskCreationService::generate_id("Simple Task"),
            "simple-task"
        );
        assert_eq!(
            TaskCreationService::generate_id("Implement OAuth2 Flow"),
            "implement-oauth2-flow"
        );
    }

    #[test]
    fn test_generate_id_special_chars() {
        assert_eq!(
            TaskCreationService::generate_id("Fix bug #123"),
            "fix-bug-123"
        );
        assert_eq!(
            TaskCreationService::generate_id("Add @mentions support"),
            "add-mentions-support"
        );
    }

    #[test]
    fn test_generate_id_multiple_spaces() {
        assert_eq!(
            TaskCreationService::generate_id("Multiple   Spaces   Here"),
            "multiple-spaces-here"
        );
    }

    #[test]
    fn test_generate_id_leading_trailing_hyphens() {
        assert_eq!(
            TaskCreationService::generate_id("---Leading and Trailing---"),
            "leading-and-trailing"
        );
    }

    #[test]
    fn test_extract_file_path_basic() {
        let path = TaskCreationService::extract_file_path("path/to/file.md#task:task-id").unwrap();
        assert_eq!(path, PathBuf::from("path/to/file.md"));
    }

    #[test]
    fn test_extract_file_path_no_fragment() {
        let path = TaskCreationService::extract_file_path("path/to/file.md").unwrap();
        assert_eq!(path, PathBuf::from("path/to/file.md"));
    }

    #[test]
    fn test_extract_file_path_with_whitespace() {
        let path = TaskCreationService::extract_file_path("  path/to/file.md  #task:id").unwrap();
        assert_eq!(path, PathBuf::from("path/to/file.md"));
    }

    #[test]
    fn test_extract_file_path_empty() {
        let result = TaskCreationService::extract_file_path("#task:id");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(matches!(
            errors[0],
            TaskCreationError::InvalidPosition { .. }
        ));
    }

    #[test]
    fn test_load_target_file_current() {
        let temp_dir = TempDir::new().unwrap();
        let config = ConfigBuilder::new().root(temp_dir.path()).build().unwrap();
        let service = TaskCreationService::new(config);

        let result = service.load_target_file(&FileTarget::Current);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_load_target_file_existing_path() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n\n## Tasks\n\n- [ ] Task 1\n";
        let path = create_test_file(temp_dir.path(), "test.md", content);

        let config = ConfigBuilder::new().root(temp_dir.path()).build().unwrap();
        let service = TaskCreationService::new(config);

        let result = service.load_target_file(&FileTarget::Path(path));
        assert!(result.is_ok());
        let file = result.unwrap();
        assert!(file.is_some());
        assert_eq!(file.unwrap().tasks.len(), 1);
    }

    #[test]
    fn test_load_target_file_nonexistent_path() {
        let temp_dir = TempDir::new().unwrap();
        let config = ConfigBuilder::new().root(temp_dir.path()).build().unwrap();
        let service = TaskCreationService::new(config);

        let path = temp_dir.path().join("nonexistent.md");
        let result = service.load_target_file(&FileTarget::Path(path));
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_load_target_file_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let config = ConfigBuilder::new().root(temp_dir.path()).build().unwrap();
        let service = TaskCreationService::new(config);

        let target = FileTarget::NewFile {
            path: temp_dir.path().join("new.md"),
            title: Some("New File".to_string()),
            description: None,
        };

        let result = service.load_target_file(&target);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_load_target_file_containing_task_exists() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n\n## Tasks\n\n- [ ] Task 1\n";
        let path = create_test_file(temp_dir.path(), "test.md", content);

        let config = ConfigBuilder::new().root(temp_dir.path()).build().unwrap();
        let service = TaskCreationService::new(config);

        let reference = format!("{}#task:task-1", path.display());
        let result = service.load_target_file(&FileTarget::ContainingTask(reference));
        assert!(result.is_ok());
        let file = result.unwrap();
        assert!(file.is_some());
    }

    #[test]
    fn test_load_target_file_containing_task_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let config = ConfigBuilder::new().root(temp_dir.path()).build().unwrap();
        let service = TaskCreationService::new(config);

        let path = temp_dir.path().join("nonexistent.md");
        let reference = format!("{}#task:task-1", path.display());
        let result = service.load_target_file(&FileTarget::ContainingTask(reference));
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(matches!(errors[0], TaskCreationError::FileNotFound(_)));
    }

    #[test]
    fn test_load_target_file_parse_failure() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n\n## Tasks\n\n- [*] Invalid checkbox\n";
        let path = create_test_file(temp_dir.path(), "bad.md", content);

        let config = ConfigBuilder::new().root(temp_dir.path()).build().unwrap();
        let service = TaskCreationService::new(config);

        let result = service.load_target_file(&FileTarget::Path(path));
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(matches!(
            errors[0],
            TaskCreationError::FileParseFailed { .. }
        ));
    }

    #[test]
    fn test_create_task_in_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let config = ConfigBuilder::new().root(temp_dir.path()).build().unwrap();
        let service = TaskCreationService::new(config);

        let file_path = temp_dir.path().join("new-file.md");
        let request = TaskCreationRequestBuilder::new("First task")
            .new_file(
                file_path.clone(),
                Some("Test File".to_string()),
                Some("Test description".to_string()),
            )
            .id("first-task")
            .label("test")
            .build();

        let result = service.create_task(&request);
        assert!(result.is_ok(), "Expected success, got: {result:?}");

        let res = result.unwrap();
        assert_eq!(res.task_id, "first-task");
        assert_eq!(res.file_path, file_path);
        assert!(res.is_new_file);

        // Verify file was created
        assert!(file_path.exists());
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("# Test File"));
        assert!(content.contains("- [ ] First task #test"));
        // GitHub issue #24: the explicit --id must be persisted as @id: so
        // the task resolves as `new-file#first-task` after creation.
        assert!(content.contains("@id: first-task"));
    }

    #[test]
    fn test_create_task_in_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test File\n\n## Tasks\n\n- [ ] Task 1\n";
        let path = create_test_file(temp_dir.path(), "test.md", content);

        let config = ConfigBuilder::new().root(temp_dir.path()).build().unwrap();
        let service = TaskCreationService::new(config);

        let request = TaskCreationRequestBuilder::new("Task 2")
            .file_path(path.clone())
            .build();

        let result = service.create_task(&request);
        assert!(result.is_ok(), "Expected success, got: {result:?}");

        let res = result.unwrap();
        assert!(!res.is_new_file);

        // Verify task was added
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("- [ ] Task 1"));
        assert!(content.contains("- [ ] Task 2"));
    }

    #[test]
    fn test_create_task_with_validation_error() {
        let temp_dir = TempDir::new().unwrap();
        let config = ConfigBuilder::new().root(temp_dir.path()).build().unwrap();
        let service = TaskCreationService::new(config);

        // Empty title should fail validation
        let request = TaskCreationRequestBuilder::new("   ")
            .file_path(temp_dir.path().join("test.md"))
            .build();

        let result = service.create_task(&request);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| matches!(e, TaskCreationError::EmptyTitle)));
    }

    #[test]
    fn test_create_task_nested_under_parent() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test File\n\n## Tasks\n\n- [ ] Parent task\n";
        let path = create_test_file(temp_dir.path(), "test.md", content);

        let config = ConfigBuilder::new().root(temp_dir.path()).build().unwrap();
        let service = TaskCreationService::new(config);

        let request = TaskCreationRequestBuilder::new("Child task")
            .file_path(path.clone())
            .parent_id("parent-task")
            .build();

        let result = service.create_task(&request);
        assert!(result.is_ok(), "Expected success, got: {result:?}");

        // Verify nested structure
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("- [ ] Parent task"));
        assert!(content.contains("  - [ ] Child task")); // Should be indented
    }

    #[test]
    fn test_create_task_parent_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test File\n\n## Tasks\n\n- [ ] Task 1\n";
        let path = create_test_file(temp_dir.path(), "test.md", content);

        let config = ConfigBuilder::new().root(temp_dir.path()).build().unwrap();
        let service = TaskCreationService::new(config);

        let request = TaskCreationRequestBuilder::new("Child task")
            .file_path(path)
            .parent_id("nonexistent-parent")
            .build();

        let result = service.create_task(&request);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| matches!(e, TaskCreationError::ParentNotFound { .. })));
    }

    #[test]
    fn test_create_task_with_dependencies() {
        let temp_dir = TempDir::new().unwrap();
        let config = ConfigBuilder::new().root(temp_dir.path()).build().unwrap();
        let service = TaskCreationService::new(config);

        let file_path = temp_dir.path().join("tasks.md");
        let request = TaskCreationRequestBuilder::new("Task with deps")
            .new_file(file_path.clone(), Some("Tasks".to_string()), None)
            .depends_on("other-task.md#task:dependency")
            .agent_note("This depends on something")
            .build();

        let result = service.create_task(&request);
        assert!(result.is_ok(), "Expected success, got: {result:?}");

        // Verify dependencies are in the file
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("@depends-on: other-task.md#task:dependency"));
        assert!(content.contains("@agent-note: This depends on something"));
    }

    #[test]
    fn test_create_task_multiple_validation_errors() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test File\n\n## Tasks\n\n- [ ] Task 1\n";
        let path = create_test_file(temp_dir.path(), "test.md", content);

        let config = ConfigBuilder::new().root(temp_dir.path()).build().unwrap();
        let service = TaskCreationService::new(config);

        // Multiple errors: empty title, invalid label
        let request = TaskCreationRequestBuilder::new("   ")
            .file_path(path)
            .label("Invalid Label!")
            .build();

        let result = service.create_task(&request);
        assert!(result.is_err());
        let errors = result.unwrap_err();

        // Should have multiple errors
        assert!(errors.len() >= 2);
        assert!(errors
            .iter()
            .any(|e| matches!(e, TaskCreationError::EmptyTitle)));
        assert!(errors
            .iter()
            .any(|e| matches!(e, TaskCreationError::InvalidLabel { .. })));
    }
}
