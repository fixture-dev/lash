//! Placement resolution for new task insertion
//!
//! This module determines WHERE to insert new tasks within Markdown files,
//! computing line numbers, indentation levels, and order indices.

use lash_types::creation::{InsertPosition, TaskCreationRequest};
use lash_types::creation_errors::TaskCreationError;
use lash_types::file::TaskFile;
use lash_types::task::Task;

use super::validation::ValidationContext;

/// Information about where to insert a task in a file
///
/// Contains all the placement details needed by the emitter to write
/// the new task to the correct location with proper formatting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementInfo {
    /// Line number to insert at (1-indexed)
    ///
    /// Special value 0 means insert at beginning of new file.
    pub line_number: usize,

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
    /// use lash_core::creation::placement::{PlacementResolver, PlacementInfo};
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
    /// assert_eq!(placement.line_number, 0); // New file
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
        // Handle empty file case
        if ctx.resolved_file.tasks.is_empty() {
            return PlacementInfo {
                line_number: 0, // Signal for new file
                order_index: 0,
                indent_level: ctx.computed_depth as usize,
            };
        }

        let siblings = Self::get_siblings(&ctx.resolved_file, ctx.parent_task.as_ref());
        let order_index = siblings.len();

        // Compute line number: after last sibling's subtree
        let line_number = if let Some(last_sibling) = siblings.last() {
            Self::find_end_of_task_subtree(&ctx.resolved_file, last_sibling) + 1
        } else if let Some(parent) = &ctx.parent_task {
            // No siblings, insert right after parent (accounting for annotations)
            let parent_line = Self::get_task_line(&ctx.resolved_file, parent);
            let annotation_lines = Self::count_annotation_lines(parent);
            parent_line + annotation_lines + 1
        } else {
            // No siblings and no parent - append at end of tasks section
            Self::find_end_of_tasks_section(&ctx.resolved_file)
        };

        PlacementInfo {
            line_number,
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
            line_number,
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
            line_number,
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
            line_number,
            order_index,
            indent_level: ctx.computed_depth as usize,
        })
    }

    /// Find the end of the ## Tasks section (before next section or EOF)
    ///
    /// For now, we estimate based on the last task in the file.
    /// In a future enhancement, this could parse the actual markdown structure.
    fn find_end_of_tasks_section(file: &TaskFile) -> usize {
        if file.tasks.is_empty() {
            // Estimate: after file header (typically ~10 lines)
            return 15;
        }

        // Find the last top-level task and return line after its subtree
        let top_level_tasks: Vec<_> = file
            .tasks
            .tasks()
            .iter()
            .filter(|t| t.parent_id.is_none())
            .collect();

        if let Some(last_task) = top_level_tasks.last() {
            Self::find_end_of_task_subtree(file, last_task) + 1
        } else {
            // Fallback: estimate
            15
        }
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
    /// Task annotations like @depends-on and @agent-note appear on separate lines
    /// after the task checkbox line.
    fn count_annotation_lines(task: &Task) -> usize {
        let mut count = 0;

        // Each dependency gets its own line
        count += task.metadata.depends_on.len();

        // Agent note gets one line if present
        if task.metadata.agent_note.is_some() {
            count += 1;
        }

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

        assert_eq!(placement.line_number, 0); // New file signal
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
        assert!(placement.line_number > 0);
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
        assert!(placement.line_number > 0);
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

        // Test: append after Level 1-4 should go on line 37
        let request = TaskCreationRequestBuilder::new("Level 1-5").build();
        let ctx = validator.validate(&request, Some(&file)).unwrap();
        let placement = PlacementResolver::resolve(&ctx, &request).unwrap();
        assert_eq!(placement.line_number, 37);
        assert_eq!(placement.order_index, 4);

        // Test: insert after Level 1-3 should go on line 34
        let request = TaskCreationRequestBuilder::new("Level 1-3.5")
            .after("level-1-3")
            .build();
        let ctx = validator.validate(&request, Some(&file)).unwrap();
        let placement = PlacementResolver::resolve(&ctx, &request).unwrap();
        assert_eq!(placement.line_number, 34);
        assert_eq!(placement.order_index, 3);
    }
}
