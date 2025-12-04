//! Annotation ordering validation rule
//!
//! Suggests alphabetical ordering of annotations for consistency.
//!
//! **Note:** This is an informational rule that requires raw file content
//! to check and fix annotation order. Currently implemented as a stub.

use lash_types::{Severity, Task, TaskFile};

use crate::linter::{LintContext, LintDiagnostic, LintRule};

/// Rule that suggests alphabetical annotation ordering
///
/// This is an optional style rule that suggests ordering annotations
/// alphabetically for consistency. It has Info severity since ordering
/// doesn't affect functionality.
///
/// **Code:** `I_SYNTAX_ORDER`
/// **Severity:** Info
///
/// # Auto-fix
///
/// The formatter can sort annotations alphabetically, with `@id` always first
/// as a special case.
///
/// # Implementation Note
///
/// This rule requires access to raw file content to check the actual order
/// of annotation lines, as the parsed structure (`HashMap`) doesn't preserve order.
///
/// Current implementation is a stub. Full implementation would require either:
/// 1. Adding a `check_raw_content` method to the `LintRule` trait
/// 2. Preserving annotation order in the parsed structure
/// 3. Deferring this check to the formatter
///
/// # Examples
///
/// Preferred order:
/// ```markdown
/// @id: task-1
/// @agent-note: Important context
/// @created: 2024-01-15
/// @depends-on: other-task
/// @estimate: 2h
/// @labels: backend, api
/// @owner: alice
/// @status: in-progress
/// ```
///
/// Out of order (still valid, just inconsistent):
/// ```markdown
/// @id: task-1
/// @status: in-progress
/// @labels: backend, api
/// @owner: alice
/// ```
pub struct AnnotationOrderRule;

impl AnnotationOrderRule {
    /// Create a new annotation order rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for AnnotationOrderRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRule for AnnotationOrderRule {
    fn code(&self) -> &'static str {
        "I_SYNTAX_ORDER"
    }

    fn severity(&self) -> Severity {
        Severity::Info
    }

    fn name(&self) -> String {
        "Annotation ordering".to_string()
    }

    fn description(&self) -> &'static str {
        "Suggests alphabetical ordering of annotations for consistency"
    }

    fn check_file(&self, _file: &TaskFile, _ctx: &LintContext) -> Vec<LintDiagnostic> {
        // Requires raw content access to check annotation order
        // Currently handled by formatter if enabled
        Vec::new()
    }

    fn check_task(&self, _task: &Task, _ctx: &LintContext) -> Vec<LintDiagnostic> {
        // Requires raw content access
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::LashConfig;
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

    #[test]
    fn test_rule_metadata() {
        let rule = AnnotationOrderRule::new();
        assert_eq!(rule.code(), "I_SYNTAX_ORDER");
        assert_eq!(rule.severity(), Severity::Info);
        assert_eq!(rule.name(), "Annotation ordering");
        assert!(!rule.description().is_empty());
    }

    #[test]
    fn test_no_diagnostics_without_raw_content() {
        use lash_types::{FileMetadata, TaskTree};
        use std::time::SystemTime;

        let rule = AnnotationOrderRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // Since we don't have raw content access, no diagnostics are generated
        // This is handled by the formatter instead

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

        let diagnostics = rule.check_file(&file, &ctx);
        assert!(diagnostics.is_empty());
    }
}
