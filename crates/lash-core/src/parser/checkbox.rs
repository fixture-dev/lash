//! Checkbox line parsing
//!
//! This module handles parsing of individual checkbox task lines, including:
//! - Indentation detection and validation
//! - Status character extraction (` `, `x`, `-`, `!`)
//! - Task title extraction
//! - Inline label parsing (#tag)
//!
//! Additionally, this module handles parsing of plain bullet lines (contextual notes),
//! which are non-checkbox bullet points that provide additional context for tasks.
//!
//! Checkbox lines are the core building blocks of task lists. Each line is
//! parsed into a `CheckboxLine` structure that captures all the information
//! needed to build the task tree.

use lash_types::{Label, TaskStatus};

use super::annotations::AnnotationBlock;

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

    /// Annotation block following the task line (if any)
    pub annotations: Option<AnnotationBlock>,
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
            annotations: None,
        }
    }

    /// Detect if a line looks like a malformed checkbox
    ///
    /// Returns an error message if the line appears to be attempting to be a
    /// checkbox but has invalid syntax. Returns `None` if the line is not
    /// checkbox-like at all.
    ///
    /// This function is conservative and only reports errors for checkboxes that
    /// are very close to being valid but have an invalid status character.
    /// Other malformed patterns (missing brackets, wrong spacing, etc.) are
    /// silently ignored as they could be regular markdown content.
    ///
    /// # Arguments
    ///
    /// * `line` - The line to check
    ///
    /// # Returns
    ///
    /// Returns `Some(error_message)` if the line is malformed, `None` if it's
    /// not checkbox-like.
    #[must_use]
    pub fn detect_malformed(line: &str) -> Option<String> {
        // Skip empty lines and pure whitespace
        if line.trim().is_empty() {
            return None;
        }

        // Count leading spaces (ignore lines with tabs)
        let indent = count_leading_spaces(line)?;
        let rest = &line[indent..];

        // First check if this looks like a checkbox attempt (starts with dash,
        // has brackets somewhere). This catches malformed checkboxes with extra spacing.
        if !rest.starts_with('-') {
            return None;
        }

        // Find opening and closing brackets
        let open_bracket_pos = rest.find('[')?;
        let close_bracket_pos = rest.find(']')?;

        // Must have brackets in reasonable positions (within first 10 chars)
        // and closing must be after opening
        if open_bracket_pos > 8 || close_bracket_pos <= open_bracket_pos {
            return None;
        }

        // Check if this is a Markdown link: "- [Link Text](url)"
        // If the character after the closing bracket is '(', it's a link, not a checkbox
        let after_close = rest.get(close_bracket_pos + 1..close_bracket_pos + 2);
        if after_close == Some("(") {
            return None;
        }

        // Now check for specific malformed patterns

        // Pattern 1: Extra space(s) between dash and bracket: "-  [" or "-   ["
        // Valid format requires exactly "- [" (dash, single space, bracket)
        if open_bracket_pos != 2 {
            // Check there's actually content after to distinguish from random markdown
            let content_start = close_bracket_pos + 1;
            if rest.len() > content_start && !rest[content_start..].trim().is_empty() {
                let extra_chars = &rest[2..=open_bracket_pos];
                return Some(format!(
                    "Malformed checkbox: extra space between '-' and '['. Expected '- [' but found '- {extra_chars}'"
                ));
            }
            return None;
        }

        // Pattern 2: Wrong number of characters in brackets: "- [  ]" or "- [xx]"
        // Valid format has exactly 1 character between [ and ]
        let bracket_content_len = close_bracket_pos - open_bracket_pos - 1;
        if bracket_content_len != 1 {
            let content_start = close_bracket_pos + 1;
            if rest.len() > content_start && !rest[content_start..].trim().is_empty() {
                let bracket_content = &rest[open_bracket_pos + 1..close_bracket_pos];
                return Some(format!(
                    "Malformed checkbox: expected single character in brackets but found '{bracket_content}'. Valid formats: [ ], [x], [X], [-], [!]"
                ));
            }
            return None;
        }

        // Extract status character (at position 3 for "- [X]")
        let status_char = rest.chars().nth(3)?;

        // Pattern 3: Invalid status character
        if lash_types::TaskStatus::from_checkbox_char(status_char).is_err() {
            // Check if there's text after it (looks like a real checkbox attempt)
            let after_bracket = 5; // Length of "- [X]"
            if rest.len() > after_bracket && !rest[after_bracket..].trim().is_empty() {
                return Some(format!(
                    "Invalid checkbox status '{status_char}': expected one of ' ' (space), 'x', 'X', '-', or '!'"
                ));
            }
        }

        // All other patterns are ignored
        None
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
    /// ```
    /// use lash_core::parser::checkbox::CheckboxLine;
    /// use lash_types::TaskStatus;
    ///
    /// let line = "  - [x] Complete task #backend";
    /// let parsed = CheckboxLine::parse(line, 5).unwrap();
    /// assert_eq!(parsed.depth, 1); // 2 spaces = depth 1
    /// assert_eq!(parsed.status, TaskStatus::Done);
    /// assert_eq!(parsed.title, "Complete task #backend");
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

/// A plain bullet line (contextual note)
///
/// Plain bullet lines are non-checkbox bullet points that provide additional
/// context, requirements, or acceptance criteria for tasks. They are parsed
/// separately from checkbox lines and attached to their parent tasks.
///
/// # Example
///
/// ```
/// use lash_core::parser::checkbox::PlainBulletLine;
///
/// let line = "  - Use library X for parsing";
/// let parsed = PlainBulletLine::parse(line, 5).unwrap();
/// assert_eq!(parsed.depth, 1);
/// assert_eq!(parsed.text, "Use library X for parsing");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlainBulletLine {
    /// Number of leading spaces (indentation)
    pub indent: usize,

    /// Computed nesting depth (indent / 2)
    pub depth: u8,

    /// Note text (after "- ")
    pub text: String,

    /// Line number in source file (1-indexed)
    pub line_num: usize,

    /// Column number where bullet starts (1-indexed)
    pub column: usize,
}

