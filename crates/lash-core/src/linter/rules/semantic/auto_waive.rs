//! Auto-waive children validation rule
//!
//! When a parent task is waived, all its children should also be waived.
//! This rule detects cases where children need to be auto-waived and
//! provides auto-fix suggestions.

use lash_types::{Severity, Task, TaskFile, TaskStatus};

use crate::linter::{Fix, LintContext, LintDiagnostic, LintRule, Replacement};

/// Rule that auto-waives children when parent is waived
///
/// This rule implements the design decision that when a parent task is
/// marked as waived (not applicable), all its children should also be
/// waived automatically. This prevents orphaned children that would
/// never be completed.
///
/// **Code:** `I_SEM_AUTO_WAIVE`
/// **Severity:** Info
///
/// # Auto-fix
///
/// The auto-fix sets all descendant tasks to Waived status. This is always
/// applied by the formatter.
///
/// # Examples
///
/// Before auto-waive:
/// ```markdown
/// - [-] Parent task (waived)
///   - [ ] Child task ← INFO: should be auto-waived
///   - [ ] Another child ← INFO: should be auto-waived
/// ```
///
/// After auto-waive:
/// ```markdown
/// - [-] Parent task (waived)
///   - [-] Child task
///   - [-] Another child
/// ```
pub struct AutoWaiveRule;

impl AutoWaiveRule {
    /// Create a new auto-waive rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Get all children of a task (recursive)
    fn get_all_descendants<'a>(task: &Task, all_tasks: &'a [Task]) -> Vec<&'a Task> {
        let mut descendants = Vec::new();

        // Find direct children
        let children: Vec<_> = all_tasks
            .iter()
            .filter(|child| child.parent_id.as_deref() == Some(&task.id))
            .collect();

        for child in children {
            descendants.push(child);
            // Recursively get grandchildren
            descendants.extend(Self::get_all_descendants(child, all_tasks));
        }

        descendants
    }

    /// Get non-waived children of a task
    fn get_non_waived_children<'a>(task: &Task, all_tasks: &'a [Task]) -> Vec<&'a Task> {
        Self::get_all_descendants(task, all_tasks)
            .into_iter()
            .filter(|child| child.status != TaskStatus::Waived)
            .collect()
    }
}

