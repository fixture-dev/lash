//! Contextual note ordering style rule
//!
//! Suggests that contextual notes should appear before child tasks
//! for better readability and consistency.

use lash_types::{Severity, TaskFile};

use crate::linter::{LintContext, LintDiagnostic, LintRule};

/// Rule that validates contextual note ordering convention
///
/// This style rule suggests that contextual notes should appear before
/// any child tasks within a parent task. This convention improves
/// readability by keeping context at the top and actionable items below.
///
/// **Code:** `W_NOTE_AFTER_CHILD_TASKS`
/// **Severity:** Warning
///
/// # Rationale
///
/// When notes appear after child tasks, readers may miss important context
/// that would help them understand the task structure. Placing notes first:
/// - Provides context before diving into subtasks
/// - Makes it easier to scan the task hierarchy
/// - Follows natural reading order (context → actions)
///
/// This is a style guideline and not a hard requirement.
///
/// # Examples
///
/// Preferred (notes before children):
/// ```markdown
/// - [ ] Implement authentication
///   - Use OAuth2 with PKCE flow
///   - Maximum session duration: 24 hours
///   - [ ] Create login endpoint
///   - [ ] Create token refresh endpoint
/// ```
///
/// Not preferred (notes after children):
/// ```markdown
/// - [ ] Implement authentication
///   - [ ] Create login endpoint
///   - [ ] Create token refresh endpoint
///   - Use OAuth2 with PKCE flow     <- Warning
///   - Maximum session duration: 24 hours  <- Warning
/// ```
pub struct NoteOrderingRule;

impl NoteOrderingRule {
    /// Create a new note ordering rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoteOrderingRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRule for NoteOrderingRule {
    fn code(&self) -> &'static str {
        "W_NOTE_AFTER_CHILD_TASKS"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> String {
        "Note ordering".to_string()
    }

