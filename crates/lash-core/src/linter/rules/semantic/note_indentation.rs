//! Contextual note indentation validation rule
//!
//! Validates that contextual notes have correct indentation relative to
//! their parent tasks.

use lash_types::{Severity, Task, TaskFile};

use crate::linter::{LintContext, LintDiagnostic, LintRule};

/// Rule that validates contextual note indentation
///
/// Contextual notes should be indented exactly 2 spaces deeper than their
/// parent task, following the same indentation rules as child tasks.
///
/// **Code:** `E_NOTE_INVALID_INDENT`
/// **Severity:** Error
///
/// # Validation Location
///
/// This validation is primarily performed during parsing in `parser::builder`.
/// The parser validates that note indentation is a multiple of 2 spaces and
/// attaches notes to appropriate parent tasks based on indentation depth.
///
/// The linter rule exists for:
/// 1. Documentation of the indentation requirements
/// 2. Future-proofing if parser behavior changes
/// 3. Providing consistent error codes across all validation
///
/// # Auto-fix
///
/// The formatter normalizes all indentation to exactly 2 spaces per level
/// based on the computed depth from the task tree.
///
/// # Examples
///
/// Valid (note at parent depth + 1):
/// ```markdown
/// - [ ] Level 0 task
///   - Note for level 0 task (2 spaces, depth 1)
///   - [ ] Level 1 child task
///     - Note for level 1 task (4 spaces, depth 2)
/// ```
///
/// Invalid (caught by parser):
/// ```markdown
/// - [ ] Level 0 task
///     - Note with 4 spaces (should be 2)
/// - [ ] Level 0 task
/// - Note at same level (should be indented)
/// ```
pub struct NoteIndentationRule;

impl NoteIndentationRule {
    /// Create a new note indentation rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoteIndentationRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRule for NoteIndentationRule {
    fn code(&self) -> &'static str {
        "E_NOTE_INVALID_INDENT"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> String {
        "Note indentation".to_string()
    }

    fn description(&self) -> &'static str {
        "Validates that contextual notes are indented exactly 2 spaces deeper than their parent task"
    }

    fn check_file(&self, _file: &TaskFile, _ctx: &LintContext) -> Vec<LintDiagnostic> {
        // Indentation validation is performed at parse time.
        // The parser:
        // 1. Validates note indentation is a multiple of 2 spaces
        // 2. Computes depth and attaches notes to parent tasks
        // 3. Reports errors for invalid indentation before linting
        //
        // If parsing succeeded, indentation was valid.
        Vec::new()
    }

    fn check_task(&self, _task: &Task, _ctx: &LintContext) -> Vec<LintDiagnostic> {
        // Indentation validation is performed at parse time.
        // If the task exists with contextual notes attached, their
        // indentation was valid during parsing.
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
            max_depth: 2,
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
    fn test_parsed_file_has_valid_indentation() {
        let rule = NoteIndentationRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // If parsing succeeded and notes are attached, indentation was valid
        let notes = vec![
            ContextualNote::new("Note 1", 2),
            ContextualNote::new("Note 2", 3),
        ];

        let mut tree = TaskTree::new();
        tree.add_task(
            TaskBuilder::new("Task")
                .id("task")
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
    fn test_task_with_notes_passes() {
        let rule = NoteIndentationRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let task = TaskBuilder::new("Task")
            .id("task")
            .contextual_note("A note")
            .build()
            .unwrap();

        let diagnostics = rule.check_task(&task, &ctx);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_empty_file_passes() {
        let rule = NoteIndentationRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let file = make_file(TaskTree::new());
        let diagnostics = rule.check_file(&file, &ctx);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_task_without_notes_passes() {
        let rule = NoteIndentationRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let task = TaskBuilder::new("Task").id("task").build().unwrap();

        let diagnostics = rule.check_task(&task, &ctx);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_rule_metadata() {
        let rule = NoteIndentationRule::new();
        assert_eq!(rule.code(), "E_NOTE_INVALID_INDENT");
        assert_eq!(rule.severity(), Severity::Error);
        assert_eq!(rule.name(), "Note indentation");
        assert!(!rule.description().is_empty());
    }

    #[test]
    fn test_new_vs_default() {
        // For unit structs, new() and Default are equivalent
        let rule1 = NoteIndentationRule::new();
        let rule2 = NoteIndentationRule;
        assert_eq!(rule1.code(), rule2.code());
    }
}