impl PlainBulletLine {
    /// Parse a plain bullet line from a string
    ///
    /// This function parses lines that start with "- " but are NOT:
    /// - Checkbox lines (e.g., "- [ ] task")
    /// - Markdown links (e.g., "- [Link Text](url)")
    ///
    /// # Arguments
    ///
    /// * `line` - The line to parse
    /// * `line_num` - Line number in source file (for error reporting)
    ///
    /// # Returns
    ///
    /// Returns `Some(PlainBulletLine)` if the line is a valid plain bullet,
    /// or `None` if it's a checkbox, markdown link, or not a bullet at all.
    ///
    /// # Example
    ///
    /// ```
    /// use lash_core::parser::checkbox::PlainBulletLine;
    ///
    /// // Valid plain bullet
    /// let parsed = PlainBulletLine::parse("  - Use library X", 1).unwrap();
    /// assert_eq!(parsed.text, "Use library X");
    ///
    /// // Checkbox - should return None
    /// assert!(PlainBulletLine::parse("- [ ] Task", 1).is_none());
    ///
    /// // Markdown link - should return None
    /// assert!(PlainBulletLine::parse("- [Link](url)", 1).is_none());
    /// ```
    #[must_use]
    pub fn parse(line: &str, line_num: usize) -> Option<Self> {
        // Count leading spaces (indentation)
        let indent = count_leading_spaces(line)?;

        // Get the rest of the line after indentation
        let rest = &line[indent..];

        // Must start with "- " (dash followed by space)
        if !rest.starts_with("- ") {
            return None;
        }

        // Check if this is a checkbox pattern: "- [" followed by something
        if rest.starts_with("- [") {
            return None;
        }

        // Check if this looks like a malformed checkbox (has "[" with checkbox-like chars)
        // This catches cases like "-  [ ] task" (extra space after dash)
        if rest.contains('[') && rest.contains(']') {
            // Check if there's a pattern that looks like a checkbox: [X] or [ ] etc.
            if let Some(open_bracket) = rest.find('[') {
                if let Some(close_bracket) = rest.find(']') {
                    // Check if bracket content looks like checkbox status
                    if close_bracket == open_bracket + 2 {
                        let bracket_content = rest
                            .get(open_bracket + 1..close_bracket)
                            .and_then(|s| s.chars().next());
                        if let Some(c) = bracket_content {
                            if c == ' ' || c == 'x' || c == 'X' || c == '-' || c == '!' {
                                // This looks like a malformed checkbox, not a plain bullet
                                return None;
                            }
                        }
                    }
                }
            }
        }

        // Check if this is a markdown link: "- [text](url)"
        // This is more complex - we need to find if there's a complete [text](url) pattern
        if is_markdown_link(rest) {
            return None;
        }

        // Extract text after "- "
        let text = rest[2..].trim();
        if text.is_empty() {
            return None;
        }

        // Column where bullet starts (after indentation, at the dash)
        let column = indent + 1;

        #[allow(clippy::cast_possible_truncation)] // Depth is limited by max_depth config
        let depth = (indent / 2) as u8;

        Some(Self {
            indent,
            depth,
            text: text.to_string(),
            line_num,
            column,
        })
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

    /// Check if this note can be a child of a checkbox line based on depth
    ///
    /// A note can be a child if its depth is exactly one more than the
    /// potential parent's depth.
    ///
    /// # Arguments
    ///
    /// * `potential_parent` - The potential parent checkbox line
    ///
    /// # Returns
    ///
    /// Returns `true` if this note can be a child of the potential parent.
    #[must_use]
    pub fn can_be_child_of(&self, potential_parent: &CheckboxLine) -> bool {
        self.depth == potential_parent.depth + 1
    }
}

/// Check if a line (after indentation) is a markdown link pattern
///
/// Detects patterns like "- [Link Text](url)" which should NOT be parsed
/// as plain bullets.
fn is_markdown_link(rest: &str) -> bool {
    // Pattern: "- [text](url)"
    if !rest.starts_with("- [") {
        return false;
    }

    // Find the closing bracket
    if let Some(close_bracket_pos) = rest.find(']') {
        // Check if the next character is '(' indicating a link
        if rest.get(close_bracket_pos + 1..close_bracket_pos + 2) == Some("(") {
            return true;
        }
    }

    false
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
/// ```
/// use lash_core::parser::checkbox::parse_inline_labels;
///
/// let labels = parse_inline_labels("Complete backend #api #database");
/// assert_eq!(labels.len(), 2);
/// // Labels are stored in a HashSet, so order is not guaranteed
/// let names: Vec<_> = labels.iter().map(|l| l.name.as_str()).collect();
/// assert!(names.contains(&"api"));
/// assert!(names.contains(&"database"));
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
    fn test_detect_malformed_invalid_status() {
        let line = "- [?] Invalid status";
        let error = CheckboxLine::detect_malformed(line);
        assert!(error.is_some());
        assert!(error.unwrap().contains("Invalid checkbox status '?'"));
    }

    #[test]
    fn test_detect_malformed_missing_closing_bracket() {
        let line = "- [ Missing bracket";
        let error = CheckboxLine::detect_malformed(line);
        // Conservative: missing bracket could be markdown, so we ignore it
        assert!(error.is_none());
    }

    #[test]
    fn test_detect_malformed_too_many_chars() {
        let line = "- [xx] Too many chars";
        let error = CheckboxLine::detect_malformed(line);
        // With content after brackets, this looks like a checkbox attempt and should error
        assert!(error.is_some());
        assert!(error
            .unwrap()
            .contains("expected single character in brackets"));
    }

    #[test]
    fn test_detect_malformed_too_many_chars_no_content() {
        // Without content after, could be markdown so we ignore
        let line = "- [xx]";
        let error = CheckboxLine::detect_malformed(line);
        assert!(error.is_none());
    }

    #[test]
    fn test_detect_malformed_missing_title() {
        let line = "- [ ]";
        let error = CheckboxLine::detect_malformed(line);
        // Conservative: missing title could be incomplete, so we ignore it
        assert!(error.is_none());
    }

    #[test]
    fn test_detect_malformed_valid_checkbox() {
        let line = "- [ ] Valid task";
        let error = CheckboxLine::detect_malformed(line);
        assert!(error.is_none());
    }

    #[test]
    fn test_detect_malformed_extra_space_after_dash() {
        let line = "-  [ ] Task with extra space";
        let error = CheckboxLine::detect_malformed(line);
        assert!(error.is_some());
        assert!(error.unwrap().contains("extra space between '-' and '['"));
    }

    #[test]
    fn test_detect_malformed_multiple_extra_spaces_after_dash() {
        let line = "-   [ ] Task with multiple spaces";
        let error = CheckboxLine::detect_malformed(line);
        assert!(error.is_some());
        assert!(error.unwrap().contains("extra space between '-' and '['"));
    }

    #[test]
    fn test_detect_malformed_extra_space_in_brackets() {
        let line = "- [  ] Task with extra space in brackets";
        let error = CheckboxLine::detect_malformed(line);
        assert!(error.is_some());
        assert!(error
            .unwrap()
            .contains("expected single character in brackets"));
    }

    #[test]
    fn test_detect_malformed_not_checkbox() {
        let line = "Just regular text";
        let error = CheckboxLine::detect_malformed(line);
        assert!(error.is_none());
    }

    #[test]
    fn test_detect_malformed_markdown_link() {
        // Markdown links should NOT be detected as malformed checkboxes
        let line = "- [Features](features/tasks.md)";
        let error = CheckboxLine::detect_malformed(line);
        assert!(error.is_none());
    }

    #[test]
    fn test_detect_malformed_markdown_link_with_spaces() {
        // Markdown links with multi-word text should NOT be detected as malformed
        let line = "- [My Feature List](features.md)";
        let error = CheckboxLine::detect_malformed(line);
        assert!(error.is_none());
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

    // ===== PlainBulletLine Tests =====

    #[test]
    fn test_plain_bullet_basic() {
        let line = "- Use library X for parsing";
        let parsed = PlainBulletLine::parse(line, 1).unwrap();

        assert_eq!(parsed.indent, 0);
        assert_eq!(parsed.depth, 0);
        assert_eq!(parsed.text, "Use library X for parsing");
        assert_eq!(parsed.line_num, 1);
        assert_eq!(parsed.column, 1);
    }

    #[test]
    fn test_plain_bullet_indented() {
        let line = "  - Target < 100ms latency";
        let parsed = PlainBulletLine::parse(line, 5).unwrap();

        assert_eq!(parsed.indent, 2);
        assert_eq!(parsed.depth, 1);
        assert_eq!(parsed.text, "Target < 100ms latency");
        assert_eq!(parsed.line_num, 5);
        assert_eq!(parsed.column, 3);
    }

    #[test]
    fn test_plain_bullet_deep_indent() {
        let line = "    - Deeply nested note";
        let parsed = PlainBulletLine::parse(line, 1).unwrap();

        assert_eq!(parsed.indent, 4);
        assert_eq!(parsed.depth, 2);
        assert_eq!(parsed.text, "Deeply nested note");
    }

    #[test]
    fn test_plain_bullet_with_extra_whitespace() {
        let line = "-   Note with extra whitespace  ";
        let parsed = PlainBulletLine::parse(line, 1).unwrap();

        assert_eq!(parsed.text, "Note with extra whitespace");
    }

    #[test]
    fn test_plain_bullet_not_checkbox_open() {
        let line = "- [ ] This is a checkbox";
        assert!(PlainBulletLine::parse(line, 1).is_none());
    }

    #[test]
    fn test_plain_bullet_not_checkbox_done() {
        let line = "- [x] Done checkbox";
        assert!(PlainBulletLine::parse(line, 1).is_none());
    }

    #[test]
    fn test_plain_bullet_not_checkbox_waived() {
        let line = "- [-] Waived checkbox";
        assert!(PlainBulletLine::parse(line, 1).is_none());
    }

    #[test]
    fn test_plain_bullet_not_checkbox_blocked() {
        let line = "- [!] Blocked checkbox";
        assert!(PlainBulletLine::parse(line, 1).is_none());
    }

    #[test]
    fn test_plain_bullet_not_markdown_link() {
        let line = "- [Link Text](https://example.com)";
        assert!(PlainBulletLine::parse(line, 1).is_none());
    }

    #[test]
    fn test_plain_bullet_not_markdown_link_multiword() {
        let line = "- [My Feature List](features.md)";
        assert!(PlainBulletLine::parse(line, 1).is_none());
    }

    #[test]
    fn test_plain_bullet_empty_text() {
        let line = "- ";
        assert!(PlainBulletLine::parse(line, 1).is_none());
    }

    #[test]
    fn test_plain_bullet_whitespace_only_text() {
        let line = "-    ";
        assert!(PlainBulletLine::parse(line, 1).is_none());
    }

    #[test]
    fn test_plain_bullet_not_bullet() {
        let line = "Just regular text";
        assert!(PlainBulletLine::parse(line, 1).is_none());
    }

    #[test]
    fn test_plain_bullet_star_bullet() {
        // Star bullets are not supported
        let line = "* Star bullet";
        assert!(PlainBulletLine::parse(line, 1).is_none());
    }

    #[test]
    fn test_plain_bullet_tab_indent() {
        let line = "\t- Tabbed note";
        assert!(PlainBulletLine::parse(line, 1).is_none());
    }

    #[test]
    fn test_plain_bullet_valid_indentation() {
        let valid = PlainBulletLine::parse("  - Note", 1).unwrap();
        assert!(valid.has_valid_indentation());

        let invalid_line = "   - Odd indent note";
        let invalid = PlainBulletLine::parse(invalid_line, 1).unwrap();
        assert!(!invalid.has_valid_indentation());
    }

    #[test]
    fn test_plain_bullet_can_be_child_of_checkbox() {
        let checkbox = CheckboxLine::parse("- [ ] Parent task", 1).unwrap();
        let note = PlainBulletLine::parse("  - Child note", 2).unwrap();

        assert!(note.can_be_child_of(&checkbox));
    }

    #[test]
    fn test_plain_bullet_not_child_same_depth() {
        let checkbox = CheckboxLine::parse("- [ ] Task", 1).unwrap();
        let note = PlainBulletLine::parse("- Sibling note", 2).unwrap();

        assert!(!note.can_be_child_of(&checkbox));
    }

    #[test]
    fn test_plain_bullet_not_child_too_deep() {
        let checkbox = CheckboxLine::parse("- [ ] Task", 1).unwrap();
        let note = PlainBulletLine::parse("    - Too deep note", 2).unwrap();

        assert!(!note.can_be_child_of(&checkbox));
    }

    #[test]
    fn test_plain_bullet_with_special_chars() {
        let line = "- Use foo, bar, and baz!";
        let parsed = PlainBulletLine::parse(line, 1).unwrap();
        assert_eq!(parsed.text, "Use foo, bar, and baz!");
    }

    #[test]
    fn test_plain_bullet_with_code() {
        let line = "- Call `process_data()` function";
        let parsed = PlainBulletLine::parse(line, 1).unwrap();
        assert_eq!(parsed.text, "Call `process_data()` function");
    }

    #[test]
    fn test_plain_bullet_with_brackets_not_link() {
        // Brackets without () after should be parsed as plain bullet
        let line = "- Use [this format] for data";
        let parsed = PlainBulletLine::parse(line, 1).unwrap();
        assert_eq!(parsed.text, "Use [this format] for data");
    }

    #[test]
    fn test_plain_bullet_not_malformed_checkbox() {
        // Malformed checkbox with extra space should NOT be parsed as plain bullet
        let line = "-  [ ] Task with extra space";
        assert!(PlainBulletLine::parse(line, 1).is_none());

        let line2 = "-   [x] Done with extra spaces";
        assert!(PlainBulletLine::parse(line2, 1).is_none());

        let line3 = "-  [-] Waived with extra space";
        assert!(PlainBulletLine::parse(line3, 1).is_none());
    }

    #[test]
    fn test_is_markdown_link_true() {
        assert!(is_markdown_link("- [Link](url)"));
        assert!(is_markdown_link("- [Multi Word](http://example.com)"));
        assert!(is_markdown_link("- [A](b)"));
    }

    #[test]
    fn test_is_markdown_link_false() {
        assert!(!is_markdown_link("- Just text"));
        assert!(!is_markdown_link("- [Brackets only]"));
        assert!(!is_markdown_link("- [ ] Checkbox"));
        assert!(!is_markdown_link("Some text"));
        // Note: "- [No closing paren](" is detected as a link because it has ](
        // This is intentional - we're conservative in excluding potential links
    }
}
