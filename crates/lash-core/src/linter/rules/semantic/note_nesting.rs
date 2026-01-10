//! Contextual note nesting validation rule
//!
//! Ensures that contextual notes do not have children. Notes are terminal
//! items that provide context, not containers for further nesting.
//!
//! As of issue #7, notes are allowed to be interleaved with child tasks
//! for structural flexibility. This rule only validates that notes themselves
//! don't have nested children, which the parser already prevents.

use lash_types::{Severity, TaskFile};

use crate::linter::{LintContext, LintDiagnostic, LintRule};

/// Rule that validates contextual notes don't have children
///
/// Contextual notes are informational items that should not contain
/// nested items (neither tasks nor other notes). This validation is
/// primarily handled by the parser, which attaches notes to their parent
/// tasks and prevents nesting under notes.
///
/// **Code:** `E_NOTE_HAS_CHILDREN`
/// **Severity:** Error
///
/// # Rationale
///
/// Notes are meant to provide quick context or requirements for their
/// parent task. If you need to create a hierarchical structure, you
/// should use tasks (with checkboxes) instead of plain bullet notes.
///
/// # Note
///
/// As of issue #7, notes can be freely interleaved with child tasks
/// for better structural flexibility. For example:
///
/// ```markdown
/// - [ ] Parent task
///   - Setup note
///   - [ ] Child task 1
///   - Configuration note (between children)
///   - [ ] Child task 2
///   - Final note (after children)
/// ```
///
/// This rule only prevents actual nesting under notes, which the parser
/// already handles by attaching notes to the most recent valid parent task.
///
/// # Examples
///
/// Valid:
/// ```markdown
/// - [ ] Parent task
///   - Note providing context
///   - Another note
///   - [ ] Child task
/// ```
///
/// Invalid (detected by parser):
/// ```markdown
/// - [ ] Parent task
///   - Note providing context
///     - [ ] This would be a child of a note (invalid)
/// ```
pub struct NoteNestingRule;

impl NoteNestingRule {
    /// Create a new note nesting rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoteNestingRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRule for NoteNestingRule {
    fn code(&self) -> &'static str {
        "E_NOTE_HAS_CHILDREN"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> String {
        "Note nesting".to_string()
    }

    fn description(&self) -> &'static str {
        "Ensures contextual notes do not have nested children"
    }

    fn check_file(&self, _file: &TaskFile, _ctx: &LintContext) -> Vec<LintDiagnostic> {
        // As of issue #7, notes are allowed to be freely interleaved with child tasks.
        // The parser already prevents actual nesting under notes (e.g., a note having
        // a child task directly beneath it at increased depth).
        //
        // Since the parser handles the only invalid case (true nesting under notes),
        // and interleaving is now explicitly allowed, this rule returns no diagnostics.
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
    fn test_task_without_notes_passes() {
        let rule = NoteNestingRule::new();
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
    fn test_task_with_notes_then_children_passes() {
        let rule = NoteNestingRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // Structure:
        // - [ ] Parent (line 1)
        //   - Note (line 2)
        //   - [ ] Child (line 3)
        // Notes come BEFORE children in document order - this is valid
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
        assert!(diagnostics.is_empty(), "Notes before children should pass");
    }

    #[test]
    fn test_interleaved_notes_and_children_passes() {
        let rule = NoteNestingRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // Structure:
        // - [ ] Parent (line 1)
        //   - [ ] Child (line 2)  <- Child appears before note
        //   - Note (line 3)       <- Note appears after child
        // As of issue #7, this is now valid - interleaving is allowed
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
        assert!(
            diagnostics.is_empty(),
            "Interleaved notes and children are now allowed as of issue #7"
        );
    }

    #[test]
    fn test_multiple_notes_before_children_passes() {
        let rule = NoteNestingRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // Structure:
        // - [ ] Parent (line 1)
        //   - Note 1 (line 2)
        //   - Note 2 (line 3)
        //   - [ ] Child (line 4)
        let notes = vec![
            ContextualNote::new("Note 1", 2),
            ContextualNote::new("Note 2", 3),
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
                .line_number(4)
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
        let rule = NoteNestingRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // Without line numbers (0), we can't detect ordering issues
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
    fn test_rule_metadata() {
        let rule = NoteNestingRule::new();
        assert_eq!(rule.code(), "E_NOTE_HAS_CHILDREN");
        assert_eq!(rule.severity(), Severity::Error);
        assert_eq!(rule.name(), "Note nesting");
        assert!(!rule.description().is_empty());
    }

    #[test]
    fn test_new_vs_default() {
        // For unit structs, new() and Default are equivalent
        let rule1 = NoteNestingRule::new();
        let rule2 = NoteNestingRule;
        assert_eq!(rule1.code(), rule2.code());
    }

    #[test]
    fn test_empty_file_passes() {
        let rule = NoteNestingRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let file = make_file(TaskTree::new());
        let diagnostics = rule.check_file(&file, &ctx);
        assert!(diagnostics.is_empty());
    }
}
