//! Contextual note length validation rule
//!
//! Ensures that contextual notes don't exceed reasonable length limits
//! to maintain readability and encourage concise documentation.

use lash_types::{
    task::{NOTE_LENGTH_ERROR_THRESHOLD, NOTE_LENGTH_WARNING_THRESHOLD},
    Severity, Task,
};

use crate::linter::{LintContext, LintDiagnostic, LintRule};

/// Rule that validates contextual note length
///
/// Contextual notes should be concise. This rule enforces length limits
/// with a warning for long notes and an error for excessively long notes.
///
/// **Thresholds:**
/// - Warning at 200 characters (`W_NOTE_TOO_LONG`)
/// - Error at 500 characters (hard limit) (`E_NOTE_EXCESSIVE_LENGTH`)
///
/// **Code:** `W_NOTE_TOO_LONG` or `E_NOTE_EXCESSIVE_LENGTH`
/// **Severity:** Warning (201-500 chars) or Error (>500 chars)
///
/// # Rationale
///
/// Long notes in task files can:
/// - Make files hard to navigate and read
/// - Reduce the effectiveness of notes as quick context
/// - Increase token usage for AI agents
///
/// For detailed content, consider breaking it into multiple notes or
/// converting the content to child tasks.
///
/// # Examples
///
/// Valid (under warning threshold):
/// ```markdown
/// - [ ] Implement feature
///   - Use library X for parsing
///   - Target < 100ms latency
/// ```
///
/// Warning (over 200 chars):
/// ```markdown
/// - [ ] Implement feature
///   - This is a very long note that goes into excessive detail about the
///     implementation requirements, design decisions, and various edge cases
///     that need to be considered. It probably should be broken up or moved
///     to documentation.
/// ```
///
/// Error (over 500 chars - hard limit):
/// ```markdown
/// - [ ] Implement feature
///   - [500+ characters - this should be in separate notes or docs]
/// ```
pub struct NoteLengthRule;

impl NoteLengthRule {
    /// Create a new note length rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoteLengthRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRule for NoteLengthRule {
    fn code(&self) -> &'static str {
        // Return warning code by default - check_task will override for errors
        "W_NOTE_TOO_LONG"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> String {
        "Note length".to_string()
    }

