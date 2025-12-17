//! Parent-child status consistency validation rule
//!
//! Ensures that parent tasks cannot be marked as Done if they have Open children.
//! This maintains logical consistency in task completion tracking.

use lash_types::{Severity, Task, TaskFile, TaskStatus};

use crate::linter::{Fix, LintContext, LintDiagnostic, LintRule, Replacement};

/// Rule that checks parent-child status consistency
///
/// This rule ensures that parent tasks are not marked as complete when they
/// still have incomplete children. This prevents logical inconsistencies in
/// the task hierarchy.
///
/// **Code:** `W_SEM_STATUS_INCONSISTENT`
/// **Severity:** Warning
///
/// # Auto-fix
///
/// The default auto-fix unmarksthe parent task (safer option). An alternative
/// fix could mark all children as complete, but that's more aggressive.
///
/// # Examples
///
/// Invalid (parent done with open children):
/// ```markdown
/// - [x] Parent task ← WARNING: has incomplete children
///   - [ ] Open child
///   - [ ] Another open child
/// ```
///
/// Valid (consistent status):
/// ```markdown
/// - [ ] Parent task
///   - [ ] Open child
///   - [ ] Another open child
/// ```
///
/// Or:
/// ```markdown
/// - [x] Parent task
///   - [x] Completed child
///   - [x] Another completed child
/// ```
pub struct StatusConsistencyRule;

impl StatusConsistencyRule {
    /// Create a new status consistency rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Check if a task has any incomplete children
    fn has_incomplete_children(task: &Task, all_tasks: &[Task]) -> bool {
        all_tasks.iter().any(|child| {
            child.parent_id.as_deref() == Some(&task.id) && !child.status.is_complete()
        })
    }

    /// Get incomplete children for a task
    fn get_incomplete_children<'a>(task: &Task, all_tasks: &'a [Task]) -> Vec<&'a Task> {
        all_tasks
            .iter()
            .filter(|child| {
                child.parent_id.as_deref() == Some(&task.id) && !child.status.is_complete()
            })
            .collect()
    }
}

impl Default for StatusConsistencyRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRule for StatusConsistencyRule {
    fn code(&self) -> &'static str {
        "W_SEM_STATUS_INCONSISTENT"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> String {
        "Status consistency".to_string()
    }

