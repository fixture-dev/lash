//! Placement resolution for new task insertion
//!
//! This module determines WHERE to insert new tasks within Markdown files,
//! computing line numbers, indentation levels, and order indices.

use lash_types::creation::{InsertPosition, TaskCreationRequest};
use lash_types::creation_errors::TaskCreationError;
use lash_types::file::TaskFile;
use lash_types::task::Task;

use super::validation::ValidationContext;

/// Where in the target file the new task goes
///
/// Most placements resolve to a concrete line, but appending to a file whose
/// `## Tasks` section holds no tasks yet cannot: a parsed
/// [`TaskFile`] records task line numbers and nothing about section
/// boundaries, so the line is only knowable from the source text the emitter
/// reads. [`Self::EndOfTasksSection`] carries that intent through to the
/// emitter rather than guessing a number here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertAnchor {
    /// A concrete 1-indexed source line; the task is inserted before it.
    Line(usize),

    /// After the last content line of the `## Tasks` section.
    ///
    /// Used when the section has no tasks to append after. If the file has no
    /// `## Tasks` heading at all, the emitter appends at the end of the file.
    EndOfTasksSection,
}

impl InsertAnchor {
    /// The 1-indexed line this anchor names, if it names one
    ///
    /// Returns `None` for [`Self::EndOfTasksSection`], which only the emitter
    /// can turn into a line.
    #[must_use]
    pub fn line(self) -> Option<usize> {
        match self {
            Self::Line(line_number) => Some(line_number),
            Self::EndOfTasksSection => None,
        }
    }
}

/// Information about where to insert a task in a file
///
/// Contains all the placement details needed by the emitter to write
/// the new task to the correct location with proper formatting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementInfo {
    /// Where to insert the task
    pub anchor: InsertAnchor,

    /// Order index among siblings (0-indexed)
    pub order_index: usize,

    /// Indentation level (depth * 2 spaces)
    pub indent_level: usize,
}

/// Resolves where to place new tasks in Markdown files
pub struct PlacementResolver;

impl PlacementResolver {
    /// Main entry point - resolves placement from validation context and request
    ///
    /// # Arguments
    ///
    /// * `ctx` - Validation context containing file and parent information
    /// * `request` - Task creation request with positioning details
    ///
    /// # Returns
    ///
    /// * `Ok(PlacementInfo)` - Placement details for the emitter
    /// * `Err(TaskCreationError)` - If placement cannot be resolved
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Referenced task (Before/After) is not found
    /// - Index is out of bounds
    /// - Position is invalid for the file state
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_core::creation::placement::{InsertAnchor, PlacementResolver, PlacementInfo};
    /// use lash_core::creation::validation::{TaskValidator, ValidationContext};
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
    /// // Validate first (for new file, file_content is None)
    /// let ctx = validator.validate(&request, None).unwrap();
    ///
    /// // Then resolve placement
    /// let placement = PlacementResolver::resolve(&ctx, &request).unwrap();
    /// // The file has no tasks, so the line is resolved from the source text
    /// // when the task is written.
    /// assert_eq!(placement.anchor, InsertAnchor::EndOfTasksSection);
    /// assert_eq!(placement.order_index, 0);
    /// assert_eq!(placement.indent_level, 0); // Top-level task
    /// ```
    pub fn resolve(
        ctx: &ValidationContext,
        request: &TaskCreationRequest,
    ) -> Result<PlacementInfo, TaskCreationError> {
        // Dispatch based on position type
        match &request.position {
            InsertPosition::Append => Ok(Self::resolve_append(ctx)),
            InsertPosition::AtIndex(index) => Self::resolve_at_index(ctx, *index),
            InsertPosition::Before(task_id) => Self::resolve_before(ctx, task_id),
            InsertPosition::After(task_id) => Self::resolve_after(ctx, task_id),
        }
    }

