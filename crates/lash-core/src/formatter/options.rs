//! Configuration options for the formatter

use serde::{Deserialize, Serialize};

/// Options for controlling formatter behavior
///
/// These options allow customization of formatting preferences while
/// maintaining consistency with Lash conventions.
///
/// # Default Values
///
/// - `indent_spaces`: 2
/// - `sort_annotations`: true
/// - `normalize_whitespace`: true
/// - `apply_auto_fixes`: true
/// - `preserve_blank_lines`: true (max 2)
///
/// # Example
///
/// ```rust
/// use lash_core::formatter::FormatOptions;
///
/// let options = FormatOptions {
///     indent_spaces: 2,
///     sort_annotations: true,
///     normalize_whitespace: true,
///     apply_auto_fixes: true,
///     preserve_blank_lines: true,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // Bools represent independent formatting options
pub struct FormatOptions {
    /// Number of spaces per indentation level
    ///
    /// Default: 2
    ///
    /// This controls how many spaces are used for each level of task nesting.
    /// The Lash standard is 2 spaces, but this can be customized if needed.
    pub indent_spaces: u8,

    /// Sort annotations alphabetically (except @id which is always first)
    ///
    /// Default: true
    ///
    /// When enabled, annotations in the header are sorted alphabetically by
    /// key name. The @id annotation is always first, followed by the rest
    /// in alphabetical order. This improves consistency and readability.
    pub sort_annotations: bool,

    /// Normalize whitespace (trailing, blank lines)
    ///
    /// Default: true
    ///
    /// When enabled, the formatter:
    /// - Removes trailing whitespace from all lines
    /// - Collapses multiple blank lines to maximum 2
    /// - Ensures single blank line between sections
    /// - Ensures file ends with single newline
    pub normalize_whitespace: bool,

    /// Apply auto-fixes from linter rules
    ///
    /// Default: true
    ///
    /// When enabled, the formatter applies automatic fixes for:
    /// - Auto-waiving children when parent is waived
    /// - Fixing parent-child status consistency
    /// - Other semantic fixes from linter rules
    pub apply_auto_fixes: bool,

    /// Preserve blank lines (up to maximum of 2)
    ///
    /// Default: true
    ///
    /// When enabled, blank lines are preserved up to a maximum of 2.
    /// When disabled, all multiple blank lines are collapsed to 1.
    ///
    /// This only applies when `normalize_whitespace` is true.
    pub preserve_blank_lines: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            indent_spaces: 2,
            sort_annotations: true,
            normalize_whitespace: true,
            apply_auto_fixes: true,
            preserve_blank_lines: true,
        }
    }
}

impl FormatOptions {
    /// Create a new `FormatOptions` with default values
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a minimal formatter (no auto-fixes, no sorting)
    ///
    /// This creates a formatter that only normalizes indentation and
    /// whitespace, without applying any semantic fixes or reordering.
    #[must_use]
    pub fn minimal() -> Self {
        Self {
            indent_spaces: 2,
            sort_annotations: false,
            normalize_whitespace: true,
            apply_auto_fixes: false,
            preserve_blank_lines: true,
        }
    }

    /// Create a strict formatter (all options enabled)
    ///
    /// This creates a formatter that applies all formatting rules and
    /// auto-fixes for maximum consistency and correctness.
    #[must_use]
    pub fn strict() -> Self {
        Self {
            indent_spaces: 2,
            sort_annotations: true,
            normalize_whitespace: true,
            apply_auto_fixes: true,
            preserve_blank_lines: false, // Strict: max 1 blank line
        }
    }

    /// Set the number of indent spaces
    #[must_use]
    pub fn with_indent_spaces(mut self, spaces: u8) -> Self {
        self.indent_spaces = spaces;
        self
    }

    /// Set whether to sort annotations
    #[must_use]
    pub fn with_sort_annotations(mut self, sort: bool) -> Self {
        self.sort_annotations = sort;
        self
    }

    /// Set whether to normalize whitespace
    #[must_use]
    pub fn with_normalize_whitespace(mut self, normalize: bool) -> Self {
        self.normalize_whitespace = normalize;
        self
    }

    /// Set whether to apply auto-fixes
    #[must_use]
    pub fn with_apply_auto_fixes(mut self, apply: bool) -> Self {
        self.apply_auto_fixes = apply;
        self
    }

    /// Set whether to preserve blank lines
    #[must_use]
    pub fn with_preserve_blank_lines(mut self, preserve: bool) -> Self {
        self.preserve_blank_lines = preserve;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let options = FormatOptions::default();
        assert_eq!(options.indent_spaces, 2);
        assert!(options.sort_annotations);
        assert!(options.normalize_whitespace);
        assert!(options.apply_auto_fixes);
        assert!(options.preserve_blank_lines);
    }

    #[test]
    fn test_minimal_options() {
        let options = FormatOptions::minimal();
        assert_eq!(options.indent_spaces, 2);
        assert!(!options.sort_annotations);
        assert!(options.normalize_whitespace);
        assert!(!options.apply_auto_fixes);
        assert!(options.preserve_blank_lines);
    }

    #[test]
    fn test_strict_options() {
        let options = FormatOptions::strict();
        assert_eq!(options.indent_spaces, 2);
        assert!(options.sort_annotations);
        assert!(options.normalize_whitespace);
        assert!(options.apply_auto_fixes);
        assert!(!options.preserve_blank_lines);
    }

    #[test]
    fn test_builder_pattern() {
        let options = FormatOptions::new()
            .with_indent_spaces(4)
            .with_sort_annotations(false)
            .with_apply_auto_fixes(false);

        assert_eq!(options.indent_spaces, 4);
        assert!(!options.sort_annotations);
        assert!(!options.apply_auto_fixes);
        assert!(options.normalize_whitespace); // Still default
    }

    #[test]
    fn test_serialization() {
        let options = FormatOptions::default();
        let json = serde_json::to_string(&options).unwrap();
        let deserialized: FormatOptions = serde_json::from_str(&json).unwrap();
        assert_eq!(options, deserialized);
    }
}
