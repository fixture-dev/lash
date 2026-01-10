//! Contextual note ordering style rule
//!
//! As of issue #7, notes are allowed to be freely interleaved with child tasks
//! for structural flexibility. This rule is now disabled and always passes.
//!
//! Previously, this rule suggested that contextual notes should appear before
//! child tasks for better readability. However, users have requested the ability
//! to place notes anywhere among child tasks for better structural flexibility.

use lash_types::{Severity, TaskFile};

use crate::linter::{LintContext, LintDiagnostic, LintRule};

/// Rule that validates contextual note ordering convention
///
/// **Note:** As of issue #7, this rule is disabled. Notes can now be freely
/// interleaved with child tasks for structural flexibility.
///
/// **Code:** `W_NOTE_AFTER_CHILD_TASKS`
/// **Severity:** Warning (but always passes)
///
/// # Rationale for Disabling
///
/// While placing notes before child tasks provides a clean structure, users
/// have requested the ability to place contextual notes anywhere among child
/// tasks. This allows for more flexible documentation patterns like:
///
/// ```markdown
/// - [ ] Create Rust workspace structure
///   - Set up Cargo.toml as workspace root
///   - [ ] Create workspace Cargo.toml
///   - Configure shared dependencies (note between tasks)
///   - [ ] Create crate directories
///   - Final validation notes (after all tasks)
/// ```
///
/// This flexibility is particularly useful when notes relate to specific
/// adjacent tasks or when providing context for groups of tasks.
///
/// # Examples
///
/// All patterns are now valid:
/// ```markdown
/// - [ ] Parent task
///   - Note before children
///   - [ ] Child task 1
///   - Note between children
///   - [ ] Child task 2
///   - Note after children
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

    fn check_file(&self, _file: &TaskFile, _ctx: &LintContext) -> Vec<LintDiagnostic> {
        // As of issue #7, notes are allowed to be freely interleaved with child tasks.
        // This rule is disabled to provide structural flexibility.
        Vec::new()
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
    fn test_notes_after_children_passes() {
        let rule = NoteOrderingRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // Structure:
        // - [ ] Parent (line 1)
        //   - [ ] Child (line 2)
        //   - Note (line 3)  <- Note after child (now valid as of issue #7)
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

        // As of issue #7, this rule is disabled
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_multiple_notes_some_after_children() {
        let rule = NoteOrderingRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // Structure:
        // - [ ] Parent (line 1)
        //   - Note 1 (line 2)
        //   - [ ] Child (line 3)
        //   - Note 2 (line 4)  <- Now valid (issue #7)
        //   - Note 3 (line 5)  <- Now valid (issue #7)
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

        // As of issue #7, this rule is disabled
        assert!(diagnostics.is_empty());
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
    fn test_multiple_parents_with_notes_after_children() {
        let rule = NoteOrderingRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // Two parent tasks, both with notes after children (now valid as of issue #7)
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

        // As of issue #7, this rule is disabled
        assert!(diagnostics.is_empty());
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
