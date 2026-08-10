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

use super::placement::{InsertAnchor, PlacementInfo};
use super::validation::ValidationContext;
use crate::parser::{checkbox, header};

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
    /// use lash_core::creation::{InsertAnchor, MarkdownEmitter, PlacementInfo};
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
    ///     anchor: InsertAnchor::Line(5),
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
        let (file_path, line_number) = if is_new_file {
            Self::create_new_file(request, &task_line, &annotation_lines)?
        } else {
            // Get the file path from the resolved file in context
            let file_path = ctx.resolved_file.path.clone();
            let line_number =
                Self::insert_into_existing(&file_path, &task_line, &annotation_lines, placement)?;
            (file_path, line_number)
        };

        Ok(TaskCreationResult {
            task_id,
            file_path,
            line_number,
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
    /// * `placement` - Where the resolver decided the task goes
    ///
    /// # Returns
    ///
    /// The 1-indexed line the task was written to.
    ///
    /// # Errors
    ///
    /// Returns `TaskCreationError::IoError` if file operations fail.
    fn insert_into_existing(
        file_path: &Path,
        task_line: &str,
        annotation_lines: &[String],
        placement: &PlacementInfo,
    ) -> Result<usize, TaskCreationError> {
        // Read the file
        let content = fs::read_to_string(file_path).map_err(|e| TaskCreationError::IoError {
            path: file_path.to_path_buf(),
            error: format!("Failed to read file: {e}"),
        })?;

        let lines: Vec<&str> = content.lines().collect();
        let insert_idx = Self::resolve_insert_index(&content, &lines, placement.anchor);

        let mut new_lines = Vec::new();

        // Copy lines before insertion point
        new_lines.extend_from_slice(&lines[..insert_idx]);

        // Separate the new task from a preceding task's body. Landing directly
        // under the last line of someone else's prose reads as a continuation
        // of it, which is the confusion this insertion point exists to avoid.
        // Tasks with no body keep butting up against each other as before.
        let follows_body = insert_idx
            .checked_sub(1)
            .is_some_and(|prev| Self::continues_task_block(lines[prev]));
        if follows_body {
            new_lines.push("");
        }

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

        // Write back atomically. The trailing newline is restored explicitly:
        // `join` alone leaves the file ending mid-line, which shows up in every
        // subsequent diff as "\ No newline at end of file".
        let mut updated = new_lines.join("\n");
        updated.push('\n');
        Self::write_file_atomic(file_path, &updated)?;

        Ok(if follows_body {
            insert_idx + 2
        } else {
            insert_idx + 1
        })
    }

    /// Translate a [`InsertAnchor`] into a 0-indexed position in `lines`
    ///
    /// [`InsertAnchor::EndOfTasksSection`] is resolved here rather than in the
    /// resolver because it depends on the source text: a parsed file records
    /// task line numbers but no section boundaries, so a `## Tasks` section
    /// with no tasks in it has nothing to anchor to. The new task goes after
    /// the section's last content line, keeping it inside `## Tasks` and below
    /// the file header.
    fn resolve_insert_index(content: &str, lines: &[&str], anchor: InsertAnchor) -> usize {
        match anchor {
            InsertAnchor::Line(line_number) => line_number.saturating_sub(1).min(lines.len()),
            InsertAnchor::AfterTaskBlock(line_number) => {
                let start = line_number.saturating_sub(1).min(lines.len());
                Self::skip_task_body(lines, start)
            }
            InsertAnchor::EndOfTasksSection => {
                // No `## Tasks` heading at all: append at the end of the file
                // rather than guessing where the section would have been.
                let Some(body) = header::tasks_section_body(content) else {
                    return lines.len();
                };
                let body_end = body.end.min(lines.len());
                let last_content = lines[body.start..body_end]
                    .iter()
                    .rposition(|line| !line.trim().is_empty());

                match last_content {
                    Some(offset) => body.start + offset + 1,
                    // Section is empty. Land on the line after the heading's
                    // blank separator so the file keeps its usual
                    // heading/blank/task shape.
                    None => (body.start + 1).min(body_end),
                }
            }
        }
    }

    /// Advance past the body lines that belong to the preceding task
    ///
    /// A parsed task records its checkbox line and its annotation lines and
    /// nothing else, so the free-text body a task may carry underneath —
    /// prose, numbered steps, acceptance criteria, indented contextual note
    /// bullets — is invisible to the placement resolver. Anchoring on parsed
    /// data alone therefore lands "after this task" between the task's title
    /// and its own body, which silently reassigns that body to the newly
    /// inserted task (GitHub issue #48). Only the source text says where the
    /// block actually ends.
    ///
    /// A line continues the block when it is indented and is not itself a
    /// checkbox; a checkbox at any depth starts a new task, and an
    /// unindented line has left the block. Blank lines are consumed only when
    /// indented content resumes after them, so a body split into paragraphs
    /// stays whole while a blank line that genuinely ends the block still
    /// stops the scan.
    ///
    /// `start` is a 0-indexed position in `lines`; the return value is the
    /// 0-indexed position to insert at.
    fn skip_task_body(lines: &[&str], start: usize) -> usize {
        let mut idx = start;

        while idx < lines.len() {
            if lines[idx].trim().is_empty() {
                // A blank line only belongs to the block if the block resumes
                // after it. Otherwise the block ended at this blank line.
                let resumes = lines[idx..]
                    .iter()
                    .position(|line| !line.trim().is_empty())
                    .map(|offset| idx + offset)
                    .filter(|&next| Self::continues_task_block(lines[next]));

                match resumes {
                    Some(next) => idx = next,
                    None => break,
                }
            } else if Self::continues_task_block(lines[idx]) {
                idx += 1;
            } else {
                break;
            }
        }

        idx
    }

    /// Does this line continue the block that a task's checkbox line opened?
    ///
    /// Indented non-checkbox content belongs to the task above it: body prose,
    /// folded annotation values, contextual note bullets. A checkbox at any
    /// depth starts a new task, and an unindented line has left the block
    /// entirely.
    fn continues_task_block(line: &str) -> bool {
        !line.trim().is_empty()
            && line.starts_with(char::is_whitespace)
            && checkbox::CheckboxLine::parse(line, 1).is_none()
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
    /// Returns the path to the newly created file and the 1-indexed line the
    /// task was written to.
    ///
    /// # Errors
    ///
    /// Returns `TaskCreationError::IoError` if file operations fail.
    fn create_new_file(
        request: &TaskCreationRequest,
        task_line: &str,
        annotation_lines: &[String],
    ) -> Result<(PathBuf, usize), TaskCreationError> {
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
        let task_line_number = content.lines().count() + 1;
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

        Ok((path, task_line_number))
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
    ///
    /// A note spanning several lines becomes one `@agent-note:` line plus one
    /// indented continuation line each, which is the shape the parser folds
    /// back into a single value:
    ///
    /// ```
    /// use lash_core::creation::emitter::MarkdownEmitter;
    ///
    /// let lines = MarkdownEmitter::format_task_annotations(
    ///     0,
    ///     None,
    ///     &[],
    ///     Some("first line\nsecond line"),
    /// );
    ///
    /// assert_eq!(lines, ["  @agent-note: first line", "  second line"]);
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

        // Add @agent-note if present. A note may span several lines; each
        // continuation has to carry the annotation indent or the parser will
        // stop at it and silently drop the rest of the note (it treats an
        // unindented line as the end of the annotation block).
        if let Some(note) = agent_note {
            let mut note_lines = note.lines();
            let first = note_lines.next().unwrap_or("");
            lines.push(format!("{annotation_indent}@agent-note: {first}"));
            for continuation in note_lines {
                lines.push(format!("{annotation_indent}{continuation}"));
            }
        }

        lines
    }

    /// Whether an agent note survives being written and parsed back
    ///
    /// The parser folds indented continuation lines into a single value, but
    /// the folding is lossy in two ways that matter here: it skips blank
    /// lines, and it treats a line starting with `@` as the beginning of a new
    /// annotation rather than as note text. A note containing either is
    /// rejected at validation time instead of being written into a file the
    /// parser will silently truncate.
    ///
    /// Leading whitespace on a continuation line is normalized away rather
    /// than rejected: the parser trims it, so the indentation is lost but no
    /// text is.
    ///
    /// # Errors
    ///
    /// Returns the reason the note cannot round-trip.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_core::creation::emitter::MarkdownEmitter;
    ///
    /// assert!(MarkdownEmitter::check_agent_note("one line").is_ok());
    /// assert!(MarkdownEmitter::check_agent_note("first\nsecond").is_ok());
    /// assert!(MarkdownEmitter::check_agent_note("first\n\nthird").is_err());
    /// assert!(MarkdownEmitter::check_agent_note("first\n@owner: me").is_err());
    /// ```
    pub fn check_agent_note(note: &str) -> Result<(), String> {
        for (offset, line) in note.lines().enumerate().skip(1) {
            let line_number = offset + 1;
            if line.trim().is_empty() {
                return Err(format!(
                    "line {line_number} is blank, and blank lines are dropped when the note is read back"
                ));
            }
            if line.trim_start().starts_with('@') {
                return Err(format!(
                    "line {line_number} starts with '@', which would be read back as a separate annotation"
                ));
            }
        }
        Ok(())
    }

    /// Synthesize a task ID from the task title
    ///
    /// Thin wrapper over [`lash_types::task::synthesize_task_id`], which is
    /// also what the parser uses when a task carries no `@id:`. Sharing it is
    /// the point: an ID reported by `lash add` has to be the ID the index
    /// stores, or `lash show` and `@depends-on` cannot use it.
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
        lash_types::task::synthesize_task_id(title)
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

        let placement = PlacementInfo {
            anchor: InsertAnchor::Line(8), // Line 8 (after Task 1)
            order_index: 1,
            indent_level: 0,
        };
        let result = MarkdownEmitter::insert_into_existing(
            &file_path,
            task_line,
            &annotation_lines,
            &placement,
        );
        assert_eq!(result.unwrap(), 8);

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
            anchor: InsertAnchor::Line(1),
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
            anchor: InsertAnchor::Line(1),
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

    /// Resolve an anchor against `content`, returning the 0-indexed position.
    fn insert_index(content: &str, anchor: InsertAnchor) -> usize {
        let lines: Vec<&str> = content.lines().collect();
        MarkdownEmitter::resolve_insert_index(content, &lines, anchor)
    }

    #[test]
    fn test_end_of_tasks_section_lands_below_an_empty_heading() {
        // The bug: an existing file with an empty `## Tasks` section resolved
        // to index 0, prepending the task above the H1 where the parser never
        // saw it.
        let content = "# T\n\n@id: t\n\n## Tasks\n";
        assert_eq!(insert_index(content, InsertAnchor::EndOfTasksSection), 5);
    }

    #[test]
    fn test_end_of_tasks_section_stays_above_a_following_section() {
        let content = "# T\n\n@id: t\n\n## Tasks\n\n## Notes\n\nprose\n";
        // Line index 6 is `## Notes`, so the task goes just above it and keeps
        // the blank separator under the Tasks heading.
        assert_eq!(insert_index(content, InsertAnchor::EndOfTasksSection), 6);
    }

    #[test]
    fn test_end_of_tasks_section_follows_the_last_task_not_the_trailing_blanks() {
        let content = "# T\n\n## Tasks\n\n- [ ] One\n\n\n## Notes\n";
        assert_eq!(insert_index(content, InsertAnchor::EndOfTasksSection), 5);
    }

    #[test]
    fn test_end_of_tasks_section_ignores_a_heading_inside_a_code_fence() {
        let content = "# T\n\n## Tasks\n\n- [ ] One\n\n```\n## Notes\n```\n";
        // The fenced `## Notes` must not close the section, so the whole fence
        // counts as section content and the task appends after it.
        assert_eq!(insert_index(content, InsertAnchor::EndOfTasksSection), 9);
    }

    #[test]
    fn test_end_of_tasks_section_without_a_heading_appends_at_eof() {
        let content = "# T\n\nJust prose, no sections.\n";
        assert_eq!(insert_index(content, InsertAnchor::EndOfTasksSection), 3);
    }

    #[test]
    fn test_line_anchor_is_clamped_to_the_file_length() {
        let content = "# T\n\n## Tasks\n";
        assert_eq!(insert_index(content, InsertAnchor::Line(2)), 1);
        assert_eq!(insert_index(content, InsertAnchor::Line(99)), 3);
    }

    #[test]
    fn test_after_task_block_steps_over_the_previous_tasks_body() {
        // GitHub issue #48: the parser records the checkbox line and nothing
        // about the body under it, so the resolver anchors on line 6 — between
        // the task's title and its own body. The body must stay with its task.
        let content = "# T\n\n## Tasks\n\n- [x] Second task\n  Body prose.\n    1. A step.\n  Acceptance: still the second task's.\n";
        assert_eq!(insert_index(content, InsertAnchor::AfterTaskBlock(6)), 8);
    }

    #[test]
    fn test_after_task_block_keeps_a_body_split_into_paragraphs_whole() {
        // A blank line inside a body is not the end of the block: indented
        // content resumes after it.
        let content =
            "# T\n\n## Tasks\n\n- [x] One\n  First paragraph.\n\n  Second paragraph.\n\n## Notes\n";
        assert_eq!(insert_index(content, InsertAnchor::AfterTaskBlock(6)), 8);
    }

    #[test]
    fn test_after_task_block_stops_at_a_blank_line_that_ends_the_block() {
        // Nothing indented follows the blank, so the block really did end.
        let content = "# T\n\n## Tasks\n\n- [x] One\n  Body.\n\nUnindented prose.\n";
        assert_eq!(insert_index(content, InsertAnchor::AfterTaskBlock(6)), 6);
    }

    #[test]
    fn test_after_task_block_stops_at_the_next_checkbox() {
        // An indented checkbox is a child task, not body text.
        let content = "# T\n\n## Tasks\n\n- [x] One\n  - [ ] Child\n- [ ] Two\n";
        assert_eq!(insert_index(content, InsertAnchor::AfterTaskBlock(6)), 5);
    }

    #[test]
    fn test_after_task_block_leaves_bodyless_tasks_where_they_were() {
        // The common case must resolve exactly like `Line` did before.
        let content = "# T\n\n## Tasks\n\n- [ ] One\n- [ ] Two\n";
        assert_eq!(insert_index(content, InsertAnchor::AfterTaskBlock(6)), 5);
        assert_eq!(insert_index(content, InsertAnchor::AfterTaskBlock(7)), 6);
    }

    #[test]
    fn test_after_task_block_absorbs_indented_note_bullets() {
        // Contextual note bullets belong to the task above them.
        let content = "# T\n\n## Tasks\n\n- [x] One\n  - a note\n  - another note\n";
        assert_eq!(insert_index(content, InsertAnchor::AfterTaskBlock(6)), 7);
    }

    /// Insert `task_line` into `content` at `anchor`, returning the new file
    /// text and the 1-indexed line the task landed on.
    fn insert_at(content: &str, anchor: InsertAnchor, task_line: &str) -> (String, usize) {
        let temp_dir =
            std::env::temp_dir().join(format!("lash-test-insert-{:p}", content.as_ptr()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("tasks.md");
        fs::write(&file_path, content).unwrap();

        let placement = PlacementInfo {
            anchor,
            order_index: 0,
            indent_level: 0,
        };
        let line = MarkdownEmitter::insert_into_existing(&file_path, task_line, &[], &placement)
            .expect("insert must succeed");
        let updated = fs::read_to_string(&file_path).unwrap();
        let _ = fs::remove_dir_all(&temp_dir);
        (updated, line)
    }

    #[test]
    fn test_insert_after_a_body_separates_the_new_task_from_it() {
        // GitHub issue #48's expected output: the new task sits below the whole
        // block, with a blank line so it does not read as more of that prose.
        let content = "# T\n\n## Tasks\n\n- [x] One\n  Body prose.\n  Acceptance: mine.\n";
        let (updated, line) = insert_at(content, InsertAnchor::AfterTaskBlock(6), "- [ ] New task");

        assert_eq!(
            updated,
            "# T\n\n## Tasks\n\n- [x] One\n  Body prose.\n  Acceptance: mine.\n\n- [ ] New task\n"
        );
        assert_eq!(line, 9, "reported line must be where the task really is");
    }

    #[test]
    fn test_insert_after_a_bodyless_task_adds_no_separator() {
        let content = "# T\n\n## Tasks\n\n- [ ] One\n";
        let (updated, line) = insert_at(content, InsertAnchor::AfterTaskBlock(6), "- [ ] New task");

        assert_eq!(updated, "# T\n\n## Tasks\n\n- [ ] One\n- [ ] New task\n");
        assert_eq!(line, 6);
    }

    /// Emit `note` as annotation lines, then read it back the way the parser
    /// would when it encounters those lines under a task.
    fn round_trip_agent_note(indent: usize, note: &str) -> Option<String> {
        let lines = MarkdownEmitter::format_task_annotations(indent, None, &[], Some(note));
        let block = crate::parser::annotations::parse_annotation_block(
            lines.iter().map(String::as_str),
            None,
        )
        .expect("emitted annotation lines must parse");
        block.get_single("agent-note").map(str::to_string)
    }

    #[test]
    fn test_agent_note_round_trips_across_line_counts_and_indents() {
        let notes = [
            "single line",
            "first line\nsecond line",
            "first\nsecond\nthird\nfourth",
            "a note with: a colon\nand a second line",
            "trailing words end here",
        ];

        for indent in 0..4 {
            for note in notes {
                assert_eq!(
                    round_trip_agent_note(indent, note).as_deref(),
                    Some(note),
                    "note did not survive a round trip at indent {indent}: {note:?}"
                );
            }
        }
    }

    #[test]
    fn test_agent_note_continuation_lines_carry_the_annotation_indent() {
        let lines = MarkdownEmitter::format_task_annotations(1, None, &[], Some("first\nsecond"));
        assert_eq!(lines, ["    @agent-note: first", "    second"]);
    }

    #[test]
    fn test_check_agent_note_accepts_what_round_trips() {
        assert!(MarkdownEmitter::check_agent_note("one line").is_ok());
        assert!(MarkdownEmitter::check_agent_note("first\nsecond\nthird").is_ok());
        // An `@` on the *first* line is part of the value, not a new annotation.
        assert!(MarkdownEmitter::check_agent_note("ask @someone").is_ok());
    }

    #[test]
    fn test_check_agent_note_rejects_what_would_be_dropped() {
        // A blank continuation line is skipped by the parser, so the note
        // would come back with the gap closed up.
        let err = MarkdownEmitter::check_agent_note("first\n\nthird").unwrap_err();
        assert!(err.contains("line 2"), "unexpected reason: {err}");

        // A continuation starting with `@` is read back as its own annotation,
        // truncating the note.
        let err = MarkdownEmitter::check_agent_note("first\n@owner: me").unwrap_err();
        assert!(err.contains("line 2"), "unexpected reason: {err}");
    }
}