    /// Append after last sibling (under parent or top-level)
    ///
    /// Finds the last task at the same level and inserts after it.
    fn resolve_append(ctx: &ValidationContext) -> PlacementInfo {
        let siblings = Self::get_siblings(&ctx.resolved_file, ctx.parent_task.as_ref());
        let order_index = siblings.len();

        let anchor = if let Some(last_sibling) = siblings.last() {
            // After the last sibling's subtree
            InsertAnchor::Line(Self::find_end_of_task_subtree(&ctx.resolved_file, last_sibling) + 1)
        } else if let Some(parent) = &ctx.parent_task {
            // No siblings, insert right after parent (accounting for annotations)
            let parent_line = Self::get_task_line(&ctx.resolved_file, parent);
            let annotation_lines = Self::count_annotation_lines(parent);
            InsertAnchor::Line(parent_line + annotation_lines + 1)
        } else {
            // Nothing to append after: the file has no tasks at this level and
            // no parent to hang off. Only the source text says where the
            // section ends, so defer to the emitter.
            InsertAnchor::EndOfTasksSection
        };

        PlacementInfo {
            anchor,
            order_index,
            indent_level: ctx.computed_depth as usize,
        }
    }

    /// Insert at specific index among siblings
    fn resolve_at_index(
        ctx: &ValidationContext,
        index: usize,
    ) -> Result<PlacementInfo, TaskCreationError> {
        let siblings = Self::get_siblings(&ctx.resolved_file, ctx.parent_task.as_ref());

        // Allow index == siblings.len() for append position
        if index > siblings.len() {
            return Err(TaskCreationError::InvalidPosition {
                reason: format!("index {} is out of bounds (max {})", index, siblings.len()),
            });
        }

        // If index == siblings.len(), this is equivalent to append
        if index == siblings.len() {
            return Ok(Self::resolve_append(ctx));
        }

        // Insert before the task at this index
        let target = siblings[index];
        let line_number = Self::get_task_line(&ctx.resolved_file, target);

        Ok(PlacementInfo {
            anchor: InsertAnchor::Line(line_number),
            order_index: index,
            indent_level: ctx.computed_depth as usize,
        })
    }

    /// Insert before a specific task
    fn resolve_before(
        ctx: &ValidationContext,
        task_id: &str,
    ) -> Result<PlacementInfo, TaskCreationError> {
        let task = ctx.resolved_file.tasks.get_task(task_id).ok_or_else(|| {
            TaskCreationError::InvalidPosition {
                reason: format!("task '{task_id}' not found"),
            }
        })?;

        // Verify task is at the right level (sibling of new task)
        let siblings = Self::get_siblings(&ctx.resolved_file, ctx.parent_task.as_ref());
        let order_index = siblings
            .iter()
            .position(|t| t.id == task_id)
            .ok_or_else(|| TaskCreationError::InvalidPosition {
                reason: format!("task '{task_id}' is not a sibling at the target level"),
            })?;

        let line_number = Self::get_task_line(&ctx.resolved_file, task);

        Ok(PlacementInfo {
            anchor: InsertAnchor::Line(line_number),
            order_index,
            indent_level: ctx.computed_depth as usize,
        })
    }

    /// Insert after a specific task (and all its descendants)
    fn resolve_after(
        ctx: &ValidationContext,
        task_id: &str,
    ) -> Result<PlacementInfo, TaskCreationError> {
        let task = ctx.resolved_file.tasks.get_task(task_id).ok_or_else(|| {
            TaskCreationError::InvalidPosition {
                reason: format!("task '{task_id}' not found"),
            }
        })?;

        // Verify task is at the right level (sibling of new task)
        let siblings = Self::get_siblings(&ctx.resolved_file, ctx.parent_task.as_ref());
        let sibling_pos = siblings
            .iter()
            .position(|t| t.id == task_id)
            .ok_or_else(|| TaskCreationError::InvalidPosition {
                reason: format!("task '{task_id}' is not a sibling at the target level"),
            })?;

        // Order index is after this sibling
        let order_index = sibling_pos + 1;

        // Line number is after the task's entire subtree
        let line_number = Self::find_end_of_task_subtree(&ctx.resolved_file, task) + 1;

        Ok(PlacementInfo {
            anchor: InsertAnchor::Line(line_number),
            order_index,
            indent_level: ctx.computed_depth as usize,
        })
    }

