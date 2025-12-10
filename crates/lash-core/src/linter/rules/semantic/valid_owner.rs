//! Owner format validation rule
//!
//! Ensures that owner names are non-empty and reasonably sized.
//! Very long owner names may indicate a data entry error.

use lash_types::{Severity, Task};

use crate::linter::{Fix, LintContext, LintDiagnostic, LintRule, Replacement};

/// Rule that validates owner format
///
/// Owner names should be:
/// - Non-empty
/// - Reasonable length (warning if > 100 characters)
///
/// **Code:** `W_SEM_OWNER_FORMAT`
/// **Severity:** Warning
///
/// # Auto-fix
///
/// The auto-fix trims excessively long owner names to a reasonable length
/// (100 characters with ellipsis).
///
/// # Examples
///
/// Valid owners:
/// ```markdown
/// @owner: John Doe
/// @owner: alice
/// @owner: team-backend
/// ```
///
/// Warning (very long):
/// ```markdown
/// @owner: This is an extremely long owner name that seems suspicious...
/// ```
pub struct ValidOwnerRule;

impl ValidOwnerRule {
    /// Create a new valid owner rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Maximum reasonable owner name length
    const MAX_OWNER_LENGTH: usize = 100;

    /// Check if owner name is valid
    fn is_valid_owner(owner: &str) -> bool {
        !owner.trim().is_empty()
    }

    /// Check if owner name is excessively long
    fn is_owner_too_long(owner: &str) -> bool {
        owner.len() > Self::MAX_OWNER_LENGTH
    }

    /// Trim owner name to reasonable length
    fn trim_owner(owner: &str) -> String {
        if owner.len() <= Self::MAX_OWNER_LENGTH {
            owner.to_string()
        } else {
            format!("{}...", &owner[..Self::MAX_OWNER_LENGTH - 3])
        }
    }
}

impl Default for ValidOwnerRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRule for ValidOwnerRule {
    fn code(&self) -> &'static str {
        "W_SEM_OWNER_FORMAT"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn name(&self) -> String {
        "Owner format".to_string()
    }

    fn description(&self) -> &'static str {
        "Ensures owner names are non-empty and reasonably sized"
    }

    fn check_task(&self, task: &Task, ctx: &LintContext) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(owner) = &task.metadata.owner {
            // Check for empty owner (after trimming)
            if !Self::is_valid_owner(owner) {
                diagnostics.push(
                    LintDiagnostic::error(
                        "E_SEM_EMPTY_OWNER",
                        "Owner name cannot be empty",
                        ctx.file_path.clone(),
                        0,
                        0,
                    )
                    .with_help("Provide a non-empty owner name or remove the @owner annotation"),
                );
            }
            // Check for excessively long owner
            else if Self::is_owner_too_long(owner) {
                let trimmed = Self::trim_owner(owner);

                let fix = Fix {
                    description: format!(
                        "Trim owner name to {} characters",
                        Self::MAX_OWNER_LENGTH
                    ),
                    replacement: Replacement::TextReplace {
                        old: format!("@owner: {owner}"),
                        new: format!("@owner: {trimmed}"),
                    },
                };

                diagnostics.push(
                    LintDiagnostic::warning(
                        self.code(),
                        format!("Owner name is unusually long ({} characters)", owner.len()),
                        ctx.file_path.clone(),
                        0,
                        0,
                    )
                    .with_help(format!(
                        "Owner names over {} characters may indicate a data entry error",
                        Self::MAX_OWNER_LENGTH
                    ))
                    .with_fix(fix),
                );
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::{LashConfig, Task, TaskMetadata, TaskStatus};
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

    fn make_task_with_owner(owner: Option<&str>) -> Task {
        Task {
            id: "task-1".to_string(),
            title: "Test task".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            metadata: TaskMetadata {
                owner: owner.map(std::string::ToString::to_string),
                ..Default::default()
            },
            body: None,
        }
    }

    #[test]
    fn test_valid_owners() {
        let rule = ValidOwnerRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let valid_owners = vec![
            "John Doe",
            "alice",
            "team-backend",
            "user@example.com",
            "A",
            "A reasonably long name that is still under the limit",
        ];

        for owner in valid_owners {
            let task = make_task_with_owner(Some(owner));
            let diagnostics = rule.check_task(&task, &ctx);
            assert!(diagnostics.is_empty(), "Owner '{owner}' should be valid");
        }
    }

    #[test]
    fn test_empty_owner() {
        let rule = ValidOwnerRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let task = make_task_with_owner(Some(""));
        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_SEM_EMPTY_OWNER");
        assert_eq!(diagnostics[0].severity, Severity::Error);
    }

    #[test]
    fn test_whitespace_only_owner() {
        let rule = ValidOwnerRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let task = make_task_with_owner(Some("   "));
        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_SEM_EMPTY_OWNER");
    }

    #[test]
    fn test_excessively_long_owner() {
        let rule = ValidOwnerRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let long_owner = "A".repeat(150);
        let task = make_task_with_owner(Some(&long_owner));
        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "W_SEM_OWNER_FORMAT");
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert!(diagnostics[0].message.contains("150 characters"));
        assert!(diagnostics[0].has_fix());
    }

    #[test]
    fn test_owner_at_limit() {
        let rule = ValidOwnerRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let owner_at_limit = "A".repeat(100);
        let task = make_task_with_owner(Some(&owner_at_limit));
        let diagnostics = rule.check_task(&task, &ctx);
        assert!(diagnostics.is_empty(), "Owner at limit should be valid");
    }

    #[test]
    fn test_owner_just_over_limit() {
        let rule = ValidOwnerRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let owner_over_limit = "A".repeat(101);
        let task = make_task_with_owner(Some(&owner_over_limit));
        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_no_owner() {
        let rule = ValidOwnerRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let task = make_task_with_owner(None);
        let diagnostics = rule.check_task(&task, &ctx);
        assert!(diagnostics.is_empty(), "No owner means no error");
    }

    #[test]
    fn test_trim_owner() {
        let long_owner = "A".repeat(150);
        let trimmed = ValidOwnerRule::trim_owner(&long_owner);
        assert_eq!(trimmed.len(), 100);
        assert!(trimmed.ends_with("..."));
    }

    #[test]
    fn test_is_valid_owner() {
        assert!(ValidOwnerRule::is_valid_owner("John"));
        assert!(ValidOwnerRule::is_valid_owner("A"));
        assert!(ValidOwnerRule::is_valid_owner(" John ")); // Trims to "John"

        assert!(!ValidOwnerRule::is_valid_owner(""));
        assert!(!ValidOwnerRule::is_valid_owner("   "));
    }

    #[test]
    fn test_is_owner_too_long() {
        assert!(!ValidOwnerRule::is_owner_too_long("John"));
        assert!(!ValidOwnerRule::is_owner_too_long(&"A".repeat(100)));
        assert!(ValidOwnerRule::is_owner_too_long(&"A".repeat(101)));
        assert!(ValidOwnerRule::is_owner_too_long(&"A".repeat(200)));
    }

    #[test]
    fn test_rule_metadata() {
        let rule = ValidOwnerRule::new();
        assert_eq!(rule.code(), "W_SEM_OWNER_FORMAT");
        assert_eq!(rule.severity(), Severity::Warning);
        assert_eq!(rule.name(), "Owner format");
        assert!(!rule.description().is_empty());
    }
}
