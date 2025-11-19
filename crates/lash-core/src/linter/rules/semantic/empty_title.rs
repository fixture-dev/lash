//! Empty task title validation rule
//!
//! Ensures that all tasks have non-empty titles. Tasks must have meaningful
//! descriptions for clarity and tracking purposes.

use lash_types::{Severity, Task};

use crate::linter::{LintContext, LintDiagnostic, LintRule};

/// Rule that checks for empty or whitespace-only task titles
///
/// Every task must have a meaningful title. Empty or whitespace-only titles
/// make tasks impossible to understand and track.
///
/// **Code:** `E_SEM_EMPTY_TITLE`
/// **Severity:** Error
///
/// # Auto-fix
///
/// No auto-fix is provided because the title content must be supplied by
/// the user. The system cannot infer what the task should be.
///
/// # Examples
///
/// Valid tasks:
/// ```markdown
/// - [ ] Implement authentication
/// - [ ] Write tests
/// - [ ] A
/// ```
///
/// Invalid tasks:
/// ```markdown
/// - [ ]  ← empty title
/// - [ ]    ← whitespace only
/// ```
pub struct EmptyTitleRule;

impl EmptyTitleRule {
    /// Create a new empty title rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Check if a title is empty or whitespace-only
    fn is_empty_title(title: &str) -> bool {
        title.trim().is_empty()
    }
}

impl Default for EmptyTitleRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRule for EmptyTitleRule {
    fn code(&self) -> &'static str {
        "E_SEM_EMPTY_TITLE"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> String {
        "Non-empty title".to_string()
    }

    fn description(&self) -> &'static str {
        "Ensures all tasks have non-empty titles"
    }

    fn check_task(&self, task: &Task, ctx: &LintContext) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        if Self::is_empty_title(&task.title) {
            diagnostics.push(
                LintDiagnostic::error(
                    self.code(),
                    "Task title cannot be empty or whitespace-only",
                    ctx.file_path.clone(),
                    0,
                    0,
                )
                .with_help("Provide a meaningful title that describes what the task is about"),
            );
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

    fn make_task_with_title(title: &str) -> Task {
        Task {
            id: "task-1".to_string(),
            title: title.to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            metadata: TaskMetadata::default(),
            body: None,
        }
    }

    #[test]
    fn test_valid_titles() {
        let rule = EmptyTitleRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let valid_titles = vec![
            "Implement authentication",
            "Write tests",
            "A",                 // Single character is ok
            "  Trimmed title  ", // Has content when trimmed
            "Task with #label",
            "Very long task title that goes on and on...",
        ];

        for title in valid_titles {
            let task = make_task_with_title(title);
            let diagnostics = rule.check_task(&task, &ctx);
            assert!(diagnostics.is_empty(), "Title '{title}' should be valid");
        }
    }

    #[test]
    fn test_empty_title() {
        let rule = EmptyTitleRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let task = make_task_with_title("");
        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_SEM_EMPTY_TITLE");
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert!(diagnostics[0].message.contains("empty"));
        assert!(diagnostics[0].help.is_some());
    }

    #[test]
    fn test_whitespace_only_title() {
        let rule = EmptyTitleRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let whitespace_titles = vec![" ", "  ", "   ", "\t", "\n", " \t\n "];

        for title in whitespace_titles {
            let task = make_task_with_title(title);
            let diagnostics = rule.check_task(&task, &ctx);
            assert_eq!(
                diagnostics.len(),
                1,
                "Whitespace-only title {title:?} should be invalid"
            );
            assert_eq!(diagnostics[0].code, "E_SEM_EMPTY_TITLE");
        }
    }

    #[test]
    fn test_is_empty_title() {
        assert!(EmptyTitleRule::is_empty_title(""));
        assert!(EmptyTitleRule::is_empty_title("  "));
        assert!(EmptyTitleRule::is_empty_title("\t"));
        assert!(EmptyTitleRule::is_empty_title(" \n "));

        assert!(!EmptyTitleRule::is_empty_title("A"));
        assert!(!EmptyTitleRule::is_empty_title("Task"));
        assert!(!EmptyTitleRule::is_empty_title("  Task  "));
    }

    #[test]
    fn test_rule_metadata() {
        let rule = EmptyTitleRule::new();
        assert_eq!(rule.code(), "E_SEM_EMPTY_TITLE");
        assert_eq!(rule.severity(), Severity::Error);
        assert_eq!(rule.name(), "Non-empty title");
        assert!(!rule.description().is_empty());
    }
}
