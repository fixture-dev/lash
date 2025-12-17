//! Task tree builder
//!
//! This module constructs hierarchical task trees from flat lists of checkbox
//! lines. It handles:
//! - Parent-child relationship inference from indentation
//! - Depth limit validation
//! - Indentation consistency checking
//! - ID uniqueness validation
//! - Synthetic ID generation for tasks without explicit @id
//!
//! # Algorithm
//!
//! The builder uses a stack-based algorithm to track the current parent
//! at each nesting level:
//!
//! 1. Maintain a stack of "current parent at each depth"
//! 2. For each checkbox line:
//!    - Compute depth from indentation (depth = indent / 2)
//!    - Validate depth doesn't exceed max
//!    - Validate depth doesn't skip levels (e.g., 0 -> 2)
//!    - Pop stack to current depth
//!    - Set parent as top of stack
//!    - Create task and push onto stack
//! 3. After all lines processed, build final task tree
//!
//! This approach ensures:
//! - O(n) time complexity (single pass)
//! - Correct parent-child relationships
//! - Clear error messages for malformed hierarchies

use super::checkbox::{CheckboxLine, PlainBulletLine};
use lash_types::{ContextualNote, Task, TaskTree};
use std::collections::HashMap;

/// Builder for constructing task trees from checkbox lines
///
/// This builder maintains state during tree construction and provides
/// methods for adding checkbox lines and validating the resulting structure.
#[derive(Debug)]
#[allow(dead_code)] // Fields will be used in Task #5
pub struct TaskTreeBuilder {
    /// Tasks being built (flat list)
    tasks: Vec<Task>,

    /// Stack of parent indices at each depth level
    /// Index 0 = root level, index 1 = depth 1, etc.
    parent_stack: Vec<Option<usize>>,

    /// Maximum allowed depth (from config)
    max_depth: u8,

    /// Set of used task IDs (for duplicate detection)
    used_ids: std::collections::HashSet<String>,
}

impl TaskTreeBuilder {
    /// Create a new task tree builder
    ///
    /// # Arguments
    ///
    /// * `max_depth` - Maximum allowed task nesting depth (typically 2 for 3 levels)
    #[must_use]
    pub fn new(max_depth: u8) -> Self {
        Self {
            tasks: Vec::new(),
            parent_stack: vec![None],
            max_depth,
            used_ids: std::collections::HashSet::new(),
        }
    }

    /// Add a checkbox line to the tree
    ///
    /// This method processes a single checkbox line, validates it, and adds
    /// it to the tree being built. It returns an error if the line violates
    /// any constraints (depth limit, indentation consistency, etc.).
    ///
    /// # Arguments
    ///
    /// * `line` - The checkbox line to add
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the line was added successfully, or an error
    /// describing the problem.
    ///
    /// # Errors
    ///
    /// Returns errors for:
    /// - Depth limit violations
    /// - Skipped indentation levels
    /// - Duplicate task IDs
    ///
    /// # Panics
    ///
    /// Panics if called when tasks list is empty (internal invariant violation).
    pub fn add_line(&mut self, line: &CheckboxLine) -> Result<(), String> {
        let depth = line.depth;

        // Validate depth doesn't exceed maximum
        self.validate_depth(depth)?;

        // Validate indentation doesn't skip levels
        if !self.tasks.is_empty() {
            let previous_depth = self.tasks.last().unwrap().depth;
            Self::validate_indentation_jump(depth, previous_depth)?;
        }

        // Validate indentation is even (multiple of 2 spaces)
        if !line.has_valid_indentation() {
            return Err(format!(
                "Invalid indentation at line {}: {} spaces (must be multiple of 2)",
                line.line_num, line.indent
            ));
        }

        // Determine parent based on depth
        // The parent stack maintains the task index at each depth level
        // Stack[0] = root task at depth 0 (or None if no tasks yet)
        // Stack[1] = most recent task at depth 1
        // Stack[2] = most recent task at depth 2
        // etc.

        // Truncate stack to current depth (removing deeper levels)
        // After truncation, stack.len() should be depth
        self.parent_stack.truncate((depth as usize) + 1);

        // Get parent ID from stack (if depth > 0)
        let parent_id = if depth > 0 {
            // Parent is at depth - 1
            self.parent_stack
                .get((depth - 1) as usize)
                .and_then(|&idx| idx.map(|i| self.tasks[i].id.clone()))
        } else {
            None
        };

        // Compute order index among siblings
        // Count how many tasks have the same parent
        let order_index = self
            .tasks
            .iter()
            .filter(|t| t.parent_id == parent_id)
            .count();

        // Generate task ID (use explicit @id from annotation if provided)
        let task_index = self.tasks.len();
        let explicit_id = line
            .annotations
            .as_ref()
            .and_then(|a| a.get_single("id"))
            .map(String::from);

        let has_explicit_id = explicit_id.is_some();
        let task_id = if let Some(id) = explicit_id {
            id
        } else {
            self.generate_synthetic_id(&line.title, task_index)
        };

        // Check for duplicate ID
        if self.is_id_used(&task_id) {
            return Err(format!(
                "Duplicate task ID '{}' at line {}",
                task_id, line.line_num
            ));
        }

        // Mark ID as used
        self.mark_id_used(task_id.clone());

        // Extract metadata from annotation block (if present)
        let metadata = if let Some(ref annotations) = line.annotations {
            // Start with labels from both inline and annotation block
            let mut labels: Vec<String> = line.labels.iter().map(|l| l.name.clone()).collect();

            // Add labels from annotation block
            let annotation_labels = annotations.get_labels("labels");
            for label in annotation_labels {
                if !labels.contains(&label.name) {
                    labels.push(label.name);
                }
            }

            // Extract other metadata
            let owner = annotations.get_single("owner").map(String::from);
            let estimate = annotations.get_single("estimate").map(String::from);
            let agent_note = annotations.get_single("agent-note").map(String::from);

            // Extract dependencies
            let depends_on = annotations.get_dependencies().unwrap_or_default();

            // Extract doc references
            let docs = annotations.get_docs().unwrap_or_default();

            lash_types::TaskMetadata {
                labels,
                owner,
                estimate,
                depends_on,
                docs,
                agent_note,
                custom: HashMap::default(),
            }
        } else {
            // No annotation block - just use inline labels
            lash_types::TaskMetadata {
                labels: line.labels.iter().map(|l| l.name.clone()).collect(),
                ..Default::default()
            }
        };

        // Create the task
        let task = Task {
            id: task_id,
            has_explicit_id,
            title: line.title.clone(),
            status: line.status,
            depth,
            parent_id,
            order_index,
            line_number: line.line_num,
            metadata,
            body: None,
            contextual_notes: Vec::new(),
        };

        // Add task to the flat list
        let task_idx = self.tasks.len();
        self.tasks.push(task);

        // Update parent stack: set this task as the most recent at this depth
        // Ensure stack is long enough
        while self.parent_stack.len() <= (depth as usize) {
            self.parent_stack.push(None);
        }
        self.parent_stack[depth as usize] = Some(task_idx);

        Ok(())
    }

