//! Contextual note nesting validation rule
//!
//! Ensures that contextual notes do not have children. Notes are terminal
//! items that provide context, not containers for further nesting.

use lash_types::{Severity, TaskFile};

use crate::linter::{LintContext, LintDiagnostic, LintRule};

/// Rule that validates contextual notes don't have children
///
/// Contextual notes are informational items that should not contain
/// nested items (neither tasks nor other notes). This rule detects
/// when items appear to be nested under notes and reports an error.
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
/// This validation is primarily handled by the parser, which attaches
/// notes to their parent tasks. However, this rule provides an additional
/// semantic check and clear error message if the parser's behavior changes.
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
/// Invalid (detected by parser, validated here):
/// ```markdown
/// - [ ] Parent task
///   - Note providing context
///     - [ ] This would be a child of a note (invalid)
/// ```
///
/// Better approach:
/// ```markdown
/// - [ ] Parent task
///   - Note providing context
/// - [ ] Separate task for hierarchical structure
///   - [ ] Child task
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

    fn check_file(&self, file: &TaskFile, ctx: &LintContext) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        let tasks = file.tasks.tasks();

        // For each task with contextual notes, check if any child tasks
        // appear at a position that would logically be "under" a note.
        //
        // The semantic rule we're checking: if a task has notes, and then
        // has children that come AFTER those notes in document order, that's
        // allowed. But if we detect nesting anomalies, we report them.
        //
        // Since the parser already handles note attachment, this rule
        // primarily catches edge cases and provides semantic validation.
        for task in tasks {
            if task.contextual_notes.is_empty() {
                continue;
            }

            // Get the line number of the last note
            let last_note_line = task
                .contextual_notes
                .iter()
                .map(lash_types::ContextualNote::line_number)
                .max()
                .unwrap_or(0);

            // Get all direct children of this task
            let children: Vec<_> = tasks
                .iter()
                .filter(|t| t.parent_id.as_deref() == Some(&task.id))
                .collect();

            // Check for children that appear between notes
            // This would indicate improper nesting that the parser allowed
            for child in &children {
                // If a child task appears after a note but at a depth that
                // would make it look like it's nested under the note,
                // that's suspicious but currently allowed by the parser.
                //
                // The parser attaches notes to the most recent valid parent,
                // so true "note has children" cases are prevented at parse time.
                // This rule exists for documentation and future-proofing.

                // For now, we check a specific case: if there are notes AND
                // children where the first child appears BEFORE the last note
                // in document order, that suggests interleaving which could
                // be confusing.
                if child.line_number > 0
                    && last_note_line > 0
                    && child.line_number < last_note_line
                    && child.depth == task.depth + 1
                {
                    // A child task appears before a note ends
                    // This means notes and tasks are interleaved
                    let diag = LintDiagnostic::error(
                        "E_NOTE_HAS_CHILDREN",
                        format!(
                            "Task '{}' has notes interleaved with child tasks",
                            task.title
                        ),
                        ctx.file_path.clone(),
                        child.line_number,
                        0,
                    )
                    .with_help(
                        "Move all contextual notes before child tasks, or convert notes to tasks if they need children",
                    );

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
    fn test_interleaved_notes_and_children_errors() {
        let rule = NoteNestingRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // Structure:
        // - [ ] Parent (line 1)
        //   - [ ] Child (line 2)  <- Child appears before last note
        //   - Note (line 3)       <- Note appears after child
        // This is interleaved and should trigger an error
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
        assert_eq!(diagnostics[0].code, "E_NOTE_HAS_CHILDREN");
        assert_eq!(diagnostics[0].severity, Severity::Error);
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
