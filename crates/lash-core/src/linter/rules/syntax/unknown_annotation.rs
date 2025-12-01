//! Unknown annotation key validation rule
//!
//! Checks that all annotation keys are either built-in or explicitly
//! allowed via custom configuration. This prevents typos and maintains
//! a strict, predictable annotation schema.

use lash_types::{Severity, Task, TaskFile};

use crate::linter::{Fix, LintContext, LintDiagnostic, LintRule};

/// Rule that validates annotation keys are known
///
/// This rule ensures that all `@key:` annotations use either built-in keys
/// or custom keys explicitly allowed in the project configuration. This helps
/// catch typos and maintains schema strictness.
///
/// **Code:** `E_SYNTAX_UNKNOWN_KEY`
/// **Severity:** Error
///
/// # Built-in Annotation Keys
///
/// - `id` - Unique identifier
/// - `labels` - Task labels/tags
/// - `status` - Task status
/// - `owner` - Task assignee
/// - `created` - Creation date
/// - `estimate` - Time estimate
/// - `depends-on` - Dependencies
/// - `agent-note` - Notes for AI agents
///
/// # Custom Keys
///
/// Custom annotation keys can be registered in `.lash/config.toml`:
///
/// ```toml
/// [annotations]
/// custom_keys = ["priority", "sprint", "epic"]
/// ```
///
/// # Examples
///
/// Valid:
/// ```markdown
/// @id: task-1
/// @labels: backend, api
/// @custom: value  # If "custom" is in custom_keys
/// ```
///
/// Invalid:
/// ```markdown
/// @unknown: value  # Not built-in, not in custom_keys
/// @lables: typo    # Typo in built-in key name
/// ```
pub struct UnknownAnnotationRule;

impl UnknownAnnotationRule {
    /// Create a new unknown annotation rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Get fuzzy match suggestions for a potentially mistyped key
    ///
    /// Returns up to 3 suggestions based on edit distance
    fn suggest_corrections(key: &str) -> Vec<&'static str> {
        const BUILTIN: &[&str] = &[
            "id",
            "labels",
            "status",
            "owner",
            "created",
            "estimate",
            "depends-on",
            "agent-note",
        ];

        // Simple fuzzy matching based on prefix or substring
        let mut suggestions: Vec<&'static str> = BUILTIN
            .iter()
            .filter(|builtin| {
                // Suggest if key is a prefix or builtin is a prefix
                builtin.starts_with(key)
                    || key.starts_with(*builtin)
                    // Or if they share significant characters (>50%)
                    || {
                        let common: usize = key.chars().filter(|c| builtin.contains(*c)).count();
                        common * 2 > key.len().max(builtin.len())
                    }
            })
            .copied()
            .collect();

        // Sort by similarity (shorter keys first for prefix matches)
        suggestions.sort_by_key(|s| s.len());
        suggestions.truncate(3);
        suggestions
    }
}

impl Default for UnknownAnnotationRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRule for UnknownAnnotationRule {
    fn code(&self) -> &'static str {
        "E_SYNTAX_UNKNOWN_KEY"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> String {
        "Unknown annotation key".to_string()
    }