    /// Add a contextual note to the most recent parent task
    ///
    /// Contextual notes are plain bullet points that provide additional context
    /// for tasks. They are attached to the task that is at depth - 1 relative
    /// to the note's depth.
    ///
    /// # Arguments
    ///
    /// * `note` - The plain bullet line to add as a contextual note
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the note was added successfully, or an error
    /// describing the problem.
    ///
    /// # Errors
    ///
    /// Returns errors for:
    /// - No parent task to attach the note to
    /// - Invalid indentation
    pub fn add_note(&mut self, note: &PlainBulletLine) -> Result<(), String> {
        let depth = note.depth;

        // Validate indentation is even (multiple of 2 spaces)
        if !note.has_valid_indentation() {
            return Err(format!(
                "Invalid note indentation at line {}: {} spaces (must be multiple of 2)",
                note.line_num, note.indent
            ));
        }

        // Notes must be children of a task, so depth must be > 0
        // unless there are no tasks yet, which is an error
        if self.tasks.is_empty() {
            return Err(format!(
                "Orphaned note at line {}: no parent task to attach to",
                note.line_num
            ));
        }

        // Find the parent task for this note
        // The parent should be at depth - 1
        let parent_depth = if depth > 0 { depth - 1 } else { 0 };

        // Look in the parent stack for the task at parent_depth
        let parent_idx = if parent_depth as usize >= self.parent_stack.len() {
            // Stack doesn't go that deep, use the deepest available
            self.parent_stack.last().and_then(|&idx| idx)
        } else {
            self.parent_stack[parent_depth as usize]
        };

        // If there's no parent at the expected depth, find the most recent task
        // that could be a parent (at any depth <= note.depth - 1)
        let parent_idx = match parent_idx {
            Some(idx) => Some(idx),
            None => {
                // Fall back to finding any task that could be a parent
                // (depth < note.depth)
                self.tasks
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|(_, t)| t.depth < depth)
                    .map(|(i, _)| i)
            }
        };

