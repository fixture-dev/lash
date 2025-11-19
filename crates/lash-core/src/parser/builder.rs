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

use super::checkbox::CheckboxLine;
use lash_types::{Task, TaskTree};

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
    #[allow(dead_code)] // Will be used in Task #5
    pub fn add_line(&mut self, _line: &CheckboxLine) -> Result<(), String> {
        // TODO: Implement in Task #5
        // This function will be implemented in the "Implement Task Tree Builder" task
        Ok(())
    }

    /// Build the final task tree
    ///
    /// Converts the flat list of tasks with parent indices into a proper
    /// `TaskTree` structure.
    ///
    /// # Returns
    ///
    /// Returns the constructed task tree.
    #[allow(dead_code)] // Will be used in Task #5
    #[must_use]
    pub fn build(self) -> TaskTree {
        // TODO: Implement in Task #5
        // This function will be implemented in the "Implement Task Tree Builder" task
        TaskTree::new()
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
    #[allow(dead_code)] // Will be used in Task #5
    #[allow(clippy::unused_self)] // Self will be used when fully implemented
    fn generate_synthetic_id(&mut self, _title: &str, _index: usize) -> String {
        // TODO: Implement in Task #5
        // This function will be implemented in the "Implement Task Tree Builder" task
        String::from("task-0")
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

    #[test]
    fn test_builder_creation() {
        let builder = TaskTreeBuilder::new(2);
        assert_eq!(builder.max_depth, 2);
        assert_eq!(builder.tasks.len(), 0);
        assert_eq!(builder.used_ids.len(), 0);
    }

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
    fn test_validate_indentation_no_skip() {
        assert!(TaskTreeBuilder::validate_indentation_jump(0, 0).is_ok());
        assert!(TaskTreeBuilder::validate_indentation_jump(1, 0).is_ok());
        assert!(TaskTreeBuilder::validate_indentation_jump(1, 1).is_ok());
        assert!(TaskTreeBuilder::validate_indentation_jump(2, 1).is_ok());
    }

    #[test]
    fn test_validate_indentation_skip() {
        // Cannot jump from depth 0 to depth 2
        assert!(TaskTreeBuilder::validate_indentation_jump(2, 0).is_err());
        // Cannot jump from depth 1 to depth 3
        assert!(TaskTreeBuilder::validate_indentation_jump(3, 1).is_err());
    }

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
}
