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
    pub fn parse(line: &str, line_num: usize) -> Option<Self> {
        // Count leading spaces (indentation)
        let indent = count_leading_spaces(line)?;

        // Get the rest of the line after indentation
        let rest = &line[indent..];

        // Check for checkbox pattern: "- [STATUS]"
        if !rest.starts_with("- [") {
            return None;
        }

        // Find the closing bracket
        let close_bracket_pos = rest.find(']')?;
        if close_bracket_pos < 3 {
            return None; // Too short to be valid
        }

        // Extract status character (should be at position 3)
        let status_char = rest.chars().nth(3)?;

        // Verify closing bracket is at position 4
        if close_bracket_pos != 4 {
            return None; // Invalid format (too many chars in brackets)
        }

        // Parse the status character
        let status = TaskStatus::from_checkbox_char(status_char).ok()?;

        // Extract title (everything after "- [X]")
        // The pattern is "- [X]" (5 chars), then optional space, then title
        let after_bracket = 5; // Length of "- [X]"
        if rest.len() <= after_bracket {
            return None; // No title
        }

        let title_with_metadata = rest[after_bracket..].trim();
        if title_with_metadata.is_empty() {
            return None; // Empty title
        }

        // Parse trailing metadata block if present
        let (title, _metadata) = extract_trailing_metadata(title_with_metadata);

        // Parse inline labels from title
        let labels = parse_inline_labels(title);

        // Column where checkbox starts (after indentation, at the dash)
        let column = indent + 1;

        Some(Self::new(
            indent,
            status,
            title.to_string(),
            labels,
            line_num,
            column,
        ))
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

/// Count leading spaces in a line
///
/// Returns `None` if the line contains tabs or is empty/whitespace-only.
/// Returns `Some(count)` for valid space indentation.
///
/// # Arguments
///
/// * `line` - The line to analyze
///
/// # Returns
///
/// - `Some(count)` if line has valid space indentation
/// - `None` if line contains tabs, is empty, or is whitespace-only
fn count_leading_spaces(line: &str) -> Option<usize> {
    if line.is_empty() {
        return None;
    }

    let mut count = 0;
    for ch in line.chars() {
        match ch {
            ' ' => count += 1,
            '\t' => return None, // Tabs not allowed
            _ => break,
        }
    }

    // If entire line is whitespace, return None
    if line.trim().is_empty() {
        return None;
    }

    Some(count)
}

/// Extract trailing metadata block from title
///
/// Looks for pattern like `[@key: value, @key2: value2]` at the end of the title.
/// Returns the title without metadata and the extracted metadata string.
///
/// # Arguments
///
/// * `title` - The full title text potentially containing metadata
///
/// # Returns
///
/// Returns `(clean_title, metadata_string)` tuple. If no metadata block found,
/// returns the original title and `None`.
fn extract_trailing_metadata(title: &str) -> (&str, Option<&str>) {
    // Look for trailing [...] pattern
    if let Some(open_pos) = title.rfind('[') {
        if title.ends_with(']') {
            // Check if this looks like a metadata block (starts with @)
            let potential_metadata = &title[open_pos + 1..title.len() - 1];
            if potential_metadata.trim_start().starts_with('@') {
                let clean_title = title[..open_pos].trim_end();
                return (clean_title, Some(potential_metadata));
            }
        }
    }

    (title, None)
}

/// Parse inline labels from a task title
///
/// Scans the title for `#word` patterns and extracts them as labels.
/// Labels are alphanumeric with optional hyphens.
///
/// This function delegates to `lash_types::label::parse_inline_labels`.
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
pub fn parse_inline_labels(title: &str) -> Vec<Label> {
    lash_types::label::parse_inline_labels(title)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== CheckboxLine Construction Tests =====

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

    // ===== Parse Valid Checkbox Patterns =====

    #[test]
    fn test_parse_open_status() {
        let line = "- [ ] Open task";
        let parsed = CheckboxLine::parse(line, 1).unwrap();

        assert_eq!(parsed.indent, 0);
        assert_eq!(parsed.depth, 0);
        assert_eq!(parsed.status, TaskStatus::Open);
        assert_eq!(parsed.title, "Open task");
        assert_eq!(parsed.line_num, 1);
        assert_eq!(parsed.column, 1);
    }

    #[test]
    fn test_parse_done_status_lowercase() {
        let line = "- [x] Done task";
        let parsed = CheckboxLine::parse(line, 1).unwrap();

        assert_eq!(parsed.status, TaskStatus::Done);
        assert_eq!(parsed.title, "Done task");
    }

    #[test]
    fn test_parse_done_status_uppercase() {
        let line = "- [X] Done task";
        let parsed = CheckboxLine::parse(line, 1).unwrap();

        assert_eq!(parsed.status, TaskStatus::Done);
        assert_eq!(parsed.title, "Done task");
    }

    #[test]
    fn test_parse_waived_status() {
        let line = "- [-] Waived task";
        let parsed = CheckboxLine::parse(line, 1).unwrap();

        assert_eq!(parsed.status, TaskStatus::Waived);
        assert_eq!(parsed.title, "Waived task");
    }

    #[test]
    fn test_parse_blocked_status() {
        let line = "- [!] Blocked task";
        let parsed = CheckboxLine::parse(line, 1).unwrap();

        assert_eq!(parsed.status, TaskStatus::Blocked);
        assert_eq!(parsed.title, "Blocked task");
    }

    // ===== Parse Indentation Levels =====

    #[test]
    fn test_parse_zero_indent() {
        let line = "- [ ] Top level task";
        let parsed = CheckboxLine::parse(line, 1).unwrap();

        assert_eq!(parsed.indent, 0);
        assert_eq!(parsed.depth, 0);
    }

    #[test]
    fn test_parse_two_space_indent() {
        let line = "  - [ ] Child task";
        let parsed = CheckboxLine::parse(line, 1).unwrap();

        assert_eq!(parsed.indent, 2);
        assert_eq!(parsed.depth, 1);
    }

    #[test]
    fn test_parse_four_space_indent() {
        let line = "    - [ ] Grandchild task";
        let parsed = CheckboxLine::parse(line, 1).unwrap();

        assert_eq!(parsed.indent, 4);
        assert_eq!(parsed.depth, 2);
    }

    #[test]
    fn test_parse_six_space_indent() {
        let line = "      - [ ] Great grandchild task";
        let parsed = CheckboxLine::parse(line, 1).unwrap();

        assert_eq!(parsed.indent, 6);
        assert_eq!(parsed.depth, 3);
    }

    // ===== Parse with Labels =====

    #[test]
    fn test_parse_single_label() {
        let line = "- [ ] Task #backend";
        let parsed = CheckboxLine::parse(line, 1).unwrap();

        assert_eq!(parsed.title, "Task #backend");
        assert_eq!(parsed.labels.len(), 1);
        assert_eq!(parsed.labels[0].name, "backend");
    }

    #[test]
    fn test_parse_multiple_labels() {
        let line = "- [ ] Task #backend #api #database";
        let parsed = CheckboxLine::parse(line, 1).unwrap();

        assert_eq!(parsed.title, "Task #backend #api #database");
        assert_eq!(parsed.labels.len(), 3);
        // Labels are sorted alphabetically by parse_inline_labels
        assert!(parsed.labels.iter().any(|l| l.name == "backend"));
        assert!(parsed.labels.iter().any(|l| l.name == "api"));
        assert!(parsed.labels.iter().any(|l| l.name == "database"));
    }

    #[test]
    fn test_parse_labels_with_punctuation() {
        let line = "- [ ] Task #backend, #api!";
        let parsed = CheckboxLine::parse(line, 1).unwrap();

        assert_eq!(parsed.labels.len(), 2);
        assert!(parsed.labels.iter().any(|l| l.name == "backend"));
        assert!(parsed.labels.iter().any(|l| l.name == "api"));
    }

    // ===== Parse with Metadata Blocks =====

    #[test]
    fn test_parse_trailing_metadata_single() {
        let line = "- [ ] Task [@owner: alice]";
        let parsed = CheckboxLine::parse(line, 1).unwrap();

        assert_eq!(parsed.title, "Task");
    }

    #[test]
    fn test_parse_trailing_metadata_multiple() {
        let line = "- [ ] Task [@owner: alice, @estimate: 2h]";
        let parsed = CheckboxLine::parse(line, 1).unwrap();

        assert_eq!(parsed.title, "Task");
    }

    #[test]
    fn test_parse_with_labels_and_metadata() {
        let line = "- [ ] Task #backend #api [@owner: alice]";
        let parsed = CheckboxLine::parse(line, 1).unwrap();

        assert_eq!(parsed.title, "Task #backend #api");
        assert_eq!(parsed.labels.len(), 2);
    }

    // ===== Parse Edge Cases =====

    #[test]
    fn test_parse_title_with_extra_whitespace() {
        let line = "- [ ]   Task with spaces   ";
        let parsed = CheckboxLine::parse(line, 1).unwrap();

        assert_eq!(parsed.title, "Task with spaces");
    }

    #[test]
    fn test_parse_title_with_special_chars() {
        let line = "- [ ] Task: with (special) chars & symbols!";
        let parsed = CheckboxLine::parse(line, 1).unwrap();

        assert_eq!(parsed.title, "Task: with (special) chars & symbols!");
    }

    #[test]
    fn test_parse_title_with_brackets() {
        let line = "- [ ] Task [with brackets]";
        let parsed = CheckboxLine::parse(line, 1).unwrap();

        // Brackets without @ prefix are part of title
        assert_eq!(parsed.title, "Task [with brackets]");
    }

    #[test]
    fn test_parse_long_title() {
        let long_title = "a".repeat(500);
        let line = format!("- [ ] {long_title}");
        let parsed = CheckboxLine::parse(&line, 1).unwrap();

        assert_eq!(parsed.title, long_title);
    }

    // ===== Invalid Patterns (Should Return None) =====

    #[test]
    fn test_parse_empty_line() {
        let line = "";
        assert!(CheckboxLine::parse(line, 1).is_none());
    }

    #[test]
    fn test_parse_whitespace_only() {
        let line = "    ";
        assert!(CheckboxLine::parse(line, 1).is_none());
    }

    #[test]
    fn test_parse_no_checkbox() {
        let line = "Just regular text";
        assert!(CheckboxLine::parse(line, 1).is_none());
    }

    #[test]
    fn test_parse_incomplete_checkbox() {
        let line = "- [ ] ";
        assert!(CheckboxLine::parse(line, 1).is_none());
    }

    #[test]
    fn test_parse_empty_title() {
        let line = "- [ ]";
        assert!(CheckboxLine::parse(line, 1).is_none());
    }

    #[test]
    fn test_parse_missing_space_after_checkbox() {
        let line = "- [ ]No space";
        let parsed = CheckboxLine::parse(line, 1).unwrap();
        assert_eq!(parsed.title, "No space");
    }

    #[test]
    fn test_parse_invalid_status_char() {
        let line = "- [?] Invalid status";
        assert!(CheckboxLine::parse(line, 1).is_none());
    }

    #[test]
    fn test_parse_no_closing_bracket() {
        let line = "- [ Invalid";
        assert!(CheckboxLine::parse(line, 1).is_none());
    }

    #[test]
    fn test_parse_wrong_bullet_format() {
        let line = "* [ ] Wrong bullet";
        assert!(CheckboxLine::parse(line, 1).is_none());
    }

    #[test]
    fn test_parse_tab_indentation() {
        let line = "\t- [ ] Tabbed task";
        assert!(CheckboxLine::parse(line, 1).is_none());
    }

    #[test]
    fn test_parse_mixed_tabs_spaces() {
        let line = " \t- [ ] Mixed indentation";
        assert!(CheckboxLine::parse(line, 1).is_none());
    }

    #[test]
    fn test_parse_odd_indent() {
        let line = "   - [ ] Odd indent";
        let parsed = CheckboxLine::parse(line, 1).unwrap();

        // Parsing succeeds, but validation will catch odd indent
        assert_eq!(parsed.indent, 3);
        assert!(!parsed.has_valid_indentation());
    }

    // ===== Helper Function Tests =====

    #[test]
    fn test_count_leading_spaces_none() {
        assert_eq!(count_leading_spaces("text"), Some(0));
    }

    #[test]
    fn test_count_leading_spaces_some() {
        assert_eq!(count_leading_spaces("  text"), Some(2));
        assert_eq!(count_leading_spaces("    text"), Some(4));
        assert_eq!(count_leading_spaces("      text"), Some(6));
    }

    #[test]
    fn test_count_leading_spaces_tab() {
        assert!(count_leading_spaces("\ttext").is_none());
    }

    #[test]
    fn test_count_leading_spaces_mixed() {
        assert!(count_leading_spaces(" \ttext").is_none());
    }

    #[test]
    fn test_count_leading_spaces_empty() {
        assert!(count_leading_spaces("").is_none());
    }

    #[test]
    fn test_count_leading_spaces_whitespace_only() {
        assert!(count_leading_spaces("    ").is_none());
    }

    #[test]
    fn test_extract_trailing_metadata_none() {
        let (title, metadata) = extract_trailing_metadata("Simple task");
        assert_eq!(title, "Simple task");
        assert!(metadata.is_none());
    }

    #[test]
    fn test_extract_trailing_metadata_single() {
        let (title, metadata) = extract_trailing_metadata("Task [@owner: alice]");
        assert_eq!(title, "Task");
        assert_eq!(metadata, Some("@owner: alice"));
    }

    #[test]
    fn test_extract_trailing_metadata_multiple() {
        let (title, metadata) = extract_trailing_metadata("Task [@owner: alice, @estimate: 2h]");
        assert_eq!(title, "Task");
        assert_eq!(metadata, Some("@owner: alice, @estimate: 2h"));
    }

    #[test]
    fn test_extract_trailing_metadata_non_metadata_brackets() {
        let (title, metadata) = extract_trailing_metadata("Task [with brackets]");
        // Not metadata because doesn't start with @
        assert_eq!(title, "Task [with brackets]");
        assert!(metadata.is_none());
    }

    #[test]
    fn test_extract_trailing_metadata_with_labels() {
        let (title, metadata) = extract_trailing_metadata("Task #label [@owner: alice]");
        assert_eq!(title, "Task #label");
        assert_eq!(metadata, Some("@owner: alice"));
    }

    #[test]
    fn test_parse_inline_labels_none() {
        let labels = parse_inline_labels("Task without labels");
        assert_eq!(labels.len(), 0);
    }

    #[test]
    fn test_parse_inline_labels_single() {
        let labels = parse_inline_labels("Task #backend");
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].name, "backend");
    }

    #[test]
    fn test_parse_inline_labels_multiple() {
        let labels = parse_inline_labels("Task #backend #api #database");
        assert_eq!(labels.len(), 3);
    }

    // ===== Additional Edge Cases =====

    #[test]
    fn test_parse_multiple_status_chars() {
        let line = "- [xx] Multiple chars in brackets";
        assert!(CheckboxLine::parse(line, 1).is_none());
    }

    #[test]
    fn test_parse_checkbox_in_middle() {
        let line = "Some text - [ ] checkbox";
        assert!(CheckboxLine::parse(line, 1).is_none());
    }

    #[test]
    fn test_parse_line_number_tracking() {
        let line = "- [ ] Task";
        let parsed = CheckboxLine::parse(line, 42).unwrap();
        assert_eq!(parsed.line_num, 42);
    }

    #[test]
    fn test_parse_column_tracking() {
        let line = "    - [ ] Indented task";
        let parsed = CheckboxLine::parse(line, 1).unwrap();
        // Column is indent + 1 (position of dash)
        assert_eq!(parsed.column, 5);
    }

    #[test]
    fn test_parse_title_with_markdown() {
        let line = "- [ ] Task with **bold** and *italic* text";
        let parsed = CheckboxLine::parse(line, 1).unwrap();
        assert_eq!(parsed.title, "Task with **bold** and *italic* text");
    }

    #[test]
    fn test_parse_title_with_links() {
        let line = "- [ ] Task with [link](url)";
        let parsed = CheckboxLine::parse(line, 1).unwrap();
        assert_eq!(parsed.title, "Task with [link](url)");
    }

    #[test]
    fn test_parse_title_with_code() {
        let line = "- [ ] Implement `function()`";
        let parsed = CheckboxLine::parse(line, 1).unwrap();
        assert_eq!(parsed.title, "Implement `function()`");
    }
}
