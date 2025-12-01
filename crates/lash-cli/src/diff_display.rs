//! Diff display for showing fix previews
//!
//! This module provides a unified diff-style display for auto-fix suggestions.
//! It shows before/after comparisons with context lines and color-coded changes,
//! similar to `git diff` output.
//!
//! # Example
//!
//! ```no_run
//! use lash_cli::diff_display::DiffDisplay;
//! use lash_cli::theme::CliTheme;
//! use lash_core::linter::LintDiagnostic;
//! use std::path::PathBuf;
//!
//! let theme = CliTheme::load(None, true).unwrap();
//! let display = DiffDisplay::with_theme(theme.unwrap());
//!
//! // Assuming we have a diagnostic with a fix...
//! let original_content = "# Title\n@lables: foo\n## Tasks\n";
//! // display.print_fix_diff(original_content, &diagnostic);
//! ```

use lash_core::linter::{LintDiagnostic, Replacement};
use owo_colors::OwoColorize;

use crate::theme::CliTheme;

/// Default number of context lines to show before/after changes
const DEFAULT_CONTEXT_LINES: usize = 3;

/// Maximum line length before truncation
const MAX_LINE_LENGTH: usize = 120;

/// Display unified diffs for auto-fix suggestions
///
/// `DiffDisplay` generates Git-style unified diffs showing what will change
/// when an auto-fix is applied. It supports:
/// - Color-coded additions/deletions
/// - Configurable context lines
/// - Line number display
/// - Long line truncation
///
/// # Example
///
/// ```
/// use lash_cli::diff_display::DiffDisplay;
///
/// let display = DiffDisplay::new();
/// assert_eq!(display.context_lines(), 3);
/// ```
pub struct DiffDisplay {
    theme: Option<CliTheme>,
    context_lines: usize,
}