    fn description(&self) -> &'static str {
        "Ensures contextual notes are concise and within reasonable length limits"
    }

    fn check_task(&self, task: &Task, ctx: &LintContext) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        for note in &task.contextual_notes {
            let len = note.len();
            let line = note.line_number();

            // Check if note exceeds error threshold (hard limit)
            if note.exceeds_error_threshold() {
                let diag = LintDiagnostic::error(
                    "E_NOTE_EXCESSIVE_LENGTH",
                    format!(
                        "Contextual note exceeds hard limit ({len} characters, max {NOTE_LENGTH_ERROR_THRESHOLD})"
                    ),
                    ctx.file_path.clone(),
                    line,
                    0,
                )
                .with_snippet(note.truncated_text(60))
                .with_help("Break the note into multiple shorter notes or move content to documentation");

                diagnostics.push(diag);
            }
            // Check if note exceeds warning threshold
            else if note.exceeds_warning_threshold() {
                let diag = LintDiagnostic::warning(
                    "W_NOTE_TOO_LONG",
                    format!(
                        "Contextual note is long ({len} characters, recommended max {NOTE_LENGTH_WARNING_THRESHOLD})"
                    ),
                    ctx.file_path.clone(),
                    line,
                    0,
                )
                .with_snippet(note.truncated_text(60))
                .with_help("Consider breaking the note into multiple shorter notes");

                diagnostics.push(diag);
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::{task::ContextualNote, LashConfig, TaskBuilder, TaskStatus};
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

    fn make_task_with_note(note_text: &str, line: usize) -> Task {
        TaskBuilder::new("Test task")
            .id("test-task")
            .status(TaskStatus::Open)
            .contextual_note_with_line(note_text, line)
            .build()
            .unwrap()
    }

    #[test]
    fn test_short_note_passes() {
        let rule = NoteLengthRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let task = make_task_with_note("Short note", 5);
        let diagnostics = rule.check_task(&task, &ctx);
        assert!(diagnostics.is_empty(), "Short note should pass");
    }

    #[test]
    fn test_exactly_warning_threshold_passes() {
        let rule = NoteLengthRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // Exactly 200 characters (at threshold, should pass)
        let note = "a".repeat(NOTE_LENGTH_WARNING_THRESHOLD);
        let task = make_task_with_note(&note, 5);
        let diagnostics = rule.check_task(&task, &ctx);
        assert!(
            diagnostics.is_empty(),
            "Note at exactly warning threshold should pass"
        );
    }

    #[test]
    fn test_over_warning_threshold_warns() {
        let rule = NoteLengthRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // 201 characters (just over warning threshold)
        let note = "a".repeat(NOTE_LENGTH_WARNING_THRESHOLD + 1);
        let task = make_task_with_note(&note, 5);
        let diagnostics = rule.check_task(&task, &ctx);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "W_NOTE_TOO_LONG");
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert!(diagnostics[0].message.contains("201"));
        assert!(diagnostics[0].message.contains("200"));
        assert!(diagnostics[0].help.is_some());
    }

    #[test]
    fn test_exactly_error_threshold_warns() {
        let rule = NoteLengthRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // Exactly 500 characters (at error threshold, should warn not error)
        let note = "a".repeat(NOTE_LENGTH_ERROR_THRESHOLD);
        let task = make_task_with_note(&note, 5);
        let diagnostics = rule.check_task(&task, &ctx);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "W_NOTE_TOO_LONG");
        assert_eq!(diagnostics[0].severity, Severity::Warning);
    }

    #[test]
    fn test_over_error_threshold_errors() {
        let rule = NoteLengthRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // 501 characters (over hard limit)
        let note = "a".repeat(NOTE_LENGTH_ERROR_THRESHOLD + 1);
        let task = make_task_with_note(&note, 5);
        let diagnostics = rule.check_task(&task, &ctx);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_NOTE_EXCESSIVE_LENGTH");
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert!(diagnostics[0].message.contains("501"));
        assert!(diagnostics[0].message.contains("500"));
        assert!(diagnostics[0].help.is_some());
    }

    #[test]
    fn test_multiple_notes() {
        let rule = NoteLengthRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let notes = vec![
            ContextualNote::new("Short note", 5),
            ContextualNote::new("a".repeat(250), 6), // Warning
            ContextualNote::new("b".repeat(550), 7), // Error
        ];

        let task = TaskBuilder::new("Test task")
            .id("test-task")
            .contextual_notes(notes)
            .build()
            .unwrap();

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 2);

        // Check we got one warning and one error
        assert!(diagnostics.iter().any(|d| d.code == "W_NOTE_TOO_LONG"));
        assert!(diagnostics
            .iter()
            .any(|d| d.code == "E_NOTE_EXCESSIVE_LENGTH"));
    }

    #[test]
    fn test_no_notes_passes() {
        let rule = NoteLengthRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let task = TaskBuilder::new("Test task")
            .id("test-task")
            .build()
            .unwrap();

        let diagnostics = rule.check_task(&task, &ctx);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_diagnostic_includes_line_number() {
        let rule = NoteLengthRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let note = "a".repeat(300);
        let task = make_task_with_note(&note, 42);
        let diagnostics = rule.check_task(&task, &ctx);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].location.line, Some(42));
    }

    #[test]
    fn test_diagnostic_includes_snippet() {
        let rule = NoteLengthRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let note = "a".repeat(300);
        let task = make_task_with_note(&note, 5);
        let diagnostics = rule.check_task(&task, &ctx);

        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].snippet.is_some());
        // Snippet should be truncated
        let snippet = diagnostics[0].snippet.as_ref().unwrap();
        assert!(snippet.len() <= 63); // 60 + "..."
    }

    #[test]
    fn test_rule_metadata() {
        let rule = NoteLengthRule::new();
        assert_eq!(rule.code(), "W_NOTE_TOO_LONG");
        assert_eq!(rule.severity(), Severity::Warning);
        assert_eq!(rule.name(), "Note length");
        assert!(!rule.description().is_empty());
    }

    #[test]
    fn test_new_vs_default() {
        // For unit structs, new() and Default are equivalent
        let rule1 = NoteLengthRule::new();
        let rule2 = NoteLengthRule;
        assert_eq!(rule1.code(), rule2.code());
    }
}
