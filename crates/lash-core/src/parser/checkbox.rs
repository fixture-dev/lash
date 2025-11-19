//! Checkbox line parsing
//!
//! This module handles parsing of individual checkbox task lines, including:
//! - Indentation detection and validation
//! - Status character extraction (` `, `x`, `-`, `!`)
//! - Task title extraction
//! - Inline label parsing (#tag)
//!
//! Checkbox lines are the core building blocks of task lists. Each line is
//! parsed into a `CheckboxLine` structure that captures all the information
//! needed to build the task tree.

use lash_types::{Label, TaskStatus};

/// Intermediate representation of a parsed checkbox line
///
/// This structure represents a single checkbox line before it's incorporated
/// into the task tree. It contains all the information extracted from the line:
/// indentation, status, title, and inline labels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckboxLine {
    /// Number of leading spaces (indentation)
    pub indent: usize,

    /// Computed nesting depth (indent / 2)
    pub depth: u8,

    /// Task status extracted from checkbox
    pub status: TaskStatus,

    /// Task title (text after checkbox)
    pub title: String,

    /// Inline labels parsed from title (#tag)
    pub labels: Vec<Label>,

    /// Line number in source file (1-indexed)
    pub line_num: usize,

    /// Column number where checkbox starts (1-indexed)
    pub column: usize,
}

impl CheckboxLine {
    /// Create a new checkbox line
    ///
    /// # Arguments
    ///
    /// * `indent` - Number of leading spaces
    /// * `status` - Task status
    /// * `title` - Task title text
    /// * `labels` - Inline labels
    /// * `line_num` - Line number in source file
    /// * `column` - Column number where checkbox starts
    #[must_use]
    pub fn new(
        indent: usize,
        status: TaskStatus,
        title: String,
        labels: Vec<Label>,
        line_num: usize,
        column: usize,
    ) -> Self {
        #[allow(clippy::cast_possible_truncation)] // Depth is limited by max_depth config
        let depth = (indent / 2) as u8;
        Self {
            indent,
            depth,
            status,
            title,
            labels,
            line_num,
            column,
        }
    }

    /// Parse a checkbox line from a string
    ///
    /// This is the main parsing function that extracts all information from
    /// a checkbox line. It returns `None` if the line is not a valid checkbox.
    ///
    /// # Arguments
    ///
    /// * `line` - The line to parse
    /// * `line_num` - Line number in source file (for error reporting)
    ///
    /// # Returns
    ///
    /// Returns `Some(CheckboxLine)` if the line is a valid checkbox, or `None`
    /// if it's not a checkbox line (which is not an error - files can have
    /// non-checkbox content).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let line = "  - [x] Complete task #backend";
    /// let parsed = CheckboxLine::parse(line, 5);
    /// assert_eq!(parsed.unwrap().depth, 1); // 2 spaces = depth 1
    /// assert_eq!(parsed.unwrap().status, TaskStatus::Done);
    /// ```
    #[must_use]
    pub fn parse(_line: &str, _line_num: usize) -> Option<Self> {
        // TODO: Implement in Task #2
        // This function will be implemented in the "Implement Checkbox Line Parser" task
        None
    }

    /// Validate that indentation is correct (multiple of 2 spaces)
    ///
    /// # Returns
    ///
    /// Returns `true` if indentation is valid, `false` otherwise.
    #[must_use]
    pub fn has_valid_indentation(&self) -> bool {
        self.indent % 2 == 0
    }

    /// Check if this line can be a child of another line based on depth
    ///
    /// A line can be a child if its depth is exactly one more than the
    /// potential parent's depth.
    ///
    /// # Arguments
    ///
    /// * `potential_parent` - The potential parent checkbox line
    ///
    /// # Returns
    ///
    /// Returns `true` if this line can be a child of the potential parent.
    #[must_use]
    pub fn can_be_child_of(&self, potential_parent: &Self) -> bool {
        self.depth == potential_parent.depth + 1
    }

    /// Check if this line is a sibling of another line (same depth)
    ///
    /// # Arguments
    ///
    /// * `other` - Another checkbox line
    ///
    /// # Returns
    ///
    /// Returns `true` if both lines have the same depth.
    #[must_use]
    pub fn is_sibling_of(&self, other: &Self) -> bool {
        self.depth == other.depth
    }
}

/// Parse inline labels from a task title
///
/// Scans the title for `#word` patterns and extracts them as labels.
/// Labels are alphanumeric with optional hyphens.
///
/// # Arguments
///
/// * `title` - The task title to scan
///
/// # Returns
///
/// Returns a vector of labels found in the title.
///
/// # Example
///
/// ```rust,ignore
/// let labels = parse_inline_labels("Complete backend #api #database");
/// assert_eq!(labels.len(), 2);
/// ```
#[must_use]
pub fn parse_inline_labels(_title: &str) -> Vec<Label> {
    // TODO: Implement in Task #2
    // This will use Label::parse_inline_labels from lash-types
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkbox_line_creation() {
        let line = CheckboxLine::new(2, TaskStatus::Open, "Test task".to_string(), vec![], 5, 1);

        assert_eq!(line.indent, 2);
        assert_eq!(line.depth, 1); // 2 spaces = depth 1
        assert_eq!(line.status, TaskStatus::Open);
        assert_eq!(line.title, "Test task");
        assert_eq!(line.line_num, 5);
        assert_eq!(line.column, 1);
    }

    #[test]
    fn test_valid_indentation() {
        let valid = CheckboxLine::new(4, TaskStatus::Open, "Task".to_string(), vec![], 1, 1);
        assert!(valid.has_valid_indentation());

        let invalid = CheckboxLine::new(3, TaskStatus::Open, "Task".to_string(), vec![], 1, 1);
        assert!(!invalid.has_valid_indentation());
    }

    #[test]
    fn test_child_relationship() {
        let parent = CheckboxLine::new(0, TaskStatus::Open, "Parent".to_string(), vec![], 1, 1);
        let child = CheckboxLine::new(2, TaskStatus::Open, "Child".to_string(), vec![], 2, 1);
        let grandchild =
            CheckboxLine::new(4, TaskStatus::Open, "Grandchild".to_string(), vec![], 3, 1);

        assert!(child.can_be_child_of(&parent));
        assert!(!grandchild.can_be_child_of(&parent));
        assert!(grandchild.can_be_child_of(&child));
    }

    #[test]
    fn test_sibling_relationship() {
        let task1 = CheckboxLine::new(2, TaskStatus::Open, "Task 1".to_string(), vec![], 1, 1);
        let task2 = CheckboxLine::new(2, TaskStatus::Open, "Task 2".to_string(), vec![], 2, 1);
        let different_depth =
            CheckboxLine::new(4, TaskStatus::Open, "Task 3".to_string(), vec![], 3, 1);

        assert!(task1.is_sibling_of(&task2));
        assert!(!task1.is_sibling_of(&different_depth));
    }
}
