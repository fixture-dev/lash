//! Markdown emitter for task creation
//!
//! This module provides the `MarkdownEmitter` which generates Markdown output
//! for newly created tasks and writes them to files. It handles both inserting
//! tasks into existing files and creating new task files with proper structure.

use lash_types::creation::{FileTarget, TaskCreationRequest, TaskCreationResult};
use lash_types::creation_errors::TaskCreationError;
use lash_types::TaskStatus;
use std::fs;
use std::path::{Path, PathBuf};

use super::placement::PlacementInfo;
use super::validation::ValidationContext;

/// Markdown emitter for task creation
///
/// Generates properly formatted Markdown for tasks and writes them to files.
pub struct MarkdownEmitter;

impl MarkdownEmitter {
    /// Main entry point - emit a task to a file
    ///
    /// Takes a validated task creation request and placement information,
    /// generates Markdown, and writes it to the appropriate file.
    ///
    /// # Arguments
    ///
    /// * `request` - The validated task creation request
    /// * `ctx` - Validation context containing resolved file and parent info
    /// * `placement` - Placement information (line number, indent, etc.)
    ///
    /// # Returns
    ///
    /// Returns `TaskCreationResult` with the task ID, file path, and line number.
    ///
    /// # Errors
    ///
    /// Returns `TaskCreationError::IoError` if file operations fail.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_core::creation::{MarkdownEmitter, PlacementInfo};
    /// use lash_core::creation::{TaskValidator, ValidationContext};
    /// use lash_types::config::ConfigBuilder;
    /// use lash_types::creation::{TaskCreationRequestBuilder, FileTarget};
    /// use std::path::PathBuf;
    ///
    /// # let config = ConfigBuilder::new().build().unwrap();
    /// # let validator = TaskValidator::new(config);
    /// # let request = TaskCreationRequestBuilder::new("Test task")
    /// #     .id("test-id")
    /// #     .build();
    /// # let ctx = validator.validate(&request, None).unwrap();
    /// let placement = PlacementInfo {
    ///     line_number: 5,
    ///     order_index: 0,
    ///     indent_level: 0,
    /// };
    ///
    /// // Note: This would perform actual file I/O in practice
    /// // let result = MarkdownEmitter::emit(&request, &ctx, &placement)?;
    /// ```
    pub fn emit(
        request: &TaskCreationRequest,
        ctx: &ValidationContext,
        placement: &PlacementInfo,
    ) -> Result<TaskCreationResult, TaskCreationError> {
        // Generate task ID if not provided
        let task_id = if let Some(ref id) = request.id {
            id.clone()
        } else {
            Self::synthesize_id(&request.title)
        };

        // Determine status (default to Open if not specified)
        let status = request.status.unwrap_or(TaskStatus::Open);

        // Check if we're creating a new file
        let is_new_file = matches!(request.file_target, FileTarget::NewFile { .. });

        // Format the task line (without metadata - that goes in annotations)
        let task_line = Self::format_task_line(
            &request.title,
            status,
            placement.indent_level,
            &request.labels,
        );

        // Format annotation lines. An explicit `--id` is written as `@id:` so
        // the task's global id (`file#id`) resolves after creation (GitHub
        // issue #24) — without this the ID was accepted, echoed in the
        // success message, and then silently dropped. Auto-synthesized ids
        // (no explicit `--id`) are still not persisted: the parser
        // re-synthesizes them from the title, matching existing behavior.
        // @owner/@estimate remain internal-only, as before.
        let annotation_lines = Self::format_task_annotations(
            placement.indent_level,
            request.id.as_deref(),
            &request.depends_on,
            request.agent_note.as_deref(),
        );

        // Either create new file or insert into existing file
        let file_path = if is_new_file {
            Self::create_new_file(request, &task_line, &annotation_lines)?
        } else {
            // Get the file path from the resolved file in context
            let file_path = ctx.resolved_file.path.clone();
            Self::insert_into_existing(
                &file_path,
                &task_line,
                &annotation_lines,
                placement.line_number,
            )?;
            file_path
        };

        Ok(TaskCreationResult {
            task_id,
            file_path,
            line_number: placement.line_number,
            is_new_file,
        })
    }

