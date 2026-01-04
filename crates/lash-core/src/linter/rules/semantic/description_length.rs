//! Description length validation rule
//!
//! Ensures that file descriptions don't exceed reasonable length limits
//! to maintain readability and encourage proper documentation structure.

use lash_types::{Severity, TaskFile};

use crate::linter::{LintContext, LintDiagnostic, LintRule};

/// Rule that validates description length
///
/// Descriptions should be concise overviews. Long descriptions should be
/// moved to dedicated documentation files and linked from the task file.
///
/// **Default Thresholds:**
/// - Warning at 1000 characters (`W_SEM_DESC_TOO_LONG`)
/// - Error at 2000 characters (hard limit) (`E_SEM_DESC_TOO_LONG`)
///
/// **Code:** `W_SEM_DESC_TOO_LONG` or `E_SEM_DESC_TOO_LONG`
/// **Severity:** Warning (1000-2000 chars) or Error (>2000 chars)
///
/// # Rationale
///
/// Long descriptions in task files can:
/// - Make files hard to navigate and read
/// - Bloat task list views in the TUI
/// - Increase token usage for AI agents
///
/// For detailed content, use the `@doc` annotation to link to external
/// documentation files.
///
/// # Examples
///
/// Valid (under warning threshold):
/// ```markdown
/// # Feature Implementation
///
/// @id: feature.auth
///
/// ## Description
///
/// Implement OAuth2 authentication flow for the API. This includes token
/// generation, validation, and refresh mechanisms.
/// ```
///
/// Warning (over 1000 chars):
/// ```markdown
/// ## Description
///
/// [1000+ characters of detailed implementation notes, architecture
/// decisions, API specifications, etc.]
/// ```
///
/// Error (over 2000 chars - hard limit):
/// ```markdown
/// ## Description
///
/// [2000+ characters - this should be in a separate doc file]
/// ```
///
/// Better approach:
/// ```markdown
/// ## Description
///
/// Implement OAuth2 authentication. See linked documentation for details.
///
/// @doc: docs/auth-implementation.md
/// ```
pub struct DescriptionLengthRule {
    warning_threshold: usize,
    error_threshold: usize,
}

impl DescriptionLengthRule {
    /// Create a new description length rule with default thresholds
    ///
    /// - Warning threshold: 1000 characters
    /// - Error threshold: 2000 characters
    #[must_use]
    pub fn new() -> Self {
        Self {
            warning_threshold: 1000,
            error_threshold: 2000,
        }
    }

    /// Create a rule with custom thresholds
    ///
    /// # Arguments
    ///
    /// * `warning` - Character count that triggers a warning
    /// * `error` - Character count that triggers an error (hard limit)
    ///
    /// # Panics
    ///
    /// Panics if `warning >= error` (error threshold must be higher)
    #[must_use]
    pub fn with_thresholds(warning: usize, error: usize) -> Self {
        assert!(
            warning < error,
            "Warning threshold must be less than error threshold"
        );
        Self {
            warning_threshold: warning,
            error_threshold: error,
        }
    }
}

impl Default for DescriptionLengthRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRule for DescriptionLengthRule {
    fn code(&self) -> &'static str {
        // Return warning code by default - check_file will override for errors
        "W_SEM_DESC_TOO_LONG"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> String {
        "Description length".to_string()
    }