        if let Some(idx) = parent_idx {
            self.tasks[idx]
                .contextual_notes
                .push(ContextualNote::new(note.text.clone(), note.line_num));
            Ok(())
        } else {
            // If note is at depth 0, attach to the most recent task at depth 0
            if depth == 0 {
                if let Some(last_task) = self.tasks.last_mut() {
                    if last_task.depth == 0 {
                        last_task
                            .contextual_notes
                            .push(ContextualNote::new(note.text.clone(), note.line_num));
                        return Ok(());
                    }
                }
            }
            Err(format!(
                "Orphaned note at line {}: no parent task at depth {} to attach to",
                note.line_num, parent_depth
            ))
        }
    }

    /// Add an orphaned annotation to the most recent applicable task
    ///
    /// Orphaned annotations are annotation lines that appear after contextual notes
    /// within a task. They should be merged into the most recent task at the
    /// appropriate depth.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - There are no tasks to attach the annotation to
    /// - The line doesn't start with '@'
    /// - The annotation cannot be parsed
    #[allow(clippy::missing_panics_doc)] // Can't panic: tasks.is_empty() checked first
    pub fn add_orphaned_annotation(
        &mut self,
        line: &str,
        line_num: usize,
        config: &lash_types::LashConfig,
    ) -> Result<(), String> {
        if self.tasks.is_empty() {
            return Err(format!(
                "Orphaned annotation at line {line_num}: no task to attach to"
            ));
        }

        // Parse the annotation line
        let trimmed = line.trim();
        if !trimmed.starts_with('@') {
            return Err(format!(
                "Invalid annotation at line {line_num}: must start with '@'"
            ));
        }

        // Parse the annotation to extract key and value
        let annotation_lines = vec![line];
        let block = match super::annotations::parse_annotation_block(
            annotation_lines.into_iter(),
            Some(config),
        ) {
            Ok(block) => block,
            Err(e) => {
                return Err(format!(
                    "Failed to parse annotation at line {line_num}: {e}"
                ))
            }
        };

        // Find the most recent task to attach this annotation to
        // We use the most recently added task as the target
        // Safety: We already checked that self.tasks is not empty above
        let task = self
            .tasks
            .last_mut()
            .expect("tasks checked non-empty above");

        // Merge the parsed annotation into the task's metadata
        // Handle @doc annotations specifically
        let docs = block.get_docs().unwrap_or_default();
        for doc_ref in docs {
            if !task.metadata.docs.contains(&doc_ref) {
                task.metadata.docs.push(doc_ref);
            }
        }

        // Handle other annotation types
        if let Some(owner) = block.get_single("owner") {
            task.metadata.owner = Some(owner.to_string());
        }
        if let Some(estimate) = block.get_single("estimate") {
            task.metadata.estimate = Some(estimate.to_string());
        }
        if let Some(agent_note) = block.get_single("agent-note") {
            task.metadata.agent_note = Some(agent_note.to_string());
        }

        // Handle dependencies
        let deps = block.get_dependencies().unwrap_or_default();
        for dep in deps {
            if !task.metadata.depends_on.contains(&dep) {
                task.metadata.depends_on.push(dep);
            }
        }

        // Handle labels
        for label in block.get_labels("labels") {
            if !task.metadata.labels.contains(&label.name) {
                task.metadata.labels.push(label.name);
            }
        }

        Ok(())
    }

    /// Build the final task tree
    ///
    /// Converts the flat list of tasks with parent indices into a proper
    /// `TaskTree` structure. This method applies auto-waiving logic: if a
    /// parent task is waived, all its descendants are also marked as waived.
    ///
    /// # Returns
    ///
    /// Returns the constructed task tree.
    ///
    /// # Panics
    ///
    /// Panics if the task tree construction fails, which indicates a bug in
    /// the builder (duplicate IDs or invalid parent references that should
    /// have been caught by `add_line`).
    #[must_use]
    pub fn build(mut self) -> TaskTree {
        // Apply auto-waiving logic
        // If parent is waived, mark all children as waived
        self.apply_auto_waiving();

        // Build the task tree
        let mut tree = TaskTree::new();

        // Add all tasks to the tree
        // We use a loop instead of moving directly to avoid issues with validation errors
        for task in self.tasks {
            // Note: We ignore errors here because:
            // 1. Duplicate IDs are already checked in add_line()
            // 2. Validation is handled during add_line()
            // 3. The tree builder ensures structural correctness
            // If add_task fails, it's a programming error, not a user error
            tree.add_task(task)
                .expect("Task tree construction failed: this is a bug in the task tree builder");
        }

        tree
    }

    /// Apply auto-waiving logic to the task tree
    ///
    /// If a parent task has status Waived, all its children should also be
    /// marked as Waived. This method traverses the tree depth-first and
    /// propagates waived status to descendants.
    fn apply_auto_waiving(&mut self) {
        // Find all waived tasks
        let waived_ids: Vec<String> = self
            .tasks
            .iter()
            .filter(|t| t.status == lash_types::TaskStatus::Waived)
            .map(|t| t.id.clone())
            .collect();

        // For each waived task, mark all descendants as waived
        for waived_id in waived_ids {
            self.mark_descendants_waived(&waived_id);
        }
    }

    /// Mark all descendants of a task as waived
    ///
    /// # Arguments
    ///
    /// * `parent_id` - The ID of the parent task whose descendants should be waived
    fn mark_descendants_waived(&mut self, parent_id: &str) {
        // Find all direct children of this parent
        let child_ids: Vec<String> = self
            .tasks
            .iter()
            .filter(|t| t.parent_id.as_deref() == Some(parent_id))
            .map(|t| t.id.clone())
            .collect();

        // Mark each child as waived and recurse
        for child_id in child_ids {
            // Find the task and update its status
            if let Some(task) = self.tasks.iter_mut().find(|t| t.id == child_id) {
                task.status = lash_types::TaskStatus::Waived;
            }

            // Recursively mark descendants
            self.mark_descendants_waived(&child_id);
        }
    }

    /// Validate that a depth is within limits
    ///
    /// # Arguments
    ///
    /// * `depth` - The depth to validate
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if depth is valid, or an error message.
    #[allow(dead_code)] // Will be used in Task #5
    fn validate_depth(&self, depth: u8) -> Result<(), String> {
        if depth > self.max_depth {
            Err(format!(
                "Task depth {depth} exceeds maximum depth {}",
                self.max_depth
            ))
        } else {
            Ok(())
        }
    }

    /// Validate that indentation doesn't skip levels
    ///
    /// # Arguments
    ///
    /// * `current_depth` - The depth of the current line
    /// * `previous_depth` - The depth of the previous line
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if indentation is valid, or an error message.
    #[allow(dead_code)] // Will be used in Task #5
    fn validate_indentation_jump(current_depth: u8, previous_depth: u8) -> Result<(), String> {
        if current_depth > previous_depth + 1 {
            Err(format!(
                "Cannot skip indentation levels: jumped from depth {previous_depth} to {current_depth}"
            ))
        } else {
            Ok(())
        }
    }

    /// Generate a synthetic ID for a task without an explicit @id
    ///
    /// The ID is generated from the task title by:
    /// 1. Converting to lowercase
    /// 2. Replacing spaces with hyphens
    /// 3. Removing non-alphanumeric characters (except hyphens)
    /// 4. Truncating to reasonable length
    /// 5. Adding a numeric suffix if the ID is already used
    ///
    /// # Arguments
    ///
    /// * `title` - The task title
    /// * `index` - The task's index in the file (fallback if title is empty)
    ///
    /// # Returns
    ///
    /// Returns a unique ID for the task.
    fn generate_synthetic_id(&mut self, title: &str, index: usize) -> String {
        // Create slug from title
        let slug = title
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' {
                    c
                } else if c.is_whitespace() {
                    '-'
                } else {
                    '\0' // Mark for removal
                }
            })
            .filter(|&c| c != '\0')
            .collect::<String>();

        // Truncate to reasonable length (40 chars)
        let truncated = if slug.chars().count() > 40 {
            slug.chars().take(40).collect::<String>()
        } else {
            slug
        };

        // Clean up: remove leading/trailing hyphens and collapse multiple hyphens
        let mut cleaned = truncated.trim_matches('-').to_string();
        while cleaned.contains("--") {
            cleaned = cleaned.replace("--", "-");
        }

        // Use numeric fallback if slug is empty
        let base_id = if cleaned.is_empty() {
            format!("task-{index}")
        } else {
            cleaned
        };

        // Check if ID is already used; if so, add numeric suffix
        if self.is_id_used(&base_id) {
            // Find next available numeric suffix
            let mut counter = 2;
            loop {
                let candidate = format!("{base_id}-{counter}");
                if !self.is_id_used(&candidate) {
                    return candidate;
                }
                counter += 1;
            }
        } else {
            base_id
        }
    }

    /// Check if an ID has already been used
    ///
    /// # Arguments
    ///
    /// * `id` - The ID to check
    ///
    /// # Returns
    ///
    /// Returns `true` if the ID has been used, `false` otherwise.
    #[allow(dead_code)] // Will be used in Task #5
    fn is_id_used(&self, id: &str) -> bool {
        self.used_ids.contains(id)
    }

    /// Mark an ID as used
    ///
    /// # Arguments
    ///
    /// * `id` - The ID to mark as used
    #[allow(dead_code)] // Will be used in Task #5
    fn mark_id_used(&mut self, id: String) {
        self.used_ids.insert(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::TaskStatus;

    // Helper function to create a checkbox line
    fn make_line(indent: usize, status: TaskStatus, title: &str, line_num: usize) -> CheckboxLine {
        CheckboxLine::new(
            indent,
            status,
            title.to_string(),
            vec![],
            line_num,
            indent + 1,
        )
    }

    // ===== Builder Creation Tests =====

    #[test]
    fn test_builder_creation() {
        let builder = TaskTreeBuilder::new(2);
        assert_eq!(builder.max_depth, 2);
        assert_eq!(builder.tasks.len(), 0);
        assert_eq!(builder.used_ids.len(), 0);
    }

    // ===== Depth Validation Tests =====

    #[test]
    fn test_validate_depth_within_limit() {
        let builder = TaskTreeBuilder::new(2);
        assert!(builder.validate_depth(0).is_ok());
        assert!(builder.validate_depth(1).is_ok());
        assert!(builder.validate_depth(2).is_ok());
    }

    #[test]
    fn test_validate_depth_exceeds_limit() {
        let builder = TaskTreeBuilder::new(2);
        assert!(builder.validate_depth(3).is_err());
        assert!(builder.validate_depth(10).is_err());
    }

    #[test]
    fn test_validate_depth_error_message() {
        let builder = TaskTreeBuilder::new(2);
        let err = builder.validate_depth(3).unwrap_err();
        assert!(err.contains("exceeds maximum depth"));
        assert!(err.contains('3'));
        assert!(err.contains('2'));
    }

    // ===== Indentation Jump Validation Tests =====

    #[test]
    fn test_validate_indentation_no_skip() {
        assert!(TaskTreeBuilder::validate_indentation_jump(0, 0).is_ok());
        assert!(TaskTreeBuilder::validate_indentation_jump(1, 0).is_ok());
        assert!(TaskTreeBuilder::validate_indentation_jump(1, 1).is_ok());
        assert!(TaskTreeBuilder::validate_indentation_jump(2, 1).is_ok());
        assert!(TaskTreeBuilder::validate_indentation_jump(0, 2).is_ok()); // Going back is ok
        assert!(TaskTreeBuilder::validate_indentation_jump(0, 1).is_ok()); // Going back is ok
    }

    #[test]
    fn test_validate_indentation_skip() {
        // Cannot jump from depth 0 to depth 2
        assert!(TaskTreeBuilder::validate_indentation_jump(2, 0).is_err());
        // Cannot jump from depth 1 to depth 3
        assert!(TaskTreeBuilder::validate_indentation_jump(3, 1).is_err());
    }

    #[test]
    fn test_validate_indentation_skip_error_message() {
        let err = TaskTreeBuilder::validate_indentation_jump(3, 1).unwrap_err();
        assert!(err.contains("Cannot skip indentation levels"));
        assert!(err.contains('1'));
        assert!(err.contains('3'));
    }

    // ===== ID Tracking Tests =====

    #[test]
    fn test_id_tracking() {
        let mut builder = TaskTreeBuilder::new(2);
        assert!(!builder.is_id_used("test-id"));

        builder.mark_id_used("test-id".to_string());
        assert!(builder.is_id_used("test-id"));
        assert!(!builder.is_id_used("other-id"));
    }

    #[test]
    fn test_multiple_ids() {
        let mut builder = TaskTreeBuilder::new(2);
        builder.mark_id_used("id-1".to_string());
        builder.mark_id_used("id-2".to_string());
        builder.mark_id_used("id-3".to_string());

        assert!(builder.is_id_used("id-1"));
        assert!(builder.is_id_used("id-2"));
        assert!(builder.is_id_used("id-3"));
        assert!(!builder.is_id_used("id-4"));
    }

    // ===== Synthetic ID Generation Tests =====

    #[test]
    fn test_generate_id_simple_title() {
        let mut builder = TaskTreeBuilder::new(2);
        let id = builder.generate_synthetic_id("Simple task", 0);
        assert_eq!(id, "simple-task");
    }

    #[test]
    fn test_generate_id_with_special_chars() {
        let mut builder = TaskTreeBuilder::new(2);
        let id = builder.generate_synthetic_id("Task: with (special) chars!", 0);
        assert_eq!(id, "task-with-special-chars");
    }

    #[test]
    fn test_generate_id_long_title() {
        let mut builder = TaskTreeBuilder::new(2);
        let long_title = "This is a very long task title that exceeds forty characters in length";
        let id = builder.generate_synthetic_id(long_title, 0);
        assert!(id.len() <= 40);
        assert_eq!(id, "this-is-a-very-long-task-title-that-exce");
    }

    #[test]
    fn test_generate_id_empty_title() {
        let mut builder = TaskTreeBuilder::new(2);
        let id = builder.generate_synthetic_id("", 5);
        assert_eq!(id, "task-5");
    }

    #[test]
    fn test_generate_id_special_chars_only() {
        let mut builder = TaskTreeBuilder::new(2);
        let id = builder.generate_synthetic_id("!!!", 3);
        assert_eq!(id, "task-3");
    }

    #[test]
    fn test_generate_id_duplicate_resolution() {
        let mut builder = TaskTreeBuilder::new(2);
        let id1 = builder.generate_synthetic_id("Task", 0);
        assert_eq!(id1, "task");
        builder.mark_id_used(id1);

        let id2 = builder.generate_synthetic_id("Task", 1);
        assert_eq!(id2, "task-2");
        builder.mark_id_used(id2);

        let id3 = builder.generate_synthetic_id("Task", 2);
        assert_eq!(id3, "task-3");
    }

    #[test]
    fn test_generate_id_collapse_multiple_hyphens() {
        let mut builder = TaskTreeBuilder::new(2);
        let id = builder.generate_synthetic_id("Task  with   spaces", 0);
        assert_eq!(id, "task-with-spaces");
    }

    #[test]
    fn test_generate_id_remove_leading_trailing_hyphens() {
        let mut builder = TaskTreeBuilder::new(2);
        let id = builder.generate_synthetic_id("  Task  ", 0);
        assert_eq!(id, "task");
    }

    // ===== Simple Flat List Tests =====

    #[test]
    fn test_flat_list_single_task() {
        let mut builder = TaskTreeBuilder::new(2);
        let line = make_line(0, TaskStatus::Open, "Task 1", 1);
        builder.add_line(&line).unwrap();

        let tree = builder.build();
        assert_eq!(tree.len(), 1);

        let task = tree.get_task("task-1").unwrap();
        assert_eq!(task.title, "Task 1");
        assert_eq!(task.depth, 0);
        assert!(task.parent_id.is_none());
        assert_eq!(task.order_index, 0);
    }

    #[test]
    fn test_flat_list_multiple_tasks() {
        let mut builder = TaskTreeBuilder::new(2);
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Task 1", 1))
            .unwrap();
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Task 2", 2))
            .unwrap();
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Task 3", 3))
            .unwrap();

        let tree = builder.build();
        assert_eq!(tree.len(), 3);

        let task1 = tree.get_task("task-1").unwrap();
        assert_eq!(task1.order_index, 0);

        let task2 = tree.get_task("task-2").unwrap();
        assert_eq!(task2.order_index, 1);

        let task3 = tree.get_task("task-3").unwrap();
        assert_eq!(task3.order_index, 2);
    }

    // ===== Two-Level Hierarchy Tests =====

    #[test]
    fn test_two_level_parent_child() {
        let mut builder = TaskTreeBuilder::new(2);
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Parent", 1))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Open, "Child", 2))
            .unwrap();

        let tree = builder.build();
        assert_eq!(tree.len(), 2);

        let parent = tree.get_task("parent").unwrap();
        assert_eq!(parent.depth, 0);
        assert!(parent.parent_id.is_none());

        let child = tree.get_task("child").unwrap();
        assert_eq!(child.depth, 1);
        assert_eq!(child.parent_id.as_deref(), Some("parent"));
        assert_eq!(child.order_index, 0);
    }

    #[test]
    fn test_two_level_multiple_children() {
        let mut builder = TaskTreeBuilder::new(2);
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Parent", 1))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Open, "Child 1", 2))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Open, "Child 2", 3))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Open, "Child 3", 4))
            .unwrap();

        let tree = builder.build();
        assert_eq!(tree.len(), 4);

        let children = tree.get_children("parent");
        assert_eq!(children.len(), 3);

        let child1 = tree.get_task("child-1").unwrap();
        assert_eq!(child1.order_index, 0);

        let child2 = tree.get_task("child-2").unwrap();
        assert_eq!(child2.order_index, 1);

        let child3 = tree.get_task("child-3").unwrap();
        assert_eq!(child3.order_index, 2);
    }

    #[test]
    fn test_two_level_multiple_parents() {
        let mut builder = TaskTreeBuilder::new(2);
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Parent 1", 1))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Open, "Child 1A", 2))
            .unwrap();
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Parent 2", 3))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Open, "Child 2A", 4))
            .unwrap();

        let tree = builder.build();
        assert_eq!(tree.len(), 4);

        let child1a = tree.get_task("child-1a").unwrap();
        assert_eq!(child1a.parent_id.as_deref(), Some("parent-1"));

        let child2a = tree.get_task("child-2a").unwrap();
        assert_eq!(child2a.parent_id.as_deref(), Some("parent-2"));
    }

    // ===== Three-Level Hierarchy Tests =====

    #[test]
    fn test_three_level_hierarchy() {
        let mut builder = TaskTreeBuilder::new(2);
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Level 0", 1))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Open, "Level 1", 2))
            .unwrap();
        builder
            .add_line(&make_line(4, TaskStatus::Open, "Level 2", 3))
            .unwrap();

        let tree = builder.build();
        assert_eq!(tree.len(), 3);

        let level0 = tree.get_task("level-0").unwrap();
        assert_eq!(level0.depth, 0);
        assert!(level0.parent_id.is_none());

        let level1 = tree.get_task("level-1").unwrap();
        assert_eq!(level1.depth, 1);
        assert_eq!(level1.parent_id.as_deref(), Some("level-0"));

        let level2 = tree.get_task("level-2").unwrap();
        assert_eq!(level2.depth, 2);
        assert_eq!(level2.parent_id.as_deref(), Some("level-1"));
    }

    #[test]
    fn test_three_level_complex_tree() {
        let mut builder = TaskTreeBuilder::new(2);
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Root", 1))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Open, "Child 1", 2))
            .unwrap();
        builder
            .add_line(&make_line(4, TaskStatus::Open, "Grandchild 1A", 3))
            .unwrap();
        builder
            .add_line(&make_line(4, TaskStatus::Open, "Grandchild 1B", 4))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Open, "Child 2", 5))
            .unwrap();
        builder
            .add_line(&make_line(4, TaskStatus::Open, "Grandchild 2A", 6))
            .unwrap();

        let tree = builder.build();
        assert_eq!(tree.len(), 6);

        let grandchild1a = tree.get_task("grandchild-1a").unwrap();
        assert_eq!(grandchild1a.parent_id.as_deref(), Some("child-1"));
        assert_eq!(grandchild1a.depth, 2);

        let grandchild2a = tree.get_task("grandchild-2a").unwrap();
        assert_eq!(grandchild2a.parent_id.as_deref(), Some("child-2"));
        assert_eq!(grandchild2a.depth, 2);
    }

    // ===== Sibling Tasks Tests =====

    #[test]
    fn test_siblings_at_same_depth() {
        let mut builder = TaskTreeBuilder::new(2);
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Task 1", 1))
            .unwrap();
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Task 2", 2))
            .unwrap();
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Task 3", 3))
            .unwrap();

        let tree = builder.build();

        let task1 = tree.get_task("task-1").unwrap();
        let task2 = tree.get_task("task-2").unwrap();
        let task3 = tree.get_task("task-3").unwrap();

        assert_eq!(task1.depth, task2.depth);
        assert_eq!(task2.depth, task3.depth);
        assert_eq!(task1.parent_id, task2.parent_id);
        assert_eq!(task2.parent_id, task3.parent_id);
    }

    #[test]
    #[allow(clippy::similar_names)]
    fn test_siblings_with_different_parents() {
        let mut builder = TaskTreeBuilder::new(2);
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Parent 1", 1))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Open, "Child 1A", 2))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Open, "Child 1B", 3))
            .unwrap();
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Parent 2", 4))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Open, "Child 2A", 5))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Open, "Child 2B", 6))
            .unwrap();

        let tree = builder.build();

        let child1a = tree.get_task("child-1a").unwrap();
        let child1b = tree.get_task("child-1b").unwrap();
        assert_eq!(child1a.parent_id, child1b.parent_id);

        let child2a = tree.get_task("child-2a").unwrap();
        let child2b = tree.get_task("child-2b").unwrap();
        assert_eq!(child2a.parent_id, child2b.parent_id);

        assert_ne!(child1a.parent_id, child2a.parent_id);
    }

    // ===== Depth Limit Error Tests =====

    #[test]
    fn test_depth_limit_exceeded() {
        let mut builder = TaskTreeBuilder::new(2); // Max depth 2 (3 levels: 0, 1, 2)
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Level 0", 1))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Open, "Level 1", 2))
            .unwrap();
        builder
            .add_line(&make_line(4, TaskStatus::Open, "Level 2", 3))
            .unwrap();

        // Try to add level 3 (depth 3) - should fail
        let result = builder.add_line(&make_line(6, TaskStatus::Open, "Level 3", 4));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exceeds maximum depth"));
    }

    #[test]
    fn test_depth_limit_at_boundary() {
        let mut builder = TaskTreeBuilder::new(2);
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Level 0", 1))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Open, "Level 1", 2))
            .unwrap();

        // Adding level 2 (depth 2) should succeed
        let result = builder.add_line(&make_line(4, TaskStatus::Open, "Level 2", 3));
        assert!(result.is_ok());
    }

    // ===== Skipped Indentation Level Error Tests =====

    #[test]
    fn test_skip_indentation_level() {
        let mut builder = TaskTreeBuilder::new(3);
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Level 0", 1))
            .unwrap();

        // Try to jump from depth 0 to depth 2 - should fail
        let result = builder.add_line(&make_line(4, TaskStatus::Open, "Level 2", 2));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Cannot skip indentation levels"));
    }

    #[test]
    fn test_skip_from_depth_1_to_3() {
        let mut builder = TaskTreeBuilder::new(3);
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Level 0", 1))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Open, "Level 1", 2))
            .unwrap();

        // Try to jump from depth 1 to depth 3 - should fail
        let result = builder.add_line(&make_line(6, TaskStatus::Open, "Level 3", 3));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Cannot skip indentation levels"));
    }

    #[test]
    fn test_invalid_odd_indentation() {
        let mut builder = TaskTreeBuilder::new(2);

        // Create a line with odd indentation (3 spaces)
        let line = CheckboxLine::new(3, TaskStatus::Open, "Task".to_string(), vec![], 1, 4);
        let result = builder.add_line(&line);

        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("Invalid indentation"));
        assert!(err_msg.contains("must be multiple of 2"));
    }

    // ===== Duplicate ID Tests =====

    #[test]
    fn test_duplicate_ids_same_title() {
        let mut builder = TaskTreeBuilder::new(2);
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Duplicate", 1))
            .unwrap();
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Duplicate", 2))
            .unwrap();

        let tree = builder.build();

        // Should have two tasks with different IDs
        assert_eq!(tree.len(), 2);
        assert!(tree.get_task("duplicate").is_some());
        assert!(tree.get_task("duplicate-2").is_some());
    }

    // ===== Auto-Waiving Tests =====

    #[test]
    fn test_auto_waiving_single_child() {
        let mut builder = TaskTreeBuilder::new(2);
        builder
            .add_line(&make_line(0, TaskStatus::Waived, "Waived parent", 1))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Open, "Child", 2))
            .unwrap();

        let tree = builder.build();

        let child = tree.get_task("child").unwrap();
        assert_eq!(child.status, TaskStatus::Waived);
    }

    #[test]
    fn test_auto_waiving_multiple_children() {
        let mut builder = TaskTreeBuilder::new(2);
        builder
            .add_line(&make_line(0, TaskStatus::Waived, "Waived parent", 1))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Open, "Child 1", 2))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Done, "Child 2", 3))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Blocked, "Child 3", 4))
            .unwrap();

        let tree = builder.build();

        let child1 = tree.get_task("child-1").unwrap();
        assert_eq!(child1.status, TaskStatus::Waived);

        let child2 = tree.get_task("child-2").unwrap();
        assert_eq!(child2.status, TaskStatus::Waived);

        let child3 = tree.get_task("child-3").unwrap();
        assert_eq!(child3.status, TaskStatus::Waived);
    }

    #[test]
    fn test_auto_waiving_grandchildren() {
        let mut builder = TaskTreeBuilder::new(2);
        builder
            .add_line(&make_line(0, TaskStatus::Waived, "Waived root", 1))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Open, "Child", 2))
            .unwrap();
        builder
            .add_line(&make_line(4, TaskStatus::Open, "Grandchild", 3))
            .unwrap();

        let tree = builder.build();

        let child = tree.get_task("child").unwrap();
        assert_eq!(child.status, TaskStatus::Waived);

        let grandchild = tree.get_task("grandchild").unwrap();
        assert_eq!(grandchild.status, TaskStatus::Waived);
    }

    #[test]
    fn test_auto_waiving_partial_tree() {
        let mut builder = TaskTreeBuilder::new(2);
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Normal parent", 1))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Open, "Normal child", 2))
            .unwrap();
        builder
            .add_line(&make_line(0, TaskStatus::Waived, "Waived parent", 3))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Open, "Waived child", 4))
            .unwrap();

        let tree = builder.build();

        let normal_child = tree.get_task("normal-child").unwrap();
        assert_eq!(normal_child.status, TaskStatus::Open);

        let waived_child = tree.get_task("waived-child").unwrap();
        assert_eq!(waived_child.status, TaskStatus::Waived);
    }

    // ===== Order Index Tests =====

    #[test]
    fn test_order_index_sequential() {
        let mut builder = TaskTreeBuilder::new(2);
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Task 1", 1))
            .unwrap();
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Task 2", 2))
            .unwrap();
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Task 3", 3))
            .unwrap();

        let tree = builder.build();

        let task1 = tree.get_task("task-1").unwrap();
        assert_eq!(task1.order_index, 0);

        let task2 = tree.get_task("task-2").unwrap();
        assert_eq!(task2.order_index, 1);

        let task3 = tree.get_task("task-3").unwrap();
        assert_eq!(task3.order_index, 2);
    }

    #[test]
    #[allow(clippy::similar_names)]
    fn test_order_index_resets_per_parent() {
        let mut builder = TaskTreeBuilder::new(2);
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Parent 1", 1))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Open, "Child 1A", 2))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Open, "Child 1B", 3))
            .unwrap();
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Parent 2", 4))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Open, "Child 2A", 5))
            .unwrap();

        let tree = builder.build();

        let child1a = tree.get_task("child-1a").unwrap();
        assert_eq!(child1a.order_index, 0);

        let child1b = tree.get_task("child-1b").unwrap();
        assert_eq!(child1b.order_index, 1);

        let child2a = tree.get_task("child-2a").unwrap();
        assert_eq!(child2a.order_index, 0); // Resets for new parent
    }

    // ===== Edge Cases =====

    #[test]
    fn test_empty_builder() {
        let builder = TaskTreeBuilder::new(2);
        let tree = builder.build();
        assert_eq!(tree.len(), 0);
        assert!(tree.is_empty());
    }

    #[test]
    fn test_return_to_root_level() {
        let mut builder = TaskTreeBuilder::new(2);
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Root 1", 1))
            .unwrap();
        builder
            .add_line(&make_line(2, TaskStatus::Open, "Child", 2))
            .unwrap();
        builder
            .add_line(&make_line(4, TaskStatus::Open, "Grandchild", 3))
            .unwrap();
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Root 2", 4))
            .unwrap();

        let tree = builder.build();
        assert_eq!(tree.len(), 4);

        let root2 = tree.get_task("root-2").unwrap();
        assert_eq!(root2.depth, 0);
        assert!(root2.parent_id.is_none());
    }

    #[test]
    fn test_labels_preserved() {
        let mut builder = TaskTreeBuilder::new(2);
        let mut line = make_line(0, TaskStatus::Open, "Task with labels", 1);
        line.labels = vec![
            lash_types::Label::new("backend"),
            lash_types::Label::new("api"),
        ];
        builder.add_line(&line).unwrap();

        let tree = builder.build();
        let task = tree.get_task("task-with-labels").unwrap();
        assert_eq!(task.metadata.labels.len(), 2);
        assert!(task.metadata.labels.contains(&"backend".to_string()));
        assert!(task.metadata.labels.contains(&"api".to_string()));
    }

    #[test]
    fn test_status_preserved() {
        let mut builder = TaskTreeBuilder::new(2);
        builder
            .add_line(&make_line(0, TaskStatus::Open, "Open", 1))
            .unwrap();
        builder
            .add_line(&make_line(0, TaskStatus::Done, "Done", 2))
            .unwrap();
        builder
            .add_line(&make_line(0, TaskStatus::Waived, "Waived", 3))
            .unwrap();
        builder
            .add_line(&make_line(0, TaskStatus::Blocked, "Blocked", 4))
            .unwrap();

        let tree = builder.build();

        assert_eq!(tree.get_task("open").unwrap().status, TaskStatus::Open);
        assert_eq!(tree.get_task("done").unwrap().status, TaskStatus::Done);
        assert_eq!(tree.get_task("waived").unwrap().status, TaskStatus::Waived);
        assert_eq!(
            tree.get_task("blocked").unwrap().status,
            TaskStatus::Blocked
        );
    }
}
