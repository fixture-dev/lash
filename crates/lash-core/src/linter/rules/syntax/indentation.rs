//! Indentation consistency validation rule
//!
//! Validates that all checkbox lines use exactly 2 spaces per indentation level.
//!
//! **Note:** This validation is currently performed at parse time. The parser
//! normalizes indentation to depth levels and validates consistency. This rule
//! exists for completeness and documentation.

use lash_types::{Severity, Task, TaskFile};

use crate::linter::{LintContext, LintDiagnostic, LintRule};

/// Rule that validates consistent indentation
///
/// Checks that all checkbox lines use exactly 2 spaces per indentation level.
/// Mixed indentation (tabs, 4 spaces, etc.) is not allowed.
///
/// **Code:** `E_SYNTAX_INDENT`
/// **Severity:** Error
///
/// # Validation Location
///
/// This validation is performed during parsing in `parser::checkbox`. The parser
/// computes depth from indentation and can detect inconsistent indentation patterns.
///
/// # Auto-fix
///
/// The formatter normalizes all indentation to exactly 2 spaces per level based
/// on the computed depth from the task tree.
///
/// # Examples
///
/// Valid (2 spaces per level):
/// ```markdown
/// - [ ] Level 0
///   - [ ] Level 1 (2 spaces)
///     - [ ] Level 2 (4 spaces)
/// ```
///
/// Invalid (caught by parser/formatter):
/// ```markdown
/// - [ ] Level 0
///     - [ ] Level 1 (4 spaces - inconsistent)
/// - [ ] Level 0
/// \t- [ ] Level 1 (tab character)
/// ```
pub struct IndentationRule;

impl IndentationRule {
    /// Create a new indentation rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for IndentationRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRule for IndentationRule {
    fn code(&self) -> &'static str {
        "E_SYNTAX_INDENT"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> String {
        "Indentation consistency".to_string()
    }

    fn description(&self) -> &'static str {
        "Validates that all checkbox lines use exactly 2 spaces per indentation level"
    }

    fn check_file(&self, _file: &TaskFile, _ctx: &LintContext) -> Vec<LintDiagnostic> {
        // Validation performed at parse time
        // Parser validates indentation consistency when computing depth
        Vec::new()
    }

    fn check_task(&self, _task: &Task, _ctx: &LintContext) -> Vec<LintDiagnostic> {
        // Validation performed at parse time
        // If depth is correctly computed, indentation was valid
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::{FileMetadata, LashConfig, TaskTree};
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
    fn test_parsed_file_has_valid_indentation() {
        let rule = IndentationRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let file = lash_types::TaskFile {
            path: PathBuf::from("test.md"),
            title: "Test".to_string(),
            id: "test".to_string(),
            metadata: FileMetadata::default(),
            tasks: TaskTree::new(),
            hash: "hash".to_string(),
            mtime: SystemTime::now(),
        };

        // If parsing succeeded, indentation was valid
        let diagnostics = rule.check_file(&file, &ctx);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_rule_metadata() {
        let rule = IndentationRule::new();
        assert_eq!(rule.code(), "E_SYNTAX_INDENT");
        assert_eq!(rule.severity(), Severity::Error);
        assert_eq!(rule.name(), "Indentation consistency");
        assert!(!rule.description().is_empty());
    }
}