    /// Insert task into an existing file
    ///
    /// Reads the file, inserts the task line and annotations at the specified
    /// line number, and writes the file back atomically.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the existing file
    /// * `task_line` - The formatted task line
    /// * `annotation_lines` - Additional annotation lines to insert
    /// * `line_number` - Line number to insert at (1-indexed)
    ///
    /// # Errors
    ///
    /// Returns `TaskCreationError::IoError` if file operations fail.
    fn insert_into_existing(
        file_path: &Path,
        task_line: &str,
        annotation_lines: &[String],
        line_number: usize,
    ) -> Result<(), TaskCreationError> {
        // Read the file
        let content = fs::read_to_string(file_path).map_err(|e| TaskCreationError::IoError {
            path: file_path.to_path_buf(),
            error: format!("Failed to read file: {e}"),
        })?;

        let lines: Vec<&str> = content.lines().collect();
        let mut new_lines = Vec::new();

        // Calculate insertion index (line_number is 1-indexed)
        let insert_idx = if line_number == 0 {
            0
        } else {
            (line_number - 1).min(lines.len())
        };

        // Copy lines before insertion point
        new_lines.extend_from_slice(&lines[..insert_idx]);

        // Insert task line
        new_lines.push(task_line);

        // Insert annotation lines
        for ann_line in annotation_lines {
            new_lines.push(ann_line);
        }

        // Copy remaining lines
        if insert_idx < lines.len() {
            new_lines.extend_from_slice(&lines[insert_idx..]);
        }

        // Write back atomically
        Self::write_file_atomic(file_path, &new_lines.join("\n"))?;

        Ok(())
    }