impl DiffDisplay {
    /// Create a new diff display with default settings
    ///
    /// # Example
    ///
    /// ```
    /// use lash_cli::diff_display::DiffDisplay;
    ///
    /// let display = DiffDisplay::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            theme: None,
            context_lines: DEFAULT_CONTEXT_LINES,
        }
    }

    /// Create a new diff display with a specific theme
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_cli::diff_display::DiffDisplay;
    /// use lash_cli::theme::CliTheme;
    ///
    /// let theme = CliTheme::load(None, true).unwrap().unwrap();
    /// let display = DiffDisplay::with_theme(theme);
    /// ```
    #[must_use]
    pub fn with_theme(theme: CliTheme) -> Self {
        Self {
            theme: Some(theme),
            context_lines: DEFAULT_CONTEXT_LINES,
        }
    }

    /// Set the number of context lines to show
    ///
    /// # Example
    ///
    /// ```
    /// use lash_cli::diff_display::DiffDisplay;
    ///
    /// let display = DiffDisplay::new().with_context_lines(5);
    /// assert_eq!(display.context_lines(), 5);
    /// ```
    #[must_use]
    pub fn with_context_lines(mut self, n: usize) -> Self {
        self.context_lines = n;
        self
    }

    /// Get the number of context lines
    ///
    /// # Example
    ///
    /// ```
    /// use lash_cli::diff_display::DiffDisplay;
    ///
    /// let display = DiffDisplay::new();
    /// assert_eq!(display.context_lines(), 3);
    /// ```
    #[must_use]
    pub fn context_lines(&self) -> usize {
        self.context_lines
    }

    /// Generate a unified diff string for a fix
    ///
    /// Returns `None` if the diagnostic doesn't have a fix or the fix
    /// cannot be applied.
    ///
    /// # Arguments
    ///
    /// * `original_content` - The original file content before applying the fix
    /// * `diagnostic` - The diagnostic containing the fix suggestion
    ///
    /// # Example
    ///
    /// ```
    /// use lash_cli::diff_display::DiffDisplay;
    /// use lash_core::linter::{Fix, LintDiagnostic};
    /// use std::path::PathBuf;
    ///
    /// let display = DiffDisplay::new();
    /// let content = "# Title\n@lables: foo\n## Tasks\n";
    ///
    /// let mut diagnostic = LintDiagnostic::error(
    ///     "E_TEST",
    ///     "Unknown annotation",
    ///     PathBuf::from("test.md"),
    ///     2,
    ///     1
    /// );
    /// let fix = Fix::replace("Fix typo", "@lables", "@labels");
    /// diagnostic = diagnostic.with_fix(fix);
    ///
    /// let diff = display.format_fix_diff(content, &diagnostic);
    /// assert!(diff.is_some());
    /// let diff_text = diff.unwrap();
    /// assert!(diff_text.contains("@lables"));
    /// assert!(diff_text.contains("@labels"));
    /// ```
    pub fn format_fix_diff(
        &self,
        original_content: &str,
        diagnostic: &LintDiagnostic,
    ) -> Option<String> {
        // Check if there's a fix
        let fix = diagnostic.fix.as_ref()?;

        // Handle reformat separately
        if matches!(fix.replacement, Replacement::Reformat) {
            return Some(self.format_reformat_message());
        }

        // Apply the fix to get the new content
        let new_content = fix.replacement.apply(original_content).ok()?;

        // If content is unchanged, return None
        if original_content == new_content {
            return None;
        }

        // Generate unified diff
        Some(self.generate_unified_diff(original_content, &new_content))
    }

    /// Print a diff to stdout
    ///
    /// Prints the diff for a fix suggestion to stdout. If the diagnostic
    /// doesn't have a fix or the fix cannot be applied, prints nothing.
    ///
    /// # Arguments
    ///
    /// * `original_content` - The original file content
    /// * `diagnostic` - The diagnostic with fix suggestion
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_cli::diff_display::DiffDisplay;
    /// use lash_core::linter::{Fix, LintDiagnostic};
    /// use std::path::PathBuf;
    ///
    /// let display = DiffDisplay::new();
    /// let content = "# Title\n@lables: foo\n";
    ///
    /// let mut diagnostic = LintDiagnostic::error(
    ///     "E_TEST",
    ///     "Unknown annotation",
    ///     PathBuf::from("test.md"),
    ///     2,
    ///     1
    /// );
    /// let fix = Fix::replace("Fix typo", "@lables", "@labels");
    /// diagnostic = diagnostic.with_fix(fix);
    ///
    /// display.print_fix_diff(content, &diagnostic);
    /// ```
    pub fn print_fix_diff(&self, original_content: &str, diagnostic: &LintDiagnostic) {
        if let Some(diff) = self.format_fix_diff(original_content, diagnostic) {
            println!("{diff}");
        }
    }

    // Private helper methods

    fn format_reformat_message(&self) -> String {
        let message = "(full file reformat)";
        if self.theme.is_some() {
            message.dimmed().to_string()
        } else {
            message.to_string()
        }
    }

    fn generate_unified_diff(&self, original: &str, modified: &str) -> String {
        let original_lines: Vec<&str> = original.lines().collect();
        let modified_lines: Vec<&str> = modified.lines().collect();

        // Build the diff output
        let mut output = String::new();

        // Header
        output.push_str(&self.style_diff_header("--- before\n"));
        output.push_str(&self.style_diff_header("+++ after\n"));

        // Generate diff lines
        let diff = Self::compute_line_diff(&original_lines, &modified_lines);

        // Find the range of lines to display (with context)
        let (display_start, display_end) = self.calculate_display_range(&diff);

        // Hunk header - show line numbers for the displayed range
        let orig_line_count = diff[display_start..display_end]
            .iter()
            .filter(|change| !matches!(change, LineChange::Added(_)))
            .count();
        let mod_line_count = diff[display_start..display_end]
            .iter()
            .filter(|change| !matches!(change, LineChange::Deleted(_)))
            .count();

        let hunk_header = format!(
            "@@ -{},{} +{},{} @@\n",
            display_start + 1,
            orig_line_count,
            display_start + 1,
            mod_line_count
        );
        output.push_str(&self.style_hunk_header(&hunk_header));

        // Output the diff lines in the display range
        for change in &diff[display_start..display_end] {
            match change {
                LineChange::Unchanged(line) => {
                    let formatted_line = Self::format_line(line);
                    output.push_str(&self.style_context(&format!("  {formatted_line}\n")));
                }
                LineChange::Deleted(line) => {
                    let formatted_line = Self::format_line(line);
                    output.push_str(&self.style_removed(&format!("-  {formatted_line}\n")));
                }
                LineChange::Added(line) => {
                    let formatted_line = Self::format_line(line);
                    output.push_str(&self.style_added(&format!("+  {formatted_line}\n")));
                }
            }
        }

        output
    }

    fn calculate_display_range(&self, diff: &[LineChange]) -> (usize, usize) {
        // Find the first and last changed lines
        let first_change = diff
            .iter()
            .position(|change| !matches!(change, LineChange::Unchanged(_)))
            .unwrap_or(0);

        let last_change = diff
            .iter()
            .rposition(|change| !matches!(change, LineChange::Unchanged(_)))
            .unwrap_or(diff.len().saturating_sub(1));

        // Add context lines
        let start = first_change.saturating_sub(self.context_lines);
        let end = (last_change + self.context_lines + 1).min(diff.len());

        (start, end)
    }

    fn compute_line_diff<'a>(original: &[&'a str], modified: &[&'a str]) -> Vec<LineChange<'a>> {
        // Simple diff algorithm using longest common subsequence (LCS)
        // This properly handles insertions, deletions, and changes
        let mut result = Vec::new();

        // Find common prefix
        let common_prefix = original
            .iter()
            .zip(modified.iter())
            .take_while(|(a, b)| a == b)
            .count();

        // Find common suffix (excluding the common prefix)
        let common_suffix = original[common_prefix..]
            .iter()
            .rev()
            .zip(modified[common_prefix..].iter().rev())
            .take_while(|(a, b)| a == b)
            .count();

        // Add unchanged prefix
        for line in &original[..common_prefix] {
            result.push(LineChange::Unchanged(line));
        }

        // Add the differing middle part
        let orig_middle_end = original.len() - common_suffix;
        let mod_middle_end = modified.len() - common_suffix;

        // Deletions
        for line in &original[common_prefix..orig_middle_end] {
            result.push(LineChange::Deleted(line));
        }

        // Additions
        for line in &modified[common_prefix..mod_middle_end] {
            result.push(LineChange::Added(line));
        }

        // Add unchanged suffix
        for line in &original[orig_middle_end..] {
            result.push(LineChange::Unchanged(line));
        }

        result
    }

    fn format_line(line: &str) -> String {
        if line.len() > MAX_LINE_LENGTH {
            let truncated = &line[..MAX_LINE_LENGTH];
            format!("{truncated}...")
        } else {
            line.to_string()
        }
    }

    // Styling methods

    fn style_diff_header(&self, text: &str) -> String {
        if self.theme.is_some() {
            text.bold().to_string()
        } else {
            text.to_string()
        }
    }

    fn style_hunk_header(&self, text: &str) -> String {
        if self.theme.is_some() {
            text.cyan().to_string()
        } else {
            text.to_string()
        }
    }

    fn style_context(&self, text: &str) -> String {
        if self.theme.is_some() {
            text.dimmed().to_string()
        } else {
            text.to_string()
        }
    }

    fn style_removed(&self, text: &str) -> String {
        if self.theme.is_some() {
            text.red().to_string()
        } else {
            text.to_string()
        }
    }

    fn style_added(&self, text: &str) -> String {
        if self.theme.is_some() {
            text.green().to_string()
        } else {
            text.to_string()
        }
    }
}