    fn description(&self) -> &'static str {
        "Ensures descriptions are concise and within reasonable length limits"
    }

    fn check_file(&self, file: &TaskFile, ctx: &LintContext) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        // No description means no issue
        let Some(description) = &file.description else {
            return diagnostics;
        };

        let len = description.len();

        // Check if description exceeds error threshold (hard limit)
        if len > self.error_threshold {
            let diag = LintDiagnostic::error(
                "E_SEM_DESC_TOO_LONG",
                format!(
                    "Description exceeds hard limit ({len} characters, max {max})",
                    len = len,
                    max = self.error_threshold
                ),
                ctx.file_path.clone(),
                0,
                0,
            )
            .with_help(
                "Consider moving detailed content to linked documentation (use @doc annotation)",
            );

            diagnostics.push(diag);
        }
        // Check if description exceeds warning threshold
        else if len > self.warning_threshold {
            let diag = LintDiagnostic::warning(
                "W_SEM_DESC_TOO_LONG",
                format!(
                    "Description is long ({len} characters, recommended max {max})",
                    len = len,
                    max = self.warning_threshold
                ),
                ctx.file_path.clone(),
                0,
                0,
            )
            .with_help(
                "Consider moving detailed content to linked documentation (use @doc annotation)",
            );

            diagnostics.push(diag);
        }

        diagnostics
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

    fn make_file_with_description(description: Option<&str>) -> TaskFile {
        TaskFile {
            path: PathBuf::from("test.md"),
            title: "Test File".to_string(),
            id: "test".to_string(),
            metadata: FileMetadata::default(),
            description: description.map(std::string::ToString::to_string),
            description_agent_notes: Vec::new(),
            tasks: TaskTree::new(),
            hash: "hash".to_string(),
            mtime: SystemTime::now(),
        }
    }

    #[test]
    fn test_no_description_passes() {
        let rule = DescriptionLengthRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let file = make_file_with_description(None);
        let diagnostics = rule.check_file(&file, &ctx);
        assert!(diagnostics.is_empty(), "No description should pass");
    }

    #[test]
    fn test_short_description_passes() {
        let rule = DescriptionLengthRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // 50 characters - well under warning threshold
        let short_desc = "This is a short description for the task file.";
        let file = make_file_with_description(Some(short_desc));
        let diagnostics = rule.check_file(&file, &ctx);
        assert!(
            diagnostics.is_empty(),
            "Short description should pass without warnings"
        );
    }

    #[test]
    fn test_exactly_warning_threshold_passes() {
        let rule = DescriptionLengthRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // Exactly 1000 characters (at threshold, should pass)
        let desc = "a".repeat(1000);
        let file = make_file_with_description(Some(&desc));
        let diagnostics = rule.check_file(&file, &ctx);
        assert!(
            diagnostics.is_empty(),
            "Description at exactly warning threshold should pass"
        );
    }

    #[test]
    fn test_over_warning_threshold_warns() {
        let rule = DescriptionLengthRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // 1001 characters (just over warning threshold)
        let desc = "a".repeat(1001);
        let file = make_file_with_description(Some(&desc));
        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "W_SEM_DESC_TOO_LONG");
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert!(diagnostics[0].message.contains("1001"));
        assert!(diagnostics[0].message.contains("1000"));
        assert!(diagnostics[0].help.is_some());
    }

    #[test]
    fn test_exactly_error_threshold_warns() {
        let rule = DescriptionLengthRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // Exactly 2000 characters (at error threshold, should still warn, not error)
        let desc = "a".repeat(2000);
        let file = make_file_with_description(Some(&desc));
        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "W_SEM_DESC_TOO_LONG");
        assert_eq!(diagnostics[0].severity, Severity::Warning);
    }

    #[test]
    fn test_over_error_threshold_errors() {
        let rule = DescriptionLengthRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // 2001 characters (over hard limit)
        let desc = "a".repeat(2001);
        let file = make_file_with_description(Some(&desc));
        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_SEM_DESC_TOO_LONG");
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert!(diagnostics[0].message.contains("2001"));
        assert!(diagnostics[0].message.contains("2000"));
        assert!(diagnostics[0].help.is_some());
    }

    #[test]
    fn test_custom_thresholds() {
        let rule = DescriptionLengthRule::with_thresholds(500, 1000);
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // 400 chars - should pass
        let desc = "a".repeat(400);
        let file = make_file_with_description(Some(&desc));
        let diagnostics = rule.check_file(&file, &ctx);
        assert!(diagnostics.is_empty());

        // 600 chars - should warn
        let desc = "a".repeat(600);
        let file = make_file_with_description(Some(&desc));
        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "W_SEM_DESC_TOO_LONG");
        assert!(diagnostics[0].message.contains("600"));
        assert!(diagnostics[0].message.contains("500"));

        // 1100 chars - should error
        let desc = "a".repeat(1100);
        let file = make_file_with_description(Some(&desc));
        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_SEM_DESC_TOO_LONG");
        assert!(diagnostics[0].message.contains("1100"));
        assert!(diagnostics[0].message.contains("1000"));
    }

    #[test]
    #[should_panic(expected = "Warning threshold must be less than error threshold")]
    fn test_invalid_thresholds_panics() {
        // Warning threshold >= error threshold should panic
        let _ = DescriptionLengthRule::with_thresholds(1000, 1000);
    }

    #[test]
    fn test_rule_metadata() {
        let rule = DescriptionLengthRule::new();
        assert_eq!(rule.code(), "W_SEM_DESC_TOO_LONG");
        assert_eq!(rule.severity(), Severity::Warning);
        assert_eq!(rule.name(), "Description length");
        assert!(!rule.description().is_empty());
    }

    #[test]
    fn test_default_trait() {
        let rule = DescriptionLengthRule::default();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // Should behave same as new()
        let desc = "a".repeat(1001);
        let file = make_file_with_description(Some(&desc));
        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "W_SEM_DESC_TOO_LONG");
    }
}