    fn description(&self) -> &'static str {
        "Suggests placing contextual notes before child tasks for better readability"
    }

    fn check_file(&self, file: &TaskFile, ctx: &LintContext) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        let tasks = file.tasks.tasks();

        for task in tasks {
            if task.contextual_notes.is_empty() {
                continue;
            }

            // Get the line number of the first child task
            let first_child_line = tasks
                .iter()
                .filter(|t| t.parent_id.as_deref() == Some(&task.id))
                .filter(|t| t.line_number > 0)
                .map(|t| t.line_number)
                .min();

            // If there are no children with line numbers, nothing to check
            let Some(first_child_line) = first_child_line else {
                continue;
            };

            // Check each note to see if it appears after the first child
            for note in &task.contextual_notes {
                let note_line = note.line_number();

                // Skip notes without line numbers
                if note_line == 0 {
                    continue;
                }

                // If the note appears after the first child task, warn
                if note_line > first_child_line {
                    let diag = LintDiagnostic::warning(
                        "W_NOTE_AFTER_CHILD_TASKS",
                        format!(
                            "Contextual note appears after child tasks in '{}'",
                            task.title
                        ),
                        ctx.file_path.clone(),
                        note_line,
                        0,
                    )
                    .with_snippet(note.truncated_text(50))
                    .with_help("Consider moving notes before child tasks for better readability");

                    diagnostics.push(diag);
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::{task::ContextualNote, FileMetadata, LashConfig, TaskBuilder, TaskTree};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::SystemTime;

    fn make_config() -> LashConfig {
        LashConfig {
            root_path: PathBuf::from("/test"),
            index_file: "index.md".to_string(),
            max_depth: 3,
            indent_spaces: 2,
            db_path: PathBuf::from(".lash/test.db"),
            custom_annotation_keys: vec![],
        }
    }

    fn make_file(tasks: TaskTree) -> TaskFile {
        TaskFile {
            path: PathBuf::from("test.md"),
            title: "Test File".to_string(),
            id: "test".to_string(),
            metadata: FileMetadata::default(),
            description: None,
            description_agent_notes: Vec::new(),
            tasks,
            hash: "hash".to_string(),
            mtime: SystemTime::now(),
        }
    }

    #[test]
    fn test_notes_before_children_passes() {
        let rule = NoteOrderingRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // Structure:
        // - [ ] Parent (line 1)
        //   - Note (line 2)
        //   - [ ] Child (line 3)
        let notes = vec![ContextualNote::new("A note", 2)];

        let mut tree = TaskTree::new();
        tree.add_task(
            TaskBuilder::new("Parent")
                .id("parent")
                .line_number(1)
                .contextual_notes(notes)
                .build()
                .unwrap(),
        )
        .unwrap();
        tree.add_task(
            TaskBuilder::new("Child")
                .id("child")
                .parent("parent")
                .depth(1)
                .line_number(3)
                .build()
                .unwrap(),
        )
        .unwrap();

        let file = make_file(tree);
        let diagnostics = rule.check_file(&file, &ctx);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_notes_after_children_warns() {
        let rule = NoteOrderingRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // Structure:
        // - [ ] Parent (line 1)
        //   - [ ] Child (line 2)
        //   - Note (line 3)  <- Note after child
        let notes = vec![ContextualNote::new("A note", 3)];

        let mut tree = TaskTree::new();
        tree.add_task(
            TaskBuilder::new("Parent")
                .id("parent")
                .line_number(1)
                .contextual_notes(notes)
                .build()
                .unwrap(),
        )
        .unwrap();
        tree.add_task(
            TaskBuilder::new("Child")
                .id("child")
                .parent("parent")
                .depth(1)
                .line_number(2)
                .build()
                .unwrap(),
        )
        .unwrap();

        let file = make_file(tree);
        let diagnostics = rule.check_file(&file, &ctx);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "W_NOTE_AFTER_CHILD_TASKS");
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert!(diagnostics[0].message.contains("Parent"));
    }

    #[test]
    fn test_multiple_notes_some_after_children() {
        let rule = NoteOrderingRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // Structure:
        // - [ ] Parent (line 1)
        //   - Note 1 (line 2)  <- OK
        //   - [ ] Child (line 3)
        //   - Note 2 (line 4)  <- Warning
        //   - Note 3 (line 5)  <- Warning
        let notes = vec![
            ContextualNote::new("Note 1", 2),
            ContextualNote::new("Note 2", 4),
            ContextualNote::new("Note 3", 5),
        ];

        let mut tree = TaskTree::new();
        tree.add_task(
            TaskBuilder::new("Parent")
                .id("parent")
                .line_number(1)
                .contextual_notes(notes)
                .build()
                .unwrap(),
        )
        .unwrap();
        tree.add_task(
            TaskBuilder::new("Child")
                .id("child")
                .parent("parent")
                .depth(1)
                .line_number(3)
                .build()
                .unwrap(),
        )
        .unwrap();

        let file = make_file(tree);
        let diagnostics = rule.check_file(&file, &ctx);

        // Two notes (line 4 and 5) appear after the child (line 3)
        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics
            .iter()
            .all(|d| d.code == "W_NOTE_AFTER_CHILD_TASKS"));
    }

    #[test]
    fn test_task_with_notes_no_children_passes() {
        let rule = NoteOrderingRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let notes = vec![ContextualNote::new("A note", 2)];

        let mut tree = TaskTree::new();
        tree.add_task(
            TaskBuilder::new("Parent")
                .id("parent")
                .line_number(1)
                .contextual_notes(notes)
                .build()
                .unwrap(),
        )
        .unwrap();

        let file = make_file(tree);
        let diagnostics = rule.check_file(&file, &ctx);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_task_with_children_no_notes_passes() {
        let rule = NoteOrderingRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let mut tree = TaskTree::new();
        tree.add_task(
            TaskBuilder::new("Parent")
                .id("parent")
                .line_number(1)
                .build()
                .unwrap(),
        )
        .unwrap();
        tree.add_task(
            TaskBuilder::new("Child")
                .id("child")
                .parent("parent")
                .depth(1)
                .line_number(2)
                .build()
                .unwrap(),
        )
        .unwrap();

        let file = make_file(tree);
        let diagnostics = rule.check_file(&file, &ctx);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_no_line_numbers_passes() {
        let rule = NoteOrderingRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // Without line numbers, we can't determine ordering
        let notes = vec![ContextualNote::new("A note", 0)];

        let mut tree = TaskTree::new();
        tree.add_task(
            TaskBuilder::new("Parent")
                .id("parent")
                .line_number(0)
                .contextual_notes(notes)
                .build()
                .unwrap(),
        )
        .unwrap();
        tree.add_task(
            TaskBuilder::new("Child")
                .id("child")
                .parent("parent")
                .depth(1)
                .line_number(0)
                .build()
                .unwrap(),
        )
        .unwrap();

        let file = make_file(tree);
        let diagnostics = rule.check_file(&file, &ctx);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_multiple_parents_with_issues() {
        let rule = NoteOrderingRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // Two parent tasks, both with notes after children
        let mut tree = TaskTree::new();
        tree.add_task(
            TaskBuilder::new("Parent 1")
                .id("parent1")
                .line_number(1)
                .contextual_notes(vec![ContextualNote::new("Note 1", 3)])
                .build()
                .unwrap(),
        )
        .unwrap();
        tree.add_task(
            TaskBuilder::new("Child 1")
                .id("child1")
                .parent("parent1")
                .depth(1)
                .line_number(2)
                .build()
                .unwrap(),
        )
        .unwrap();

        tree.add_task(
            TaskBuilder::new("Parent 2")
                .id("parent2")
                .line_number(4)
                .contextual_notes(vec![ContextualNote::new("Note 2", 6)])
                .build()
                .unwrap(),
        )
        .unwrap();
        tree.add_task(
            TaskBuilder::new("Child 2")
                .id("child2")
                .parent("parent2")
                .depth(1)
                .line_number(5)
                .build()
                .unwrap(),
        )
        .unwrap();

        let file = make_file(tree);
        let diagnostics = rule.check_file(&file, &ctx);

        assert_eq!(diagnostics.len(), 2);
    }

    #[test]
    fn test_note_same_line_as_child_passes() {
        let rule = NoteOrderingRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // Edge case: note on same line as child (unlikely but should not warn)
        let notes = vec![ContextualNote::new("A note", 2)];

        let mut tree = TaskTree::new();
        tree.add_task(
            TaskBuilder::new("Parent")
                .id("parent")
                .line_number(1)
                .contextual_notes(notes)
                .build()
                .unwrap(),
        )
        .unwrap();
        tree.add_task(
            TaskBuilder::new("Child")
                .id("child")
                .parent("parent")
                .depth(1)
                .line_number(2) // Same line as note
                .build()
                .unwrap(),
        )
        .unwrap();

        let file = make_file(tree);
        let diagnostics = rule.check_file(&file, &ctx);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_rule_metadata() {
        let rule = NoteOrderingRule::new();
        assert_eq!(rule.code(), "W_NOTE_AFTER_CHILD_TASKS");
        assert_eq!(rule.severity(), Severity::Warning);
        assert_eq!(rule.name(), "Note ordering");
        assert!(!rule.description().is_empty());
    }

    #[test]
    fn test_new_vs_default() {
        // For unit structs, new() and Default are equivalent
        let rule1 = NoteOrderingRule::new();
        let rule2 = NoteOrderingRule;
        assert_eq!(rule1.code(), rule2.code());
    }

    #[test]
    fn test_empty_file_passes() {
        let rule = NoteOrderingRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let file = make_file(TaskTree::new());
        let diagnostics = rule.check_file(&file, &ctx);
        assert!(diagnostics.is_empty());
    }
}