    /// Find the end of a task's subtree (last descendant line number)
    ///
    /// Returns the line number where the task and all its descendants end.
    /// This accounts for annotation lines that may follow the last task.
    fn find_end_of_task_subtree(file: &TaskFile, task: &Task) -> usize {
        let descendants = file.tasks.get_descendants(&task.id);

        let last_task = if descendants.is_empty() {
            task
        } else {
            // Find the last descendant by line number
            descendants
                .iter()
                .max_by_key(|t| t.line_number)
                .unwrap_or(&task)
        };

        // Use actual line number, accounting for potential annotation lines
        // We add lines for any annotations that follow the task
        let annotation_lines = Self::count_annotation_lines(last_task);
        last_task.line_number + annotation_lines
    }

    /// Count the number of annotation lines that follow a task
    ///
    /// Every annotation type is written on its own line after the task
    /// checkbox line, so appending correctly requires knowing exactly how
    /// many trailing lines belong to the task being appended after —
    /// undercounting here inserts the new task in the middle of that
    /// annotation block, silently detaching a trailing annotation (e.g. an
    /// existing task's `@id:`) from its owner.
    ///
    /// An annotation value may span several source lines: the parser folds
    /// indented continuation lines into the value, joining them with `\n`
    /// (see `parser::annotations::parse_annotation_block`). Such a value
    /// therefore occupies `value.lines().count()` source lines, not one, and
    /// counting it as one splices the new task into the middle of the note —
    /// destroying the continuation lines on the next reindex.
    ///
    /// `@labels` is deliberately not counted: labels may be written inline
    /// on the checkbox line (`#backend`) or as a separate `@labels:` block,
    /// and [`lash_types::task::TaskMetadata`] doesn't record which form was
    /// used, so it can't be counted without re-reading the source line.
    /// `lash add` itself always emits labels inline (see
    /// `MarkdownEmitter::format_task_line`), so this only affects appending
    /// after a hand-edited task that used the block form.
    fn count_annotation_lines(task: &Task) -> usize {
        /// Source lines occupied by a single annotation value.
        ///
        /// Always at least 1 (the `@key:` line itself), plus one per folded
        /// continuation line.
        fn value_lines(value: &str) -> usize {
            value.lines().count().max(1)
        }

        let mut count = 0;

        // @id: one line, only when it's an explicit annotation (not a
        // synthesized id, which isn't written to Markdown at all).
        if task.has_explicit_id {
            count += 1;
        }

        // @owner / @estimate / @agent-note: one line each, plus any folded
        // continuation lines.
        if let Some(owner) = &task.metadata.owner {
            count += value_lines(owner);
        }
        if let Some(estimate) = &task.metadata.estimate {
            count += value_lines(estimate);
        }
        if let Some(note) = &task.metadata.agent_note {
            count += value_lines(note);
        }

        // @depends-on / @doc: one line per reference, as emitted by
        // `MarkdownEmitter::format_task_annotations`. A hand-written
        // comma-separated `@depends-on: a, b, c` parses to several references
        // from a single line and is over-counted here; that is the same
        // inline-vs-block ambiguity described above for `@labels`, and
        // over-counting appends after the block rather than inside it.
        count += task.metadata.depends_on.len();
        count += task.metadata.docs.len();

        // Custom annotations: one line per key, plus folded continuations.
        count += task
            .metadata
            .custom
            .values()
            .map(|v| value_lines(v))
            .sum::<usize>();

        count
    }

    /// Get a task's actual line number
    ///
    /// Uses the line number stored during parsing. Falls back to estimation
    /// only if `line_number` is 0 (unknown).
    fn get_task_line(file: &TaskFile, task: &Task) -> usize {
        if task.line_number > 0 {
            task.line_number
        } else {
            // Fallback for tasks without stored line numbers
            Self::estimate_task_line(file, task)
        }
    }

