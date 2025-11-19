//! Auto-fix suggestions for linting diagnostics
//!
//! Fixes describe how to automatically correct linting issues. They can be:
//! - Text replacements
//! - Text insertions
//! - Text deletions
//! - Whole-file reformats

use serde::{Deserialize, Serialize};

/// Auto-fix suggestion for a diagnostic
///
/// Fixes describe what should be changed to resolve a linting issue.
/// They include both a human-readable description and a machine-executable
/// replacement operation.
///
/// # Example
///
/// ```
/// use lash_core::linter::{Fix, Replacement};
///
/// let fix = Fix {
///     description: "Replace with valid checkbox syntax".to_string(),
///     replacement: Replacement::TextReplace {
///         old: "- [*]".to_string(),
///         new: "- [ ]".to_string(),
///     },
/// };
/// assert_eq!(fix.description, "Replace with valid checkbox syntax");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Fix {
    /// Human-readable description of what this fix does
    pub description: String,

    /// The replacement operation to perform
    pub replacement: Replacement,
}

impl Fix {
    /// Create a new text replacement fix
    #[must_use]
    pub fn replace(
        description: impl Into<String>,
        old: impl Into<String>,
        new: impl Into<String>,
    ) -> Self {
        Self {
            description: description.into(),
            replacement: Replacement::TextReplace {
                old: old.into(),
                new: new.into(),
            },
        }
    }

    /// Create a new insertion fix
    #[must_use]
    pub fn insert(
        description: impl Into<String>,
        position: usize,
        text: impl Into<String>,
    ) -> Self {
        Self {
            description: description.into(),
            replacement: Replacement::Insert {
                position,
                text: text.into(),
            },
        }
    }

    /// Create a new deletion fix
    #[must_use]
    pub fn delete(description: impl Into<String>, start: usize, end: usize) -> Self {
        Self {
            description: description.into(),
            replacement: Replacement::Delete { start, end },
        }
    }

    /// Create a new whole-file reformat fix
    #[must_use]
    pub fn reformat(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            replacement: Replacement::Reformat,
        }
    }
}

/// Type of replacement operation for a fix
///
/// Replacements describe the specific edit operation to apply:
/// - `TextReplace`: Replace all occurrences of old text with new text
/// - `Insert`: Insert text at a specific position
/// - `Delete`: Delete text in a specific range
/// - `Reformat`: Reformat the entire file (via formatter)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Replacement {
    /// Replace all occurrences of old text with new text
    ///
    /// This is the most common type of fix for simple text corrections.
    /// The replacement is applied to all occurrences in the relevant scope
    /// (usually a single line or task).
    TextReplace {
        /// Text to find and replace
        old: String,
        /// Text to replace with
        new: String,
    },

    /// Insert text at a specific byte position
    ///
    /// Used for adding missing content (e.g., required annotations).
    /// The position is a byte offset from the start of the file.
    Insert {
        /// Byte position to insert at
        position: usize,
        /// Text to insert
        text: String,
    },

    /// Delete text in a specific byte range
    ///
    /// Used for removing invalid or redundant content.
    /// The range is specified as [start, end) byte offsets.
    Delete {
        /// Start byte position (inclusive)
        start: usize,
        /// End byte position (exclusive)
        end: usize,
    },

    /// Reformat the entire file
    ///
    /// This delegates to the formatter to apply all formatting rules.
    /// Used when multiple small fixes would be better handled by a
    /// comprehensive reformat.
    Reformat,
}

impl Replacement {
    /// Apply this replacement to the given content
    ///
    /// # Errors
    ///
    /// Returns error if the replacement cannot be applied (e.g., position out of bounds)
    ///
    /// # Note
    ///
    /// For `Reformat`, this returns the original content unchanged. Reformatting
    /// must be done by the formatter, not by the linter.
    pub fn apply(&self, content: &str) -> Result<String, String> {
        match self {
            Self::TextReplace { old, new } => {
                if !content.contains(old.as_str()) {
                    return Err(format!("Text to replace not found: '{old}'"));
                }
                Ok(content.replace(old, new))
            }
            Self::Insert { position, text } => {
                if *position > content.len() {
                    return Err(format!(
                        "Insert position {position} out of bounds (content length: {})",
                        content.len()
                    ));
                }
                let mut result = String::with_capacity(content.len() + text.len());
                result.push_str(&content[..*position]);
                result.push_str(text);
                result.push_str(&content[*position..]);
                Ok(result)
            }
            Self::Delete { start, end } => {
                if *start > content.len() || *end > content.len() || start > end {
                    return Err(format!(
                        "Delete range [{start}, {end}) invalid (content length: {})",
                        content.len()
                    ));
                }
                let mut result = String::with_capacity(content.len());
                result.push_str(&content[..*start]);
                result.push_str(&content[*end..]);
                Ok(result)
            }
            Self::Reformat => {
                // Reformat is handled by the formatter, not here
                Ok(content.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_replace() {
        let fix = Fix::replace("fix checkbox", "- [*]", "- [ ]");
        assert_eq!(fix.description, "fix checkbox");

        let content = "- [*] task one\n- [*] task two";
        if let Replacement::TextReplace { old, new } = &fix.replacement {
            let result = fix.replacement.apply(content).unwrap();
            assert_eq!(result, "- [ ] task one\n- [ ] task two");
            assert_eq!(old, "- [*]");
            assert_eq!(new, "- [ ]");
        } else {
            panic!("Expected TextReplace");
        }
    }

    #[test]
    fn test_insert() {
        let fix = Fix::insert("add annotation", 9, "@id: test\n");
        let content = "# Title\n\n## Tasks";
        // Position 9 is after "# Title\n\n" (8 chars + newline)
        let result = fix.replacement.apply(content).unwrap();
        assert_eq!(result, "# Title\n\n@id: test\n## Tasks");
    }

    #[test]
    fn test_delete() {
        let fix = Fix::delete("remove line", 8, 17);
        let content = "# Title\nbad line\n## Tasks";
        // Delete from position 8 (after "# Title\n") to 17 (before "## Tasks")
        let result = fix.replacement.apply(content).unwrap();
        assert_eq!(result, "# Title\n## Tasks");
    }

    #[test]
    fn test_reformat() {
        let fix = Fix::reformat("reformat file");
        let content = "some content";
        let result = fix.replacement.apply(content).unwrap();
        assert_eq!(result, content); // Reformat doesn't change content here
    }

    #[test]
    fn test_replace_not_found() {
        let replacement = Replacement::TextReplace {
            old: "missing".to_string(),
            new: "new".to_string(),
        };
        let content = "some content";
        let result = replacement.apply(content);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_insert_out_of_bounds() {
        let replacement = Replacement::Insert {
            position: 1000,
            text: "text".to_string(),
        };
        let content = "short";
        let result = replacement.apply(content);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of bounds"));
    }

    #[test]
    fn test_delete_invalid_range() {
        let replacement = Replacement::Delete { start: 10, end: 5 };
        let content = "some content";
        let result = replacement.apply(content);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid"));
    }

    #[test]
    fn test_fix_constructors() {
        let replace_fix = Fix::replace("desc", "old", "new");
        matches!(replace_fix.replacement, Replacement::TextReplace { .. });

        let insert_fix = Fix::insert("desc", 0, "text");
        matches!(insert_fix.replacement, Replacement::Insert { .. });

        let delete_fix = Fix::delete("desc", 0, 10);
        matches!(delete_fix.replacement, Replacement::Delete { .. });

        let reformat_fix = Fix::reformat("desc");
        matches!(reformat_fix.replacement, Replacement::Reformat);
    }
}