    /// Create a new task file with proper structure
    ///
    /// Generates a complete task file with header, metadata, and the first task.
    ///
    /// # Arguments
    ///
    /// * `request` - The task creation request
    /// * `task_line` - The formatted task line
    /// * `annotation_lines` - Additional annotation lines
    ///
    /// # Returns
    ///
    /// Returns the path to the newly created file.
    ///
    /// # Errors
    ///
    /// Returns `TaskCreationError::IoError` if file operations fail.
    fn create_new_file(
        request: &TaskCreationRequest,
        task_line: &str,
        annotation_lines: &[String],
    ) -> Result<PathBuf, TaskCreationError> {
        // Extract file details from FileTarget::NewFile
        let (path, title, description) = match &request.file_target {
            FileTarget::NewFile {
                path,
                title,
                description,
            } => (path.clone(), title.clone(), description.clone()),
            _ => {
                return Err(TaskCreationError::IoError {
                    path: PathBuf::new(),
                    error: "Expected NewFile target".to_string(),
                })
            }
        };

        // Generate file ID from filename
        let file_id = Self::synthesize_id_from_path(&path);

        // Build the file content
        let mut content = String::new();

        // 1. Title
        let file_title = title.unwrap_or_else(|| {
            // Use the task title or derive from filename
            request.title.clone()
        });
        content.push_str("# ");
        content.push_str(&file_title);
        content.push('\n');
        content.push('\n');

        // 2. File-level annotations
        content.push_str("@id: ");
        content.push_str(&file_id);
        content.push('\n');

        // Add file-level labels if provided
        if !request.labels.is_empty() {
            content.push_str("@labels: ");
            content.push_str(&request.labels.join(", "));
            content.push('\n');
        }

        // Add file-level owner if provided
        if let Some(ref owner) = request.owner {
            content.push_str("@owner: ");
            content.push_str(owner);
            content.push('\n');
        }

        content.push('\n');

        // 3. Description section (if provided)
        if let Some(ref desc) = description {
            content.push_str("## Description\n");
            content.push('\n');
            content.push_str(desc);
            content.push('\n');
            content.push('\n');
        }

        // 4. Tasks section
        content.push_str("## Tasks\n");
        content.push('\n');
        content.push_str(task_line);
        content.push('\n');

        // Add annotation lines
        for ann_line in annotation_lines {
            content.push_str(ann_line);
            content.push('\n');
        }

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| TaskCreationError::IoError {
                    path: parent.to_path_buf(),
                    error: format!("Failed to create parent directories: {e}"),
                })?;
            }
        }

        // Write file atomically
        Self::write_file_atomic(&path, &content)?;

        Ok(path)
    }

    /// Format a single task line in Markdown
    ///
    /// Generates a properly formatted checkbox line with optional inline labels.
    /// Note: Task-level annotations (@id, @owner, etc.) should be formatted separately
    /// using `format_annotation_lines`.
    ///
    /// # Arguments
    ///
    /// * `title` - The task title
    /// * `status` - The task status (for checkbox character)
    /// * `indent` - Indentation level (0 for top-level)
    /// * `inline_labels` - Labels to include inline (e.g., `#backend`)
    ///
    /// # Returns
    ///
    /// A formatted task line string.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_core::creation::emitter::MarkdownEmitter;
    /// use lash_types::TaskStatus;
    ///
    /// let line = MarkdownEmitter::format_task_line(
    ///     "Implement feature",
    ///     TaskStatus::Open,
    ///     0,
    ///     &["backend".to_string(), "api".to_string()],
    /// );
    ///
    /// assert!(line.contains("- [ ] Implement feature"));
    /// assert!(line.contains("#backend"));
    /// assert!(line.contains("#api"));
    /// ```
    #[must_use]
    pub fn format_task_line(
        title: &str,
        status: TaskStatus,
        indent: usize,
        inline_labels: &[String],
    ) -> String {
        let indent_str = "  ".repeat(indent);
        let checkbox_char = status.to_checkbox_char();

        let mut line = format!("{indent_str}- [{checkbox_char}] {title}");

        // Add inline labels
        for label in inline_labels {
            line.push_str(" #");
            line.push_str(label);
        }

        line
    }

    /// Format task annotations (`@id`, `@depends-on`, `@agent-note`)
    ///
    /// Generates annotation lines with proper indentation (2 spaces deeper
    /// than the task checkbox line, matching the format documented in
    /// `docs/agent-guide.md` and `docs/design-doc.md`).
    ///
    /// Note: `@owner`/`@estimate` are still NOT stored in Markdown format —
    /// they remain internal-only, synthesized/derived elsewhere. `@id` IS
    /// written when the caller supplied an explicit id, since it is what
    /// gives the task a stable, resolvable global id (`file#id`).
    ///
    /// # Arguments
    ///
    /// * `indent` - Base indentation level (for the task checkbox line)
    /// * `id` - Explicit task id, if one was provided (`--id`)
    /// * `depends_on` - List of dependency references
    /// * `agent_note` - Optional agent note
    ///
    /// # Returns
    ///
    /// A vector of formatted annotation lines, `@id` first.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_core::creation::emitter::MarkdownEmitter;
    ///
    /// let lines = MarkdownEmitter::format_task_annotations(
    ///     0,
    ///     Some("auth-impl"),
    ///     &["tasks/core.md#task:session-manager".to_string()],
    ///     Some("Consider using existing auth middleware"),
    /// );
    ///
    /// assert_eq!(lines[0], "  @id: auth-impl");
    /// assert!(lines.iter().any(|l| l.contains("@depends-on:")));
    /// assert!(lines.iter().any(|l| l.contains("@agent-note:")));
    /// ```
    #[must_use]
    pub fn format_task_annotations(
        indent: usize,
        id: Option<&str>,
        depends_on: &[String],
        agent_note: Option<&str>,
    ) -> Vec<String> {
        let indent_str = "  ".repeat(indent);
        let annotation_indent = format!("{indent_str}  "); // Extra 2 spaces for annotations
        let mut lines = Vec::new();

        // @id comes first, matching the documented annotation order.
        if let Some(id) = id {
            lines.push(format!("{annotation_indent}@id: {id}"));
        }

        // Add @depends-on if present
        for dep in depends_on {
            lines.push(format!("{annotation_indent}@depends-on: {dep}"));
        }

        // Add @agent-note if present
        if let Some(note) = agent_note {
            lines.push(format!("{annotation_indent}@agent-note: {note}"));
        }

        lines
    }

    /// Synthesize a task ID from the task title
    ///
    /// Converts the title to a slug-like format (lowercase, hyphens for spaces).
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_core::creation::emitter::MarkdownEmitter;
    ///
    /// assert_eq!(
    ///     MarkdownEmitter::synthesize_id("Implement OAuth2 Flow"),
    ///     "implement-oauth2-flow"
    /// );
    /// ```
    #[must_use]
    pub fn synthesize_id(title: &str) -> String {
        title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            // Remove leading/trailing hyphens and collapse multiple hyphens
            .trim_matches('-')
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }

    /// Synthesize a file ID from the file path
    ///
    /// Uses the filename (without extension) to generate an ID.
    fn synthesize_id_from_path(path: &Path) -> String {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map_or_else(|| "file".to_string(), Self::synthesize_id)
    }

    /// Write file atomically using a temporary file
    ///
    /// Writes to a temp file first, then renames to the target path.
    fn write_file_atomic(path: &Path, content: &str) -> Result<(), TaskCreationError> {
        // Create a temporary file in the same directory
        let temp_path = path.with_extension("tmp");

        // Write to temp file
        fs::write(&temp_path, content).map_err(|e| TaskCreationError::IoError {
            path: temp_path.clone(),
            error: format!("Failed to write temporary file: {e}"),
        })?;

        // Rename to target path (atomic on most platforms)
        fs::rename(&temp_path, path).map_err(|e| {
            // Clean up temp file on error
            let _ = fs::remove_file(&temp_path);
            TaskCreationError::IoError {
                path: path.to_path_buf(),
                error: format!("Failed to rename temporary file: {e}"),
            }
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::config::ConfigBuilder;
    use lash_types::creation::TaskCreationRequestBuilder;

    use crate::creation::validation::TaskValidator;

    #[test]
    fn test_synthesize_id_basic() {
        assert_eq!(MarkdownEmitter::synthesize_id("Simple Task"), "simple-task");
        assert_eq!(
            MarkdownEmitter::synthesize_id("Implement OAuth2 Flow"),
            "implement-oauth2-flow"
        );
    }

    #[test]
    fn test_synthesize_id_special_chars() {
        assert_eq!(
            MarkdownEmitter::synthesize_id("Fix bug #123"),
            "fix-bug-123"
        );
        assert_eq!(
            MarkdownEmitter::synthesize_id("Add @mentions support"),
            "add-mentions-support"
        );
    }

    #[test]
    fn test_synthesize_id_multiple_spaces() {
        assert_eq!(
            MarkdownEmitter::synthesize_id("Multiple   Spaces   Here"),
            "multiple-spaces-here"
        );
    }

    #[test]
    fn test_format_task_line_open() {
        let line = MarkdownEmitter::format_task_line("Simple task", TaskStatus::Open, 0, &[]);
        assert_eq!(line, "- [ ] Simple task");
    }

    #[test]
    fn test_format_task_line_done() {
        let line = MarkdownEmitter::format_task_line("Completed task", TaskStatus::Done, 0, &[]);
        assert_eq!(line, "- [x] Completed task");
    }

    #[test]
    fn test_format_task_line_waived() {
        let line = MarkdownEmitter::format_task_line("Not applicable", TaskStatus::Waived, 0, &[]);
        assert_eq!(line, "- [-] Not applicable");
    }

    #[test]
    fn test_format_task_line_blocked() {
        let line = MarkdownEmitter::format_task_line("Blocked task", TaskStatus::Blocked, 0, &[]);
        assert_eq!(line, "- [!] Blocked task");
    }

    #[test]
    fn test_format_task_line_with_indent() {
        let line = MarkdownEmitter::format_task_line("Nested task", TaskStatus::Open, 2, &[]);
        assert_eq!(line, "    - [ ] Nested task");
    }

    #[test]
    fn test_format_task_line_with_labels() {
        let line = MarkdownEmitter::format_task_line(
            "Task with labels",
            TaskStatus::Open,
            0,
            &["backend".to_string(), "security".to_string()],
        );
        assert_eq!(line, "- [ ] Task with labels #backend #security");
    }

    #[test]
    fn test_format_task_line_with_labels_and_indent() {
        let line = MarkdownEmitter::format_task_line(
            "Complex task",
            TaskStatus::Open,
            1,
            &["backend".to_string()],
        );
        assert_eq!(line, "  - [ ] Complex task #backend");
    }

    #[test]
    fn test_format_task_annotations_empty() {
        let lines = MarkdownEmitter::format_task_annotations(0, None, &[], None);
        assert_eq!(lines.len(), 0);
    }

    #[test]
    fn test_format_task_annotations_depends_on() {
        let lines = MarkdownEmitter::format_task_annotations(
            0,
            None,
            &["tasks/core.md#task:session-manager".to_string()],
            None,
        );
        assert_eq!(lines.len(), 1);
        assert_eq!(
            lines[0],
            "  @depends-on: tasks/core.md#task:session-manager"
        );
    }

    #[test]
    fn test_format_task_annotations_agent_note() {
        let lines = MarkdownEmitter::format_task_annotations(
            0,
            None,
            &[],
            Some("Consider using existing auth middleware"),
        );
        assert_eq!(lines.len(), 1);
        assert_eq!(
            lines[0],
            "  @agent-note: Consider using existing auth middleware"
        );
    }

    #[test]
    fn test_format_task_annotations_both() {
        let lines = MarkdownEmitter::format_task_annotations(
            0,
            None,
            &["dep1".to_string(), "dep2".to_string()],
            Some("Note here"),
        );
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("@depends-on: dep1"));
        assert!(lines[1].contains("@depends-on: dep2"));
        assert!(lines[2].contains("@agent-note: Note here"));
    }

    #[test]
    fn test_format_task_annotations_with_indent() {
        let lines = MarkdownEmitter::format_task_annotations(2, None, &["dep".to_string()], None);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].starts_with("      ")); // 4 spaces indent + 2 for annotation
    }

    #[test]
    fn test_format_task_annotations_id_written_first() {
        // GitHub issue #24: an explicit --id must be persisted as `@id:`,
        // ahead of any other annotations, so the task resolves as `file#id`.
        let lines = MarkdownEmitter::format_task_annotations(
            0,
            Some("auth-impl"),
            &["dep1".to_string()],
            Some("note"),
        );
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "  @id: auth-impl");
        assert!(lines[1].contains("@depends-on: dep1"));
        assert!(lines[2].contains("@agent-note: note"));
    }

    #[test]
    fn test_format_task_annotations_id_only() {
        let lines = MarkdownEmitter::format_task_annotations(0, Some("short-e"), &[], None);
        assert_eq!(lines, vec!["  @id: short-e".to_string()]);
    }

    #[test]
    fn test_create_new_file() {
        let temp_dir = std::env::temp_dir().join("lash-test-emitter");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let file_path = temp_dir.join("new-file.md");

        let request = TaskCreationRequestBuilder::new("First task")
            .new_file(
                file_path.clone(),
                Some("Test File".to_string()),
                Some("This is a test file.".to_string()),
            )
            .id("first-task")
            .label("test")
            .owner("alice")
            .build();

        let task_line = "- [ ] First task #test";
        let annotation_lines = vec![]; // No task-level annotations

        let result = MarkdownEmitter::create_new_file(&request, task_line, &annotation_lines);
        assert!(result.is_ok());

        // Verify file content
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("# Test File"));
        assert!(content.contains("@id: new-file"));
        assert!(content.contains("@labels: test"));
        assert!(content.contains("@owner: alice"));
        assert!(content.contains("## Description"));
        assert!(content.contains("This is a test file."));
        assert!(content.contains("## Tasks"));
        assert!(content.contains("- [ ] First task #test"));
        // No task-level @id in markdown format

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_insert_into_existing() {
        let temp_dir = std::env::temp_dir().join("lash-test-emitter-insert");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let file_path = temp_dir.join("existing.md");

        // Create initial file
        let initial_content =
            "# Existing File\n\n@id: existing\n\n## Tasks\n\n- [ ] Task 1\n- [ ] Task 2\n";
        fs::write(&file_path, initial_content).unwrap();

        // Insert a new task between Task 1 and Task 2
        let task_line = "- [ ] New Task";
        let annotation_lines = vec![]; // No task-level annotations

        let result = MarkdownEmitter::insert_into_existing(
            &file_path,
            task_line,
            &annotation_lines,
            8, // Line 8 (after Task 1)
        );
        assert!(result.is_ok());

        // Verify content
        let content = fs::read_to_string(&file_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        // Find the new task
        let new_task_idx = lines.iter().position(|l| l.contains("New Task")).unwrap();
        assert_eq!(lines[new_task_idx], "- [ ] New Task");

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_emit_full_flow() {
        let temp_dir = std::env::temp_dir().join("lash-test-emitter-full");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let file_path = temp_dir.join("full-test.md");

        // Create a request
        let request = TaskCreationRequestBuilder::new("Test task")
            .new_file(file_path.clone(), Some("Full Test".to_string()), None)
            .label("backend")
            .owner("bob")
            .estimate("3h")
            .depends_on("dep1")
            .agent_note("This is a test")
            .build();

        // Create validation context
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);
        let ctx = validator.validate(&request, None).unwrap();

        // Create placement info
        let placement = PlacementInfo {
            line_number: 1,
            order_index: 0,
            indent_level: 0,
        };

        // Emit
        let result = MarkdownEmitter::emit(&request, &ctx, &placement);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.task_id, "test-task");
        assert_eq!(result.file_path, file_path);
        assert!(result.is_new_file);

        // Verify file content
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("# Full Test"));
        assert!(content.contains("## Tasks"));
        assert!(content.contains("- [ ] Test task #backend"));
        // Note: Task-level @id, @owner, @estimate are NOT in markdown format
        // Only file-level and dependency annotations are present
        assert!(content.contains("@depends-on: dep1"));
        assert!(content.contains("@agent-note: This is a test"));

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_round_trip() {
        let temp_dir = std::env::temp_dir().join("lash-test-emitter-roundtrip");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let file_path = temp_dir.join("roundtrip.md");

        // Create a task
        let request = TaskCreationRequestBuilder::new("Round trip task")
            .new_file(file_path.clone(), Some("Round Trip".to_string()), None)
            .id("roundtrip-task")
            .label("test")
            .build();

        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config.clone());
        let ctx = validator.validate(&request, None).unwrap();

        let placement = PlacementInfo {
            line_number: 1,
            order_index: 0,
            indent_level: 0,
        };

        let result = MarkdownEmitter::emit(&request, &ctx, &placement);
        assert!(result.is_ok());

        // Now parse the file
        let parsed = crate::parser::parse_file(&file_path, &config);
        assert!(parsed.is_ok());

        let parsed_file = parsed.unwrap();
        assert_eq!(parsed_file.title, "Round Trip");
        assert_eq!(parsed_file.id, "roundtrip");
        assert_eq!(parsed_file.tasks.tasks().len(), 1);

        let task = &parsed_file.tasks.tasks()[0];
        // Note: Parser includes inline labels in title
        assert_eq!(task.title, "Round trip task #test");
        // GitHub issue #24: an explicit --id is now persisted as `@id:`, so
        // it round-trips back out of the parser unchanged (previously the
        // parser would re-synthesize a slug from the title instead).
        assert_eq!(task.id, "roundtrip-task");
        assert!(task.has_explicit_id);
        assert_eq!(task.status, TaskStatus::Open);
        // Verify the label was parsed
        assert_eq!(task.metadata.labels, vec!["test"]);

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
