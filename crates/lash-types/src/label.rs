//! Label model and parsing utilities

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::error::{codes, LashError, Result};

/// A label for categorizing tasks and files
///
/// Labels are normalized strings that follow specific formatting rules:
/// - Lowercase
/// - Alphanumeric, hyphens, underscores only
/// - 1-50 characters
/// - Cannot start with a number
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Label {
    /// Normalized label name
    pub name: String,
}

impl Label {
    /// Create a new label from a string, normalizing it
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_types::label::Label;
    ///
    /// let label = Label::new("Backend API");
    /// assert_eq!(label.name, "backend-api");
    /// ```
    #[must_use]
    pub fn new(s: &str) -> Self {
        Self { name: normalize(s) }
    }

    /// Validate the label
    ///
    /// # Errors
    ///
    /// Returns error if the label doesn't meet validation criteria
    pub fn validate(&self) -> Result<()> {
        if !is_valid_label(&self.name) {
            return Err(LashError::Lint {
                code: codes::E_LINT_INVALID_LABEL,
                message: format!("Invalid label: '{}'", self.name),
                location: None,
                snippet: None,
                help: Some("labels must be alphanumeric with hyphens".to_string()),
            });
        }
        Ok(())
    }
}

/// Normalize a label string
///
/// Normalization process:
/// 1. Convert to lowercase
/// 2. Trim whitespace
/// 3. Replace spaces with hyphens
/// 4. Keep only alphanumeric, hyphens, underscores
/// 5. Strip leading/trailing hyphens
///
/// # Examples
///
/// ```
/// use lash_types::label::normalize;
///
/// assert_eq!(normalize("Backend API"), "backend-api");
/// assert_eq!(normalize("  trim-me  "), "trim-me");
/// assert_eq!(normalize("special!@#chars"), "specialchars");
/// assert_eq!(normalize("-leading-"), "leading");
/// ```
#[must_use]
pub fn normalize(s: &str) -> String {
    s.to_lowercase()
        .trim()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Check if a string is a valid label
///
/// Valid labels:
/// - 1-50 characters
/// - Alphanumeric, hyphens, underscores only
/// - Cannot start with a number
///
/// # Examples
///
/// ```
/// use lash_types::label::is_valid_label;
///
/// assert!(is_valid_label("backend"));
/// assert!(is_valid_label("backend-api"));
/// assert!(is_valid_label("backend_api"));
/// assert!(!is_valid_label(""));
/// assert!(!is_valid_label("123invalid"));
/// assert!(!is_valid_label("has spaces"));
/// ```
#[must_use]
pub fn is_valid_label(s: &str) -> bool {
    if s.is_empty() || s.len() > 50 {
        return false;
    }

    // Check doesn't start with number
    if s.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return false;
    }

    // Check all characters are valid
    s.chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

/// Parse inline labels from text (e.g., "#backend #api")
///
/// Finds all `#word` patterns in the text, extracts and normalizes them.
///
/// # Examples
///
/// ```
/// use lash_types::label::parse_inline_labels;
///
/// let labels = parse_inline_labels("Implement #backend #api feature");
/// assert_eq!(labels.len(), 2);
/// assert!(labels.iter().any(|l| l.name == "backend"));
/// assert!(labels.iter().any(|l| l.name == "api"));
/// ```
#[must_use]
pub fn parse_inline_labels(text: &str) -> Vec<Label> {
    let mut labels = HashSet::new();

    // Find all #word patterns
    for word in text.split_whitespace() {
        if let Some(label_text) = word.strip_prefix('#') {
            // Remove any trailing punctuation
            let clean = label_text.trim_end_matches(|c: char| !c.is_alphanumeric());
            if !clean.is_empty() {
                let normalized = normalize(clean);
                if is_valid_label(&normalized) {
                    labels.insert(normalized);
                }
            }
        }
    }

    labels.into_iter().map(|name| Label { name }).collect()
}

/// Parse annotation labels from text (e.g., "@labels: backend, api")
///
/// Parses the `@labels: a, b, c` format, splitting on commas.
///
/// Note: This function expects just the label values, not the `@labels:` prefix.
///
/// # Examples
///
/// ```
/// use lash_types::label::parse_annotation_labels;
///
/// let labels = parse_annotation_labels("backend, api, database");
/// assert_eq!(labels.len(), 3);
/// assert!(labels.iter().any(|l| l.name == "backend"));
/// assert!(labels.iter().any(|l| l.name == "api"));
/// assert!(labels.iter().any(|l| l.name == "database"));
/// ```
#[must_use]
pub fn parse_annotation_labels(text: &str) -> Vec<Label> {
    let mut labels = HashSet::new();

    // Split on commas and normalize each
    for part in text.split(',') {
        let trimmed = part.trim();
        if !trimmed.is_empty() {
            let normalized = normalize(trimmed);
            if is_valid_label(&normalized) {
                labels.insert(normalized);
            }
        }
    }

    labels.into_iter().map(|name| Label { name }).collect()
}

/// Merge labels from different sources
///
/// Combines inline and annotation labels, deduplicates, and sorts alphabetically.
///
/// # Examples
///
/// ```
/// use lash_types::label::{Label, merge_labels};
///
/// let inline = vec![Label::new("backend"), Label::new("api")];
/// let annotation = vec![Label::new("api"), Label::new("database")];
///
/// let merged = merge_labels(inline, annotation);
/// assert_eq!(merged.len(), 3);
/// ```
#[must_use]
pub fn merge_labels(inline: Vec<Label>, annotation: Vec<Label>) -> Vec<Label> {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();

    // Add inline labels first (keep first occurrence)
    for label in inline {
        if seen.insert(label.name.clone()) {
            merged.push(label);
        }
    }

    // Add annotation labels
    for label in annotation {
        if seen.insert(label.name.clone()) {
            merged.push(label);
        }
    }

    // Sort alphabetically for consistency
    merged.sort_by(|a, b| a.name.cmp(&b.name));

    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize() {
        assert_eq!(normalize("Backend API"), "backend-api");
        assert_eq!(normalize("  trim-me  "), "trim-me");
        assert_eq!(normalize("special!@#chars"), "specialchars");
        assert_eq!(normalize("-leading-"), "leading");
        assert_eq!(normalize("UPPERCASE"), "uppercase");
        assert_eq!(normalize("under_score"), "under_score");
    }

    #[test]
    fn test_is_valid_label() {
        // Valid
        assert!(is_valid_label("backend"));
        assert!(is_valid_label("backend-api"));
        assert!(is_valid_label("backend_api"));
        assert!(is_valid_label("b"));
        assert!(is_valid_label("long-but-valid-label-name"));

        // Invalid: empty
        assert!(!is_valid_label(""));

        // Invalid: starts with number
        assert!(!is_valid_label("123invalid"));

        // Invalid: too long
        assert!(!is_valid_label(&"a".repeat(51)));

        // Invalid: special characters
        assert!(!is_valid_label("has spaces"));
        assert!(!is_valid_label("has@symbol"));
        assert!(!is_valid_label("has.dot"));
    }

    #[test]
    fn test_label_new() {
        let label = Label::new("Backend API");
        assert_eq!(label.name, "backend-api");
    }

    #[test]
    fn test_label_validate() {
        let valid = Label::new("backend");
        assert!(valid.validate().is_ok());

        let invalid = Label {
            name: "123invalid".to_string(),
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_parse_inline_labels() {
        let labels = parse_inline_labels("Implement #backend #api feature");
        assert_eq!(labels.len(), 2);
        assert!(labels.iter().any(|l| l.name == "backend"));
        assert!(labels.iter().any(|l| l.name == "api"));
    }

    #[test]
    fn test_parse_inline_labels_with_punctuation() {
        let labels = parse_inline_labels("Task #backend, #api, and #database!");
        assert_eq!(labels.len(), 3);
        assert!(labels.iter().any(|l| l.name == "backend"));
        assert!(labels.iter().any(|l| l.name == "api"));
        assert!(labels.iter().any(|l| l.name == "database"));
    }

    #[test]
    fn test_parse_inline_labels_deduplication() {
        let labels = parse_inline_labels("#backend #api #backend");
        assert_eq!(labels.len(), 2);
    }

    #[test]
    fn test_parse_inline_labels_empty() {
        let labels = parse_inline_labels("No labels here");
        assert_eq!(labels.len(), 0);
    }

    #[test]
    fn test_parse_annotation_labels() {
        let labels = parse_annotation_labels("backend, api, database");
        assert_eq!(labels.len(), 3);
        assert!(labels.iter().any(|l| l.name == "backend"));
        assert!(labels.iter().any(|l| l.name == "api"));
        assert!(labels.iter().any(|l| l.name == "database"));
    }

    #[test]
    fn test_parse_annotation_labels_normalization() {
        let labels = parse_annotation_labels("Backend API, Database");
        assert!(labels.iter().any(|l| l.name == "backend-api"));
        assert!(labels.iter().any(|l| l.name == "database"));
    }

    #[test]
    fn test_parse_annotation_labels_deduplication() {
        let labels = parse_annotation_labels("backend, api, backend");
        assert_eq!(labels.len(), 2);
    }

    #[test]
    fn test_parse_annotation_labels_empty() {
        let labels = parse_annotation_labels("");
        assert_eq!(labels.len(), 0);
    }

    #[test]
    fn test_merge_labels() {
        let inline = vec![Label::new("backend"), Label::new("api")];
        let annotation = vec![Label::new("api"), Label::new("database")];

        let merged = merge_labels(inline, annotation);
        assert_eq!(merged.len(), 3);

        // Check alphabetical order
        assert_eq!(merged[0].name, "api");
        assert_eq!(merged[1].name, "backend");
        assert_eq!(merged[2].name, "database");
    }

    #[test]
    fn test_merge_labels_keeps_first() {
        let inline = vec![Label::new("backend")];
        let annotation = vec![Label::new("backend")];

        let merged = merge_labels(inline, annotation);
        assert_eq!(merged.len(), 1);
    }

    #[test]
    fn test_merge_labels_empty() {
        let merged = merge_labels(vec![], vec![]);
        assert_eq!(merged.len(), 0);
    }
}