    fn description(&self) -> &'static str {
        "Ensures parent tasks are not marked complete when children are incomplete"
    }

    fn check_file(&self, file: &TaskFile, ctx: &LintContext) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        let all_tasks = file.tasks.tasks();

        for task in all_tasks {
            // Only check tasks marked as Done
            if task.status != TaskStatus::Done {
                continue;
            }

            // Check if this task has incomplete children
            if Self::has_incomplete_children(task, all_tasks) {
                let incomplete = Self::get_incomplete_children(task, all_tasks);
                let child_list = incomplete
                    .iter()
                    .map(|c| format!("  - \"{}\"", c.title))
                    .collect::<Vec<_>>()
                    .join("\n");

                let fix = Fix {
                    description: format!("Unmark parent task \"{}\"", task.title),
                    replacement: Replacement::TextReplace {
                        old: format!("- [x] {}", task.title),
                        new: format!("- [ ] {}", task.title),
                    },
                };

                diagnostics.push(
                    LintDiagnostic::warning(
                        self.code(),
                        format!(
                            "Parent task \"{}\" is marked complete but has {} incomplete child(ren)",
                            task.title,
                            incomplete.len()
                        ),
                        ctx.file_path.clone(),
                        0,
                        0,
                    )
                    .with_help(format!(
                        "Complete all children first, or waive them. Incomplete children:\n{child_list}"
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

    fn make_task(id: &str, title: &str, status: TaskStatus, parent: Option<&str>) -> Task {
        Task {
            id: id.to_string(),
            has_explicit_id: false,
            title: title.to_string(),
            status,
            depth: u8::from(parent.is_some()),
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
    fn test_consistent_parent_all_children_done() {
        let rule = StatusConsistencyRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let file = make_file_with_tasks(vec![
            make_task("parent", "Parent", TaskStatus::Done, None),
            make_task("child1", "Child 1", TaskStatus::Done, Some("parent")),
            make_task("child2", "Child 2", TaskStatus::Done, Some("parent")),
        ]);

        let diagnostics = rule.check_file(&file, &ctx);
        assert!(
            diagnostics.is_empty(),
            "Parent with all done children should be valid"
        );
    }

    #[test]
    fn test_consistent_parent_open() {
        let rule = StatusConsistencyRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let file = make_file_with_tasks(vec![
            make_task("parent", "Parent", TaskStatus::Open, None),
            make_task("child1", "Child 1", TaskStatus::Open, Some("parent")),
            make_task("child2", "Child 2", TaskStatus::Done, Some("parent")),
        ]);

        let diagnostics = rule.check_file(&file, &ctx);
        assert!(diagnostics.is_empty(), "Open parent is always valid");
    }

    #[test]
    fn test_inconsistent_parent_done_with_open_children() {
        let rule = StatusConsistencyRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let file = make_file_with_tasks(vec![
            make_task("parent", "Parent", TaskStatus::Done, None),
            make_task("child1", "Child 1", TaskStatus::Open, Some("parent")),
            make_task("child2", "Child 2", TaskStatus::Done, Some("parent")),
        ]);

        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "W_SEM_STATUS_INCONSISTENT");
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert!(diagnostics[0].message.contains("Parent"));
        assert!(diagnostics[0].message.contains("1 incomplete"));
        assert!(diagnostics[0].help.is_some());
        assert!(diagnostics[0].has_fix());
    }

    #[test]
    fn test_inconsistent_all_children_open() {
        let rule = StatusConsistencyRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let file = make_file_with_tasks(vec![
            make_task("parent", "Parent", TaskStatus::Done, None),
            make_task("child1", "Child 1", TaskStatus::Open, Some("parent")),
            make_task("child2", "Child 2", TaskStatus::Open, Some("parent")),
        ]);

        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("2 incomplete"));
    }

    #[test]
    fn test_waived_children_ok() {
        let rule = StatusConsistencyRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let file = make_file_with_tasks(vec![
            make_task("parent", "Parent", TaskStatus::Done, None),
            make_task("child1", "Child 1", TaskStatus::Waived, Some("parent")),
            make_task("child2", "Child 2", TaskStatus::Done, Some("parent")),
        ]);

        let diagnostics = rule.check_file(&file, &ctx);
        assert!(diagnostics.is_empty(), "Waived children count as complete");
    }

    #[test]
    fn test_blocked_child() {
        let rule = StatusConsistencyRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let file = make_file_with_tasks(vec![
            make_task("parent", "Parent", TaskStatus::Done, None),
            make_task("child1", "Child 1", TaskStatus::Blocked, Some("parent")),
        ]);

        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1, "Blocked children are incomplete");
    }

    #[test]
    fn test_multiple_parents_with_issues() {
        let rule = StatusConsistencyRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let file = make_file_with_tasks(vec![
            make_task("parent1", "Parent 1", TaskStatus::Done, None),
            make_task("child1", "Child 1", TaskStatus::Open, Some("parent1")),
            make_task("parent2", "Parent 2", TaskStatus::Done, None),
            make_task("child2", "Child 2", TaskStatus::Open, Some("parent2")),
        ]);

        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 2, "Both parents should be flagged");
    }

    #[test]
    fn test_no_children() {
        let rule = StatusConsistencyRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let file = make_file_with_tasks(vec![
            make_task("task1", "Task 1", TaskStatus::Done, None),
            make_task("task2", "Task 2", TaskStatus::Open, None),
        ]);

        let diagnostics = rule.check_file(&file, &ctx);
        assert!(
            diagnostics.is_empty(),
            "Tasks without children should be valid"
        );
    }

    #[test]
    fn test_rule_metadata() {
        let rule = StatusConsistencyRule::new();
        assert_eq!(rule.code(), "W_SEM_STATUS_INCONSISTENT");
        assert_eq!(rule.severity(), Severity::Warning);
        assert_eq!(rule.name(), "Status consistency");
        assert!(!rule.description().is_empty());
    }
}