    /// Estimate a task's line number based on its position in the tree
    ///
    /// This is a fallback for when line numbers are not available.
    /// Used only for backwards compatibility with older parsed files.
    fn estimate_task_line(file: &TaskFile, task: &Task) -> usize {
        const HEADER_LINES: usize = 10;
        const TASKS_SECTION_HEADER: usize = 2;

        // Count all tasks that come before this one (in document order)
        let tasks_before = file
            .tasks
            .tasks()
            .iter()
            .take_while(|t| t.id != task.id)
            .count();

        HEADER_LINES + TASKS_SECTION_HEADER + tasks_before + 1
    }

    /// Get siblings at a given depth under a parent
    ///
    /// Returns all tasks that are direct children of the parent (or top-level
    /// if parent is None), in order.
    fn get_siblings<'a>(file: &'a TaskFile, parent: Option<&Task>) -> Vec<&'a Task> {
        if let Some(p) = parent {
            // Get children of parent
            let mut children = file.tasks.get_children(&p.id);
            // Sort by order_index to ensure correct ordering
            children.sort_by_key(|t| t.order_index);
            children
        } else {
            // Get top-level tasks
            let mut top_level: Vec<_> = file
                .tasks
                .tasks()
                .iter()
                .filter(|t| t.parent_id.is_none())
                .collect();
            top_level.sort_by_key(|t| t.order_index);
            top_level
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
    use std::path::PathBuf;
    use std::time::SystemTime;

    use crate::creation::validation::TaskValidator;

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
    fn test_append_to_empty_file() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let request = TaskCreationRequestBuilder::new("New task")
            .file_path(PathBuf::from("test.md"))
            .build();

        let ctx = validator.validate(&request, None).unwrap();
        let placement = PlacementResolver::resolve(&ctx, &request).unwrap();

        // No tasks to append after and no `## Tasks` line numbers to work
        // from, so the emitter resolves the line from the source text.
        assert_eq!(placement.anchor, InsertAnchor::EndOfTasksSection);
        assert_eq!(placement.order_index, 0);
        assert_eq!(placement.indent_level, 0); // Top-level
    }

    #[test]
    fn test_append_as_top_level_task() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let mut tasks = TaskTree::new();
        tasks
            .add_task(TaskBuilder::new("Task 1").id("task-1").build().unwrap())
            .unwrap();
        tasks
            .add_task(TaskBuilder::new("Task 2").id("task-2").build().unwrap())
            .unwrap();
        let file = create_test_file(tasks);

        let request = TaskCreationRequestBuilder::new("New task").build();

        let ctx = validator.validate(&request, Some(&file)).unwrap();
        let placement = PlacementResolver::resolve(&ctx, &request).unwrap();

        assert_eq!(placement.order_index, 2); // After 2 existing tasks
        assert_eq!(placement.indent_level, 0); // Top-level
        assert!(placement.anchor.line().unwrap() > 0);
    }

    #[test]
    fn test_append_as_child_of_parent() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let mut tasks = TaskTree::new();
        tasks
            .add_task(
                TaskBuilder::new("Parent")
                    .id("parent")
                    .depth(0)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        tasks
            .add_task(
                TaskBuilder::new("Child 1")
                    .id("child-1")
                    .parent("parent")
                    .depth(1)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let file = create_test_file(tasks);

        let request = TaskCreationRequestBuilder::new("Child 2")
            .parent_id("parent")
            .build();

        let ctx = validator.validate(&request, Some(&file)).unwrap();
        let placement = PlacementResolver::resolve(&ctx, &request).unwrap();

        assert_eq!(placement.order_index, 1); // After 1 existing child
        assert_eq!(placement.indent_level, 1); // Depth 1
        assert!(placement.anchor.line().unwrap() > 0);
    }

    #[test]
    fn test_append_as_first_child_of_parent() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let mut tasks = TaskTree::new();
        tasks
            .add_task(
                TaskBuilder::new("Parent")
                    .id("parent")
                    .depth(0)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let file = create_test_file(tasks);

        let request = TaskCreationRequestBuilder::new("Child 1")
            .parent_id("parent")
            .build();

        let ctx = validator.validate(&request, Some(&file)).unwrap();
        let placement = PlacementResolver::resolve(&ctx, &request).unwrap();

        assert_eq!(placement.order_index, 0); // First child
        assert_eq!(placement.indent_level, 1); // Depth 1
    }

    #[test]
    fn test_insert_before_specific_task() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let mut tasks = TaskTree::new();
        tasks
            .add_task(
                TaskBuilder::new("Task 1")
                    .id("task-1")
                    .order_index(0)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        tasks
            .add_task(
                TaskBuilder::new("Task 2")
                    .id("task-2")
                    .order_index(1)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let file = create_test_file(tasks);

        let request = TaskCreationRequestBuilder::new("New task")
            .before("task-2")
            .build();

        let ctx = validator.validate(&request, Some(&file)).unwrap();
        let placement = PlacementResolver::resolve(&ctx, &request).unwrap();

        assert_eq!(placement.order_index, 1); // Insert at index 1 (before task-2)
        assert_eq!(placement.indent_level, 0);
    }

    #[test]
    fn test_insert_after_task_with_children() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let mut tasks = TaskTree::new();
        tasks
            .add_task(
                TaskBuilder::new("Task 1")
                    .id("task-1")
                    .order_index(0)
                    .depth(0)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        tasks
            .add_task(
                TaskBuilder::new("Child 1")
                    .id("child-1")
                    .parent("task-1")
                    .order_index(0)
                    .depth(1)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        tasks
            .add_task(
                TaskBuilder::new("Task 2")
                    .id("task-2")
                    .order_index(1)
                    .depth(0)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let file = create_test_file(tasks);

        let request = TaskCreationRequestBuilder::new("New task")
            .after("task-1")
            .build();

        let ctx = validator.validate(&request, Some(&file)).unwrap();
        let placement = PlacementResolver::resolve(&ctx, &request).unwrap();

        assert_eq!(placement.order_index, 1); // After task-1 (before task-2)
        assert_eq!(placement.indent_level, 0);
        // Line number should be after child-1
    }

    #[test]
    fn test_insert_at_specific_index() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let mut tasks = TaskTree::new();
        tasks
            .add_task(
                TaskBuilder::new("Task 1")
                    .id("task-1")
                    .order_index(0)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        tasks
            .add_task(
                TaskBuilder::new("Task 2")
                    .id("task-2")
                    .order_index(1)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let file = create_test_file(tasks);

        let request = TaskCreationRequestBuilder::new("New task")
            .at_index(1)
            .build();

        let ctx = validator.validate(&request, Some(&file)).unwrap();
        let placement = PlacementResolver::resolve(&ctx, &request).unwrap();

        assert_eq!(placement.order_index, 1);
        assert_eq!(placement.indent_level, 0);
    }

    #[test]
    fn test_insert_at_index_append_position() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let mut tasks = TaskTree::new();
        tasks
            .add_task(TaskBuilder::new("Task 1").id("task-1").build().unwrap())
            .unwrap();
        let file = create_test_file(tasks);

        let request = TaskCreationRequestBuilder::new("New task")
            .at_index(1)
            .build();

        let ctx = validator.validate(&request, Some(&file)).unwrap();
        let placement = PlacementResolver::resolve(&ctx, &request).unwrap();

        assert_eq!(placement.order_index, 1); // Append position
    }

    #[test]
    fn test_insert_at_index_out_of_bounds() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let mut tasks = TaskTree::new();
        tasks
            .add_task(TaskBuilder::new("Task 1").id("task-1").build().unwrap())
            .unwrap();
        let file = create_test_file(tasks);

        let request = TaskCreationRequestBuilder::new("New task")
            .at_index(5)
            .build();

        let ctx = validator.validate(&request, Some(&file)).unwrap();
        let result = PlacementResolver::resolve(&ctx, &request);

        assert!(result.is_err());
        match result.unwrap_err() {
            TaskCreationError::InvalidPosition { reason } => {
                assert!(reason.contains("out of bounds"));
            }
            _ => panic!("Expected InvalidPosition error"),
        }
    }

    #[test]
    fn test_insert_before_task_not_found() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let tasks = TaskTree::new();
        let file = create_test_file(tasks);

        let request = TaskCreationRequestBuilder::new("New task")
            .before("nonexistent")
            .build();

        let ctx = validator.validate(&request, Some(&file)).unwrap();
        let result = PlacementResolver::resolve(&ctx, &request);

        assert!(result.is_err());
        match result.unwrap_err() {
            TaskCreationError::InvalidPosition { reason } => {
                assert!(reason.contains("not found"));
            }
            _ => panic!("Expected InvalidPosition error"),
        }
    }

    #[test]
    fn test_insert_after_task_not_found() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let tasks = TaskTree::new();
        let file = create_test_file(tasks);

        let request = TaskCreationRequestBuilder::new("New task")
            .after("nonexistent")
            .build();

        let ctx = validator.validate(&request, Some(&file)).unwrap();
        let result = PlacementResolver::resolve(&ctx, &request);

        assert!(result.is_err());
        match result.unwrap_err() {
            TaskCreationError::InvalidPosition { reason } => {
                assert!(reason.contains("not found"));
            }
            _ => panic!("Expected InvalidPosition error"),
        }
    }

    #[test]
    fn test_insert_before_wrong_level() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let mut tasks = TaskTree::new();
        tasks
            .add_task(
                TaskBuilder::new("Parent")
                    .id("parent")
                    .depth(0)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        tasks
            .add_task(
                TaskBuilder::new("Child")
                    .id("child")
                    .parent("parent")
                    .depth(1)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let file = create_test_file(tasks);

        // Try to insert before "child" as a top-level task
        let request = TaskCreationRequestBuilder::new("New task")
            .before("child")
            .build();

        let ctx = validator.validate(&request, Some(&file)).unwrap();
        let result = PlacementResolver::resolve(&ctx, &request);

        assert!(result.is_err());
        match result.unwrap_err() {
            TaskCreationError::InvalidPosition { reason } => {
                assert!(reason.contains("not a sibling"));
            }
            _ => panic!("Expected InvalidPosition error"),
        }
    }

    #[test]
    fn test_indent_level_calculation() {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let mut tasks = TaskTree::new();
        tasks
            .add_task(
                TaskBuilder::new("Level 0")
                    .id("level-0")
                    .depth(0)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        tasks
            .add_task(
                TaskBuilder::new("Level 1")
                    .id("level-1")
                    .parent("level-0")
                    .depth(1)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        tasks
            .add_task(
                TaskBuilder::new("Level 2")
                    .id("level-2")
                    .parent("level-1")
                    .depth(2)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let file = create_test_file(tasks);

        // Test depth 0
        let request = TaskCreationRequestBuilder::new("New task").build();
        let ctx = validator.validate(&request, Some(&file)).unwrap();
        let placement = PlacementResolver::resolve(&ctx, &request).unwrap();
        assert_eq!(placement.indent_level, 0);

        // Test depth 1
        let request = TaskCreationRequestBuilder::new("New task")
            .parent_id("level-0")
            .build();
        let ctx = validator.validate(&request, Some(&file)).unwrap();
        let placement = PlacementResolver::resolve(&ctx, &request).unwrap();
        assert_eq!(placement.indent_level, 1);

        // Test depth 2
        let request = TaskCreationRequestBuilder::new("New task")
            .parent_id("level-1")
            .build();
        let ctx = validator.validate(&request, Some(&file)).unwrap();
        let placement = PlacementResolver::resolve(&ctx, &request).unwrap();
        assert_eq!(placement.indent_level, 2);

        // Test depth 3
        let request = TaskCreationRequestBuilder::new("New task")
            .parent_id("level-2")
            .build();
        let ctx = validator.validate(&request, Some(&file)).unwrap();
        let placement = PlacementResolver::resolve(&ctx, &request).unwrap();
        assert_eq!(placement.indent_level, 3);
    }

    /// Helper to add children to a parent task in the test
    fn add_children(tasks: &mut TaskTree, parent_id: &str, start_line: usize, count: usize) {
        for i in 0..count {
            tasks
                .add_task(
                    TaskBuilder::new(format!("Subtask {}", i + 1))
                        .id(format!("{parent_id}-child-{}", i + 1))
                        .parent(parent_id)
                        .depth(1)
                        .order_index(i)
                        .line_number(start_line + i)
                        .build()
                        .unwrap(),
                )
                .unwrap();
        }
    }

    #[test]
    fn test_multiline_task_placement_uses_line_numbers() {
        // Regression test: tasks were placed incorrectly because line numbers were
        // estimated (1 line per task) rather than using actual line numbers
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let mut tasks = TaskTree::new();
        // Structure: Level 1-1 (line 17, 4 children), Level 1-2 (line 23, 4 children),
        // Level 1-3 (line 29, 4 children), Level 1-4 (line 35, 1 child)

        // Level 1-1 with 4 children (lines 17-21)
        tasks
            .add_task(
                TaskBuilder::new("Level 1-1")
                    .id("level-1-1")
                    .order_index(0)
                    .line_number(17)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        add_children(&mut tasks, "level-1-1", 18, 4);

        // Level 1-2 with 4 children (lines 23-27)
        tasks
            .add_task(
                TaskBuilder::new("Level 1-2")
                    .id("level-1-2")
                    .order_index(1)
                    .line_number(23)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        add_children(&mut tasks, "level-1-2", 24, 4);

        // Level 1-3 with 4 children (lines 29-33)
        tasks
            .add_task(
                TaskBuilder::new("Level 1-3")
                    .id("level-1-3")
                    .order_index(2)
                    .line_number(29)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        add_children(&mut tasks, "level-1-3", 30, 4);

        // Level 1-4 with 1 child (lines 35-36)
        tasks
            .add_task(
                TaskBuilder::new("Level 1-4")
                    .id("level-1-4")
                    .order_index(3)
                    .line_number(35)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        add_children(&mut tasks, "level-1-4", 36, 1);

        let file = create_test_file(tasks);

        // Every task in this fixture is built via `.id(...)`, so
        // `has_explicit_id` is true throughout and `count_annotation_lines`
        // correctly attributes one extra `@id:` line to each — hence the
        // "+1" past the last task's own line in both expectations below.

        // Test: append after Level 1-4 should go on line 38 (37 = child's
        // line + its @id: line, +1 to move past it).
        let request = TaskCreationRequestBuilder::new("Level 1-5").build();
        let ctx = validator.validate(&request, Some(&file)).unwrap();
        let placement = PlacementResolver::resolve(&ctx, &request).unwrap();
        assert_eq!(placement.anchor.line(), Some(38));
        assert_eq!(placement.order_index, 4);

        // Test: insert after Level 1-3 should go on line 35 (34 = last
        // child's line + its @id: line, +1 to move past it).
        let request = TaskCreationRequestBuilder::new("Level 1-3.5")
            .after("level-1-3")
            .build();
        let ctx = validator.validate(&request, Some(&file)).unwrap();
        let placement = PlacementResolver::resolve(&ctx, &request).unwrap();
        assert_eq!(placement.anchor.line(), Some(35));
        assert_eq!(placement.order_index, 3);
    }

    #[test]
    fn test_append_after_task_with_explicit_id_skips_its_annotation_line() {
        // Regression test: appending after a task that carries an explicit
        // `@id:` annotation used to insert one line too early, landing
        // *between* the existing task's checkbox line and its own `@id:`
        // line and detaching the annotation from its owner. This is the
        // scenario `lash add --id` (GitHub issue #24) hits constantly, since
        // essentially every well-formed task carries an `@id:`.
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);

        let mut tasks = TaskTree::new();
        tasks
            .add_task(
                TaskBuilder::new("Existing task")
                    .id("existing") // sets has_explicit_id = true
                    .order_index(0)
                    .line_number(6) // checkbox line
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let file = create_test_file(tasks);

        let request = TaskCreationRequestBuilder::new("New task").build();
        let ctx = validator.validate(&request, Some(&file)).unwrap();
        let placement = PlacementResolver::resolve(&ctx, &request).unwrap();

        // Checkbox on line 6, `@id:` annotation on line 7 -> insert at line 8,
        // not line 7 (which would land on top of the `@id:` line).
        assert_eq!(placement.anchor.line(), Some(8));
        assert_eq!(placement.order_index, 1);
    }

    /// Build a single-task file whose one task carries `note` as its
    /// `@agent-note`, with the checkbox on line 6.
    fn file_with_agent_note(note: &str) -> lash_types::TaskFile {
        let mut task = TaskBuilder::new("Existing task")
            .id("existing")
            .order_index(0)
            .line_number(6)
            .build()
            .unwrap();
        task.metadata.agent_note = Some(note.to_string());

        let mut tasks = TaskTree::new();
        tasks.add_task(task).unwrap();
        create_test_file(tasks)
    }

    fn append_line_for(file: &lash_types::TaskFile) -> usize {
        let config = ConfigBuilder::new().build().unwrap();
        let validator = TaskValidator::new(config);
        let request = TaskCreationRequestBuilder::new("New task").build();
        let ctx = validator.validate(&request, Some(file)).unwrap();
        PlacementResolver::resolve(&ctx, &request)
            .unwrap()
            .anchor
            .line()
            .expect("appending after an existing task resolves to a line")
    }

    #[test]
    fn test_append_after_task_with_multiline_agent_note_clears_whole_note() {
        // Regression test: a multi-line `@agent-note` was counted as a single
        // line, so appending landed *between* the note's first line and its
        // continuation lines. The orphaned continuations were then destroyed
        // on reindex — silent data loss, exit code 0.
        //
        // The parser folds indented continuation lines into the value joined
        // by '\n' (parser::annotations::parse_annotation_block), so a value of
        // "a\nb\nc" occupies three source lines.
        //
        // Layout: checkbox 6, @id: 7, @agent-note: 8, continuations 9 and 10
        // -> append at 11.
        let file = file_with_agent_note("line one\nline two\nline three");
        assert_eq!(append_line_for(&file), 11);
    }

    #[test]
    fn test_append_after_task_with_single_line_agent_note_is_unchanged() {
        // Guards the fix against over-correcting: a one-line note must still
        // count as exactly one line. Checkbox 6, @id: 7, @agent-note: 8
        // -> append at 9.
        let file = file_with_agent_note("just one line");
        assert_eq!(append_line_for(&file), 9);
    }

    #[test]
    fn test_append_after_task_with_empty_agent_note_counts_one_line() {
        // An empty value still occupies its own `@agent-note:` line, but
        // "".lines().count() is 0 — hence the .max(1) floor. Without it this
        // would append at 8, landing on top of the annotation.
        let file = file_with_agent_note("");
        assert_eq!(append_line_for(&file), 9);
    }

    #[test]
    fn test_append_after_task_with_multiline_owner_and_estimate() {
        // The same folding applies to every single-value annotation, not just
        // `@agent-note`. Checkbox 6, @id: 7, @owner: 8-9, @estimate: 10-11
        // -> append at 12.
        let mut task = TaskBuilder::new("Existing task")
            .id("existing")
            .order_index(0)
            .line_number(6)
            .build()
            .unwrap();
        task.metadata.owner = Some("alice\ncontinued".to_string());
        task.metadata.estimate = Some("3d\ncontinued".to_string());

        let mut tasks = TaskTree::new();
        tasks.add_task(task).unwrap();
        let file = create_test_file(tasks);

        assert_eq!(append_line_for(&file), 12);
    }
}