impl Default for AutoWaiveRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRule for AutoWaiveRule {
    fn code(&self) -> &'static str {
        "I_SEM_AUTO_WAIVE"
    }

    fn severity(&self) -> Severity {
        Severity::Info
    }

    fn name(&self) -> String {
        "Auto-waive children".to_string()
    }

    fn description(&self) -> &'static str {
        "Auto-waives children when parent task is waived"
    }

    fn check_file(&self, file: &TaskFile, ctx: &LintContext) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        let all_tasks = file.tasks.tasks();

        for task in all_tasks {
            // Only check waived tasks
            if task.status != TaskStatus::Waived {
                continue;
            }

            // Check if this task has non-waived children
            let non_waived = Self::get_non_waived_children(task, all_tasks);

            if !non_waived.is_empty() {
                let child_list = non_waived
                    .iter()
                    .map(|c| format!("  - \"{}\" ({})", c.title, c.status))
                    .collect::<Vec<_>>()
                    .join("\n");

                // Build auto-fix description
                // Note: For multiple changes, we use Reformat which delegates to the formatter
                let fix = Fix {
                    description: format!(
                        "Auto-waive {} child(ren) of \"{}\". This will be applied by the formatter.",
                        non_waived.len(),
                        task.title
                    ),
                    replacement: Replacement::Reformat,
                };

                diagnostics.push(
                    LintDiagnostic::info(
                        self.code(),
                        format!(
                            "Parent task \"{}\" is waived; {} child(ren) should be auto-waived",
                            task.title,
                            non_waived.len()
                        ),
                        ctx.file_path.clone(),
                        0,
                        0,
                    )
                    .with_help(format!(
                        "Children will be auto-waived by formatter. Affected children:\n{child_list}"
                    ))
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
    use lash_types::{FileMetadata, LashConfig, Task, TaskMetadata, TaskTree};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::SystemTime;

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

    fn make_task(
        id: &str,
        title: &str,
        status: TaskStatus,
        parent: Option<&str>,
        depth: u8,
    ) -> Task {
        Task {
            id: id.to_string(),
            title: title.to_string(),
            status,
            depth,
            parent_id: parent.map(std::string::ToString::to_string),
            order_index: 0,
            line_number: 0,
            metadata: TaskMetadata::default(),
            body: None,
            contextual_notes: Vec::new(),
        }
    }

    fn make_file_with_tasks(tasks: Vec<Task>) -> TaskFile {
        let mut tree = TaskTree::new();
        for task in tasks {
            tree.add_task(task).unwrap();
        }

        TaskFile {
            path: PathBuf::from("test.md"),
            title: "Test File".to_string(),
            id: "test".to_string(),
            metadata: FileMetadata::default(),
            description: None,
            description_agent_notes: Vec::new(),
            tasks: tree,
            hash: "hash".to_string(),
            mtime: SystemTime::now(),
        }
    }

    #[test]
    fn test_waived_parent_all_children_waived() {
        let rule = AutoWaiveRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let file = make_file_with_tasks(vec![
            make_task("parent", "Parent", TaskStatus::Waived, None, 0),
            make_task("child1", "Child 1", TaskStatus::Waived, Some("parent"), 1),
            make_task("child2", "Child 2", TaskStatus::Waived, Some("parent"), 1),
        ]);

        let diagnostics = rule.check_file(&file, &ctx);
        assert!(diagnostics.is_empty(), "All children already waived");
    }

    #[test]
    fn test_waived_parent_with_open_children() {
        let rule = AutoWaiveRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let file = make_file_with_tasks(vec![
            make_task("parent", "Parent", TaskStatus::Waived, None, 0),
            make_task("child1", "Child 1", TaskStatus::Open, Some("parent"), 1),
            make_task("child2", "Child 2", TaskStatus::Open, Some("parent"), 1),
        ]);

        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "I_SEM_AUTO_WAIVE");
        assert_eq!(diagnostics[0].severity, Severity::Info);
        assert!(diagnostics[0].message.contains("Parent"));
        assert!(diagnostics[0].message.contains("2 child"));
        assert!(diagnostics[0].has_fix());
    }

    #[test]
    fn test_waived_parent_mixed_children() {
        let rule = AutoWaiveRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let file = make_file_with_tasks(vec![
            make_task("parent", "Parent", TaskStatus::Waived, None, 0),
            make_task("child1", "Child 1", TaskStatus::Waived, Some("parent"), 1),
            make_task("child2", "Child 2", TaskStatus::Open, Some("parent"), 1),
            make_task("child3", "Child 3", TaskStatus::Done, Some("parent"), 1),
        ]);

        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("2 child")); // Two non-waived
    }

    #[test]
    fn test_non_waived_parent() {
        let rule = AutoWaiveRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let file = make_file_with_tasks(vec![
            make_task("parent", "Parent", TaskStatus::Open, None, 0),
            make_task("child1", "Child 1", TaskStatus::Open, Some("parent"), 1),
        ]);

        let diagnostics = rule.check_file(&file, &ctx);
        assert!(
            diagnostics.is_empty(),
            "Non-waived parent doesn't trigger rule"
        );
    }

    #[test]
    fn test_nested_waived_hierarchy() {
        let rule = AutoWaiveRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let file = make_file_with_tasks(vec![
            make_task("parent", "Parent", TaskStatus::Waived, None, 0),
            make_task("child", "Child", TaskStatus::Open, Some("parent"), 1),
            make_task(
                "grandchild",
                "Grandchild",
                TaskStatus::Open,
                Some("child"),
                2,
            ),
        ]);

        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);
        // Should catch both child and grandchild
        assert!(diagnostics[0].message.contains("2 child"));
    }

    #[test]
    fn test_multiple_waived_parents() {
        let rule = AutoWaiveRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let file = make_file_with_tasks(vec![
            make_task("parent1", "Parent 1", TaskStatus::Waived, None, 0),
            make_task("child1", "Child 1", TaskStatus::Open, Some("parent1"), 1),
            make_task("parent2", "Parent 2", TaskStatus::Waived, None, 0),
            make_task("child2", "Child 2", TaskStatus::Open, Some("parent2"), 1),
        ]);

        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(
            diagnostics.len(),
            2,
            "Both waived parents should be flagged"
        );
    }

    #[test]
    fn test_waived_parent_no_children() {
        let rule = AutoWaiveRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let file = make_file_with_tasks(vec![make_task(
            "parent",
            "Parent",
            TaskStatus::Waived,
            None,
            0,
        )]);

        let diagnostics = rule.check_file(&file, &ctx);
        assert!(diagnostics.is_empty(), "No children to waive");
    }

    #[test]
    fn test_rule_metadata() {
        let rule = AutoWaiveRule::new();
        assert_eq!(rule.code(), "I_SEM_AUTO_WAIVE");
        assert_eq!(rule.severity(), Severity::Info);
        assert_eq!(rule.name(), "Auto-waive children");
        assert!(!rule.description().is_empty());
    }
}
