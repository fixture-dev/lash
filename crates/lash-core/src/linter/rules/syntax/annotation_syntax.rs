//! Annotation syntax validation rule
//!
//! Validates that annotation lines match the required format: `@key: value`
//!
//! **Note:** This validation is currently performed at parse time. The parser
//! only accepts valid annotation syntax. This rule exists for completeness.

use lash_types::{Severity, Task, TaskFile};

use crate::linter::{LintContext, LintDiagnostic, LintRule};

/// Rule that validates annotation syntax
///
/// Checks that lines starting with `@` match the pattern `@key: value`:
/// - Starts with `@` (no space before)
/// - Followed by alphanumeric key (with `-` allowed)
/// - Then `: ` (colon and space)
/// - Then the value (rest of line)
///
/// **Code:** `E_SYNTAX_ANNOTATION`
/// **Severity:** Error
///
/// # Validation Location
///
/// This validation is performed during parsing in `parser::annotations`. The
/// parser only accepts valid annotation syntax and generates diagnostics for
/// invalid patterns.
///
/// # Auto-fix
///
/// For common mistakes like missing colon, the fix would be to add `: ` after
/// the key. This could be implemented with raw content access.
///
/// # Examples
///
/// Valid:
/// ```markdown
/// @id: task-123
/// @labels: backend, api
/// @depends-on: other-task
/// ```
///
/// Invalid (caught by parser):
/// ```markdown
/// @id task-123        # Missing colon
/// @ id: task-123      # Space after @
/// @: task-123         # Missing key
/// ```
pub struct AnnotationSyntaxRule;

impl AnnotationSyntaxRule {
    /// Create a new annotation syntax rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for AnnotationSyntaxRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRule for AnnotationSyntaxRule {
    fn code(&self) -> &'static str {
        "E_SYNTAX_ANNOTATION"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> String {
        "Annotation syntax".to_string()
    }

    fn description(&self) -> &'static str {
        "Validates that annotation lines match the format: @key: value"
    }

    fn check_file(&self, _file: &TaskFile, _ctx: &LintContext) -> Vec<LintDiagnostic> {
        // Validation performed at parse time
        // If annotations exist in metadata, they had valid syntax
        Vec::new()
    }

    fn check_task(&self, _task: &Task, _ctx: &LintContext) -> Vec<LintDiagnostic> {
        // Validation performed at parse time
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
    fn test_parsed_file_has_valid_annotations() {
        let rule = AnnotationSyntaxRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let file = lash_types::TaskFile {
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

        // If parsing succeeded, annotation syntax was valid
        let diagnostics = rule.check_file(&file, &ctx);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_rule_metadata() {
        let rule = AnnotationSyntaxRule::new();
        assert_eq!(rule.code(), "E_SYNTAX_ANNOTATION");
        assert_eq!(rule.severity(), Severity::Error);
        assert_eq!(rule.name(), "Annotation syntax");
        assert!(!rule.description().is_empty());
    }
}