    fn description(&self) -> &'static str {
        "Validates that annotation keys are either built-in or explicitly allowed"
    }

    fn check_file(&self, file: &TaskFile, ctx: &LintContext) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        // Check file-level custom annotations
        for key in file.metadata.custom.keys() {
            if !ctx.is_annotation_allowed(key) {
                let suggestions = Self::suggest_corrections(key);

                // Build base diagnostic with help text
                let help = if suggestions.is_empty() {
                    format!(
                        "Unknown annotation '@{key}'. Add to .lash/config.toml [annotations.custom_keys] or remove it"
                    )
                } else {
                    format!(
                        "Unknown annotation '@{key}'. Add to .lash/config.toml [annotations.custom_keys] or fix typo\n  Did you mean: {}?",
                        suggestions
                            .iter()
                            .map(|s| format!("@{s}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };

                let mut diag = LintDiagnostic::error(
                    self.code(),
                    format!("Unknown annotation key: @{key}"),
                    ctx.file_path.clone(),
                    0, // Line number not available in parsed structure
                    0,
                )
                .with_help(help);

                // Add fix: prefer correction if available, otherwise deletion
                if let Some(best_match) = suggestions.first() {
                    // Fix: replace with best matching suggestion
                    diag = diag.with_fix(Fix::replace(
                        format!("Replace '@{key}' with '@{best_match}'"),
                        format!("@{key}:"),
                        format!("@{best_match}:"),
                    ));
                } else {
                    // Fix: remove the unknown annotation line
                    // Note: This requires the full line to be matched, including the value
                    // Since we don't have access to the value here, we can only suggest
                    // removing the key part
                    diag = diag.with_fix(Fix::replace(
                        format!("Remove unknown annotation '@{key}'"),
                        format!(
                            "@{key}: {}",
                            file.metadata.custom.get(key).unwrap_or(&String::new())
                        ),
                        String::new(),
                    ));
                }

                diagnostics.push(diag);
            }
        }

        diagnostics
    }

    fn check_task(&self, _task: &Task, _ctx: &LintContext) -> Vec<LintDiagnostic> {
        // Check task-level custom annotations (if any - currently TaskMetadata doesn't expose custom)
        // This would check task.metadata.custom if it existed
        // For now, custom annotations are only at file level in our data model
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

    fn make_config(custom_keys: Vec<String>) -> LashConfig {
        LashConfig {
            root_path: PathBuf::from("/test"),
            index_file: "index.md".to_string(),
            max_depth: 2,
            indent_spaces: 2,
            db_path: PathBuf::from(".lash/test.db"),
            custom_annotation_keys: custom_keys,
        }
    }

    fn make_file(custom: HashMap<String, String>) -> TaskFile {
        TaskFile {
            path: PathBuf::from("test.md"),
            title: "Test".to_string(),
            id: "test".to_string(),
            metadata: FileMetadata {
                custom,
                ..Default::default()
            },
            tasks: TaskTree::new(),
            hash: "hash".to_string(),
            mtime: SystemTime::now(),
        }
    }

    #[test]
    fn test_no_custom_annotations() {
        let rule = UnknownAnnotationRule::new();
        let config = make_config(vec![]);
        let file = make_file(HashMap::new());
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let diagnostics = rule.check_file(&file, &ctx);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_allowed_custom_annotation() {
        let rule = UnknownAnnotationRule::new();
        let config = make_config(vec!["priority".to_string()]);

        let mut custom = HashMap::new();
        custom.insert("priority".to_string(), "high".to_string());
        let file = make_file(custom);

        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let diagnostics = rule.check_file(&file, &ctx);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_unknown_custom_annotation() {
        let rule = UnknownAnnotationRule::new();
        let config = make_config(vec![]);

        let mut custom = HashMap::new();
        custom.insert("unknown".to_string(), "value".to_string());
        let file = make_file(custom);

        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_SYNTAX_UNKNOWN_KEY");
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert!(diagnostics[0].message.contains("unknown"));
        assert!(diagnostics[0].help.is_some());
    }

    #[test]
    fn test_multiple_unknown_annotations() {
        let rule = UnknownAnnotationRule::new();
        let config = make_config(vec![]);

        let mut custom = HashMap::new();
        custom.insert("foo".to_string(), "1".to_string());
        custom.insert("bar".to_string(), "2".to_string());
        custom.insert("baz".to_string(), "3".to_string());
        let file = make_file(custom);

        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 3);
    }

    #[test]
    fn test_mixed_known_and_unknown() {
        let rule = UnknownAnnotationRule::new();
        let config = make_config(vec!["allowed".to_string()]);

        let mut custom = HashMap::new();
        custom.insert("allowed".to_string(), "ok".to_string());
        custom.insert("unknown".to_string(), "not ok".to_string());
        let file = make_file(custom);

        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("unknown"));
    }

    #[test]
    fn test_fuzzy_suggestions() {
        // Test typo suggestions
        let suggestions = UnknownAnnotationRule::suggest_corrections("lables"); // typo of "labels"
        assert!(suggestions.contains(&"labels"));

        let suggestions = UnknownAnnotationRule::suggest_corrections("statu"); // partial "status"
        assert!(suggestions.contains(&"status"));

        let suggestions = UnknownAnnotationRule::suggest_corrections("depend"); // partial "depends-on"
        assert!(suggestions.contains(&"depends-on"));
    }

    #[test]
    fn test_suggestions_in_diagnostic() {
        let rule = UnknownAnnotationRule::new();
        let config = make_config(vec![]);

        let mut custom = HashMap::new();
        custom.insert("lables".to_string(), "typo".to_string()); // Typo of "labels"
        let file = make_file(custom);

        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0]
            .help
            .as_ref()
            .unwrap()
            .contains("Did you mean"));
        assert!(diagnostics[0].help.as_ref().unwrap().contains("@labels"));
    }

    #[test]
    fn test_rule_metadata() {
        let rule = UnknownAnnotationRule::new();
        assert_eq!(rule.code(), "E_SYNTAX_UNKNOWN_KEY");
        assert_eq!(rule.severity(), Severity::Error);
        assert!(!rule.description().is_empty());
    }

    #[test]
    fn test_fix_generation_with_suggestion() {
        let rule = UnknownAnnotationRule::new();
        let config = make_config(vec![]);

        let mut custom = HashMap::new();
        custom.insert("lables".to_string(), "typo".to_string()); // Typo of "labels"
        let file = make_file(custom);

        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].has_fix());

        // Verify the fix suggests the correct replacement
        let fix = diagnostics[0].fix.as_ref().unwrap();
        assert!(fix.description.contains("labels"));
        if let crate::linter::Replacement::TextReplace { old, new } = &fix.replacement {
            assert_eq!(old, "@lables:");
            assert_eq!(new, "@labels:");
        } else {
            panic!("Expected TextReplace fix");
        }
    }

    #[test]
    fn test_fix_generation_without_suggestion() {
        let rule = UnknownAnnotationRule::new();
        let config = make_config(vec![]);

        let mut custom = HashMap::new();
        custom.insert("xyz123".to_string(), "value".to_string()); // No similar built-in
        let file = make_file(custom);

        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].has_fix());

        // Verify the fix suggests removal
        let fix = diagnostics[0].fix.as_ref().unwrap();
        assert!(fix.description.contains("Remove"));
        if let crate::linter::Replacement::TextReplace { old, new } = &fix.replacement {
            assert_eq!(old, "@xyz123: value");
            assert_eq!(new, "");
        } else {
            panic!("Expected TextReplace fix for removal");
        }
    }
}
