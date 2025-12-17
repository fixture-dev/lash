//! Checkbox pattern validation rule
//!
//! Validates that checkbox syntax matches the required pattern: `- [ ]`, `- [x]`, etc.
//!
//! **Note:** This validation is currently performed at parse time by the checkbox
//! parser. Invalid checkbox patterns fail to parse and generate parser diagnostics.
//! This rule exists for completeness and to document the validation logic.

use lash_types::{Severity, Task, TaskFile};

use crate::linter::{LintContext, LintDiagnostic, LintRule};

/// Rule that validates checkbox pattern syntax
///
/// Checks that checkbox lines match the pattern: `- [X]` where X is one of:
/// - ` ` (space) - Open task
/// - `x` or `X` - Done task
/// - `-` - Waived task
/// - `!` - Blocked task
///
/// **Code:** `E_SYNTAX_CHECKBOX`
/// **Severity:** Error
///
/// # Validation Location
///
/// This validation is performed during parsing in `parser::checkbox`. The parser
/// only accepts valid checkbox patterns and generates diagnostics for invalid ones.
///
/// This lint rule serves as documentation and could be extended to perform
/// additional post-parse validation if needed.
///
/// # Examples
///
/// Valid patterns:
/// ```markdown
/// - [ ] Open task
/// - [x] Done task
/// - [X] Done task (capital X)
/// - [-] Waived task
/// - [!] Blocked task
/// ```
///
/// Invalid patterns (caught by parser):
/// ```markdown
/// - [] Missing space
/// - [ x] Extra space
/// - [v] Invalid character
/// - [*] Invalid character
/// ```
pub struct CheckboxPatternRule;

impl CheckboxPatternRule {
    /// Create a new checkbox pattern rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for CheckboxPatternRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRule for CheckboxPatternRule {
    fn code(&self) -> &'static str {
        "E_SYNTAX_CHECKBOX"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> String {
        "Checkbox pattern".to_string()
    }

    fn description(&self) -> &'static str {
        "Validates checkbox syntax: - [X] where X is space, x, -, or !"
    }

    fn check_file(&self, _file: &TaskFile, _ctx: &LintContext) -> Vec<LintDiagnostic> {
        // Validation performed at parse time
        // If the file parsed successfully, all checkboxes are valid
        Vec::new()
    }

    fn check_task(&self, _task: &Task, _ctx: &LintContext) -> Vec<LintDiagnostic> {
        // Validation performed at parse time
        // If the task exists in the parsed tree, its checkbox was valid
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::{
        FileMetadata, LashConfig, Task, TaskFile, TaskMetadata, TaskStatus, TaskTree,
    };
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

    #[test]
    fn test_parsed_file_has_valid_checkboxes() {
        // If parsing succeeded, checkboxes were valid
        let rule = CheckboxPatternRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let file = TaskFile {
            path: PathBuf::from("test.md"),
            title: "Test".to_string(),
            id: "test".to_string(),
            metadata: FileMetadata::default(),
            description: None,
            description_agent_notes: Vec::new(),
            tasks: TaskTree::new(),
            hash: "hash".to_string(),
            mtime: SystemTime::now(),
        };

        let diagnostics = rule.check_file(&file, &ctx);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_parsed_task_has_valid_checkbox() {
        let rule = CheckboxPatternRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let task = Task {
            id: "task-1".to_string(),
            has_explicit_id: false,
            title: "Test task".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            metadata: TaskMetadata::default(),
            body: None,
            contextual_notes: Vec::new(),
        };

        let diagnostics = rule.check_task(&task, &ctx);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_rule_metadata() {
        let rule = CheckboxPatternRule::new();
        assert_eq!(rule.code(), "E_SYNTAX_CHECKBOX");
        assert_eq!(rule.severity(), Severity::Error);
        assert_eq!(rule.name(), "Checkbox pattern");
        assert!(!rule.description().is_empty());
    }
}