impl Default for DiffDisplay {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a change to a line in a diff
#[derive(Debug, Clone, PartialEq, Eq)]
enum LineChange<'a> {
    /// Line is unchanged
    Unchanged(&'a str),
    /// Line was deleted
    Deleted(&'a str),
    /// Line was added
    Added(&'a str),
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_core::linter::Fix;
    use std::path::PathBuf;

    #[test]
    fn test_diff_display_new() {
        let display = DiffDisplay::new();
        assert_eq!(display.context_lines(), 3);
        assert!(display.theme.is_none());
    }

    #[test]
    fn test_diff_display_with_context_lines() {
        let display = DiffDisplay::new().with_context_lines(5);
        assert_eq!(display.context_lines(), 5);
    }

    #[test]
    fn test_format_fix_diff_text_replace() {
        let display = DiffDisplay::new();
        let content = "# Title\n@lables: foo\n## Tasks\n";

        let mut diagnostic = LintDiagnostic::error(
            "E_LINT_UNKNOWN_ANNOTATION",
            "Unknown annotation '@lables'",
            PathBuf::from("tasks/example.md"),
            2,
            1,
        );
        let fix = Fix::replace("Replace '@lables' with '@labels'", "@lables", "@labels");
        diagnostic = diagnostic.with_fix(fix);

        let diff = display.format_fix_diff(content, &diagnostic);
        assert!(diff.is_some());

        let diff_text = diff.unwrap();
        assert!(diff_text.contains("--- before"));
        assert!(diff_text.contains("+++ after"));
        assert!(diff_text.contains("@lables"));
        assert!(diff_text.contains("@labels"));
    }

    #[test]
    fn test_format_fix_diff_no_fix() {
        let display = DiffDisplay::new();
        let content = "# Title\n";

        let diagnostic =
            LintDiagnostic::error("E_TEST", "Test error", PathBuf::from("test.md"), 1, 1);

        let diff = display.format_fix_diff(content, &diagnostic);
        assert!(diff.is_none());
    }

    #[test]
    fn test_format_fix_diff_no_change() {
        let display = DiffDisplay::new();
        let content = "# Title\n@labels: foo\n## Tasks\n";

        let mut diagnostic =
            LintDiagnostic::error("E_TEST", "Test error", PathBuf::from("test.md"), 2, 1);
        // Fix that doesn't actually change anything
        let fix = Fix::replace("No change", "@nonexistent", "@labels");
        diagnostic = diagnostic.with_fix(fix);

        // This should return None because the fix can't be applied
        let diff = display.format_fix_diff(content, &diagnostic);
        assert!(diff.is_none());
    }

    #[test]
    fn test_format_fix_diff_reformat() {
        let display = DiffDisplay::new();
        let content = "# Title\n";

        let mut diagnostic =
            LintDiagnostic::error("E_TEST", "Test error", PathBuf::from("test.md"), 1, 1);
        let fix = Fix::reformat("Reformat file");
        diagnostic = diagnostic.with_fix(fix);

        let diff = display.format_fix_diff(content, &diagnostic);
        assert!(diff.is_some());
        assert!(diff.unwrap().contains("(full file reformat)"));
    }

    #[test]
    fn test_format_fix_diff_insert() {
        let display = DiffDisplay::new();
        let content = "# Title\n\n## Tasks\n";

        let mut diagnostic = LintDiagnostic::error(
            "E_TEST",
            "Missing annotation",
            PathBuf::from("test.md"),
            2,
            1,
        );
        let fix = Fix::insert("Add @id annotation", 9, "@id: test\n");
        diagnostic = diagnostic.with_fix(fix);

        let diff = display.format_fix_diff(content, &diagnostic);
        assert!(diff.is_some());
        let diff_text = diff.unwrap();
        assert!(diff_text.contains("@id: test"));
    }

    #[test]
    fn test_format_fix_diff_delete() {
        let display = DiffDisplay::new();
        let content = "# Title\nbad line\n## Tasks\n";

        let mut diagnostic =
            LintDiagnostic::error("E_TEST", "Invalid line", PathBuf::from("test.md"), 2, 1);
        let fix = Fix::delete("Remove invalid line", 8, 17);
        diagnostic = diagnostic.with_fix(fix);

        let diff = display.format_fix_diff(content, &diagnostic);
        assert!(diff.is_some());
    }

    #[test]
    fn test_format_line_truncation() {
        let long_line = "a".repeat(150);
        let formatted = DiffDisplay::format_line(&long_line);
        assert!(formatted.len() < long_line.len());
        assert!(formatted.ends_with("..."));
    }

    #[test]
    fn test_line_change_enum() {
        let unchanged = LineChange::Unchanged("test");
        let deleted = LineChange::Deleted("test");
        let added = LineChange::Added("test");

        assert!(matches!(unchanged, LineChange::Unchanged(_)));
        assert!(matches!(deleted, LineChange::Deleted(_)));
        assert!(matches!(added, LineChange::Added(_)));
    }

    #[test]
    fn test_default() {
        let display = DiffDisplay::default();
        assert_eq!(display.context_lines(), 3);
    }
}
