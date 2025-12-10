//! Label format validation rule
//!
//! Ensures that task labels match the required format:
//! lowercase alphanumeric with hyphens and underscores.

use lash_types::{Severity, Task};

use crate::linter::{Fix, LintContext, LintDiagnostic, LintRule, Replacement};

/// Rule that validates label format
///
/// Labels must follow the pattern: `[a-z0-9][a-z0-9-_]*`
/// - Start with a lowercase letter or digit
/// - Contain only lowercase letters, digits, hyphens, and underscores
/// - No spaces or special characters
///
/// **Code:** `E_SEM_INVALID_LABEL`
/// **Severity:** Error
///
/// # Auto-fix
///
/// The auto-fix normalizes labels by:
/// - Converting to lowercase
/// - Replacing spaces with hyphens
/// - Removing invalid characters
///
/// # Examples
///
/// Valid labels:
/// ```markdown
/// - [ ] Task #backend
/// - [ ] Task #api-endpoint
/// - [ ] Task #v2_migration
/// ```
///
/// Invalid labels:
/// ```markdown
/// - [ ] Task #Backend ← uppercase
/// - [ ] Task #API Endpoint ← spaces
/// - [ ] Task #v2.0 ← dots not allowed
/// ```
pub struct ValidLabelRule;

impl ValidLabelRule {
    /// Create a new valid label rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Check if a label is valid
    fn is_valid_label(label: &str) -> bool {
        if label.is_empty() {
            return false;
        }

        // Must start with lowercase letter or digit
        let first_char = label.chars().next().unwrap();
        if !first_char.is_ascii_lowercase() && !first_char.is_ascii_digit() {
            return false;
        }

        // All characters must be lowercase alphanumeric, hyphen, or underscore
        label
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    }

    /// Normalize a label to valid format
    fn normalize_label(label: &str) -> String {
        label
            .to_lowercase()
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect()
    }
}

impl Default for ValidLabelRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRule for ValidLabelRule {
    fn code(&self) -> &'static str {
        "E_SEM_INVALID_LABEL"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> String {
        "Label format".to_string()
    }

    fn description(&self) -> &'static str {
        "Ensures labels match pattern: lowercase alphanumeric with hyphens/underscores"
    }

    fn check_task(&self, task: &Task, ctx: &LintContext) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        for label in &task.metadata.labels {
            if !Self::is_valid_label(label) {
                let normalized = Self::normalize_label(label);

                let fix = Fix {
                    description: format!("Normalize label '{label}' to '{normalized}'"),
                    replacement: Replacement::TextReplace {
                        old: format!("#{label}"),
                        new: format!("#{normalized}"),
                    },
                };

                diagnostics.push(
                    LintDiagnostic::error(
                        self.code(),
                        format!("Invalid label format: '{label}'"),
                        ctx.file_path.clone(),
                        0,
                        0,
                    )
                    .with_help(
                        "Labels must be lowercase alphanumeric with hyphens or underscores, \
                        starting with a letter or digit",
                    )
                    .with_fix(fix),
                );
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::{LashConfig, Task, TaskMetadata, TaskStatus};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn make_config() -> LashConfig {
        LashConfig {
            root_path: PathBuf::from("/test"),
            index_file: "index.md".to_string(),
            max_depth: 2,
            indent_spaces: 2,
            db_path: PathBuf::from(".lash/test.db"),
            custom_annotation_keys: vec![],
        }
    }

    fn make_task_with_labels(labels: &[&str]) -> Task {
        Task {
            id: "task-1".to_string(),
            title: "Test task".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            metadata: TaskMetadata {
                labels: labels.iter().map(|s| (*s).to_string()).collect(),
                ..Default::default()
            },
            body: None,
        }
    }

    #[test]
    fn test_valid_labels() {
        let rule = ValidLabelRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let valid_labels = vec![
            "backend",
            "api-endpoint",
            "v2_migration",
            "auth",
            "2fa",
            "ui-component",
            "bug_fix",
        ];

        for label in valid_labels {
            let task = make_task_with_labels(&[label]);
            let diagnostics = rule.check_task(&task, &ctx);
            assert!(diagnostics.is_empty(), "Label '{label}' should be valid");
        }
    }

    #[test]
    fn test_invalid_uppercase() {
        let rule = ValidLabelRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let task = make_task_with_labels(&["Backend"]);
        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_SEM_INVALID_LABEL");
        assert!(diagnostics[0].message.contains("Backend"));
        assert!(diagnostics[0].has_fix());
    }

    #[test]
    fn test_invalid_spaces() {
        let rule = ValidLabelRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let task = make_task_with_labels(&["API Endpoint"]);
        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("API Endpoint"));
    }

    #[test]
    fn test_invalid_special_chars() {
        let rule = ValidLabelRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let invalid_labels = vec!["v2.0", "api@rest", "bug#123", "task!", "my/label"];

        for label in invalid_labels {
            let task = make_task_with_labels(&[label]);
            let diagnostics = rule.check_task(&task, &ctx);
            assert!(!diagnostics.is_empty(), "Label '{label}' should be invalid");
        }
    }

    #[test]
    fn test_invalid_start_with_hyphen() {
        let rule = ValidLabelRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let task = make_task_with_labels(&["-invalid"]);
        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_multiple_invalid_labels() {
        let rule = ValidLabelRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let task = make_task_with_labels(&["Valid", "Invalid Label", "another-BAD"]);
        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 3, "All three labels are invalid");
    }

    #[test]
    fn test_mixed_valid_invalid() {
        let rule = ValidLabelRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let task = make_task_with_labels(&["valid", "Invalid", "also-valid", "BAD LABEL"]);
        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 2, "Two invalid labels");
    }

    #[test]
    fn test_no_labels() {
        let rule = ValidLabelRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let task = make_task_with_labels(&[]);
        let diagnostics = rule.check_task(&task, &ctx);
        assert!(diagnostics.is_empty(), "No labels means no errors");
    }

    #[test]
    fn test_normalize_label() {
        assert_eq!(ValidLabelRule::normalize_label("Backend"), "backend");
        assert_eq!(
            ValidLabelRule::normalize_label("API Endpoint"),
            "api-endpoint"
        );
        assert_eq!(ValidLabelRule::normalize_label("v2.0"), "v20");
        assert_eq!(ValidLabelRule::normalize_label("My Label!"), "my-label");
    }

    #[test]
    fn test_is_valid_label() {
        assert!(ValidLabelRule::is_valid_label("backend"));
        assert!(ValidLabelRule::is_valid_label("api-endpoint"));
        assert!(ValidLabelRule::is_valid_label("v2_migration"));
        assert!(ValidLabelRule::is_valid_label("2fa"));

        assert!(!ValidLabelRule::is_valid_label("Backend"));
        assert!(!ValidLabelRule::is_valid_label("API Endpoint"));
        assert!(!ValidLabelRule::is_valid_label("v2.0"));
        assert!(!ValidLabelRule::is_valid_label("-invalid"));
        assert!(!ValidLabelRule::is_valid_label(""));
    }

    #[test]
    fn test_rule_metadata() {
        let rule = ValidLabelRule::new();
        assert_eq!(rule.code(), "E_SEM_INVALID_LABEL");
        assert_eq!(rule.severity(), Severity::Error);
        assert_eq!(rule.name(), "Label format");
        assert!(!rule.description().is_empty());
    }
}
