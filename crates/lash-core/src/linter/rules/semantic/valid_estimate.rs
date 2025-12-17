//! Estimate format validation rule
//!
//! Ensures that time estimates follow the required pattern: a number followed
//! by a time unit (h, d, w, m, y).

use lash_types::{Severity, Task};

use crate::linter::{LintContext, LintDiagnostic, LintRule};

/// Rule that validates estimate format
///
/// Estimates must follow the pattern: `\d+[hdwmy]`
/// - One or more digits
/// - Followed by a single time unit character:
///   - `h` = hours
///   - `d` = days
///   - `w` = weeks
///   - `m` = months
///   - `y` = years
///
/// **Code:** `E_SEM_INVALID_ESTIMATE`
/// **Severity:** Error
///
/// # Auto-fix
///
/// No auto-fix is provided because the conversion from one format to another
/// is ambiguous (e.g., "2 hours" could be "2h" or the user might have meant
/// something else).
///
/// # Examples
///
/// Valid estimates:
/// ```markdown
/// @estimate: 2h
/// @estimate: 3d
/// @estimate: 1w
/// @estimate: 6m
/// ```
///
/// Invalid estimates:
/// ```markdown
/// @estimate: 2 hours ← spaces and full words not allowed
/// @estimate: 3days ← full words not allowed
/// @estimate: 1.5h ← decimals not allowed
/// @estimate: 2H ← uppercase not allowed
/// ```
pub struct ValidEstimateRule;

impl ValidEstimateRule {
    /// Create a new valid estimate rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Check if an estimate string is valid
    fn is_valid_estimate(estimate: &str) -> bool {
        if estimate.is_empty() {
            return false;
        }

        // Must end with a valid unit
        let last_char = estimate.chars().last().unwrap();
        if !matches!(last_char, 'h' | 'd' | 'w' | 'm' | 'y') {
            return false;
        }

        // All characters before the unit must be digits
        let digits = &estimate[..estimate.len() - 1];
        if digits.is_empty() {
            return false;
        }

        digits.chars().all(|c| c.is_ascii_digit())
    }
}

impl Default for ValidEstimateRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRule for ValidEstimateRule {
    fn code(&self) -> &'static str {
        "E_SEM_INVALID_ESTIMATE"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> String {
        "Estimate format".to_string()
    }

    fn description(&self) -> &'static str {
        "Ensures time estimates match pattern: number + unit (h/d/w/m/y)"
    }

    fn check_task(&self, task: &Task, ctx: &LintContext) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(estimate) = &task.metadata.estimate {
            if !Self::is_valid_estimate(estimate) {
                diagnostics.push(
                    LintDiagnostic::error(
                        self.code(),
                        format!("Invalid estimate format: '{estimate}'"),
                        ctx.file_path.clone(),
                        0,
                        0,
                    )
                    .with_help(
                        "Use format like: 2h, 3d, 1w, 6m, 1y (number + unit)\n\
                        Valid units: h=hours, d=days, w=weeks, m=months, y=years",
                    ),
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

    fn make_task_with_estimate(estimate: Option<&str>) -> Task {
        Task {
            id: "task-1".to_string(),
            has_explicit_id: false,
            title: "Test task".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            metadata: TaskMetadata {
                estimate: estimate.map(std::string::ToString::to_string),
                ..Default::default()
            },
            body: None,
            contextual_notes: Vec::new(),
        }
    }

    #[test]
    fn test_valid_estimates() {
        let rule = ValidEstimateRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let valid_estimates = vec![
            "1h", "2h", "24h", "1d", "5d", "30d", "1w", "2w", "52w", "1m", "6m", "12m", "1y", "2y",
            "10y", "100d", // Large numbers ok
        ];

        for estimate in valid_estimates {
            let task = make_task_with_estimate(Some(estimate));
            let diagnostics = rule.check_task(&task, &ctx);
            assert!(
                diagnostics.is_empty(),
                "Estimate '{estimate}' should be valid"
            );
        }
    }

    #[test]
    fn test_invalid_with_spaces() {
        let rule = ValidEstimateRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let task = make_task_with_estimate(Some("2 hours"));
        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_SEM_INVALID_ESTIMATE");
        assert!(diagnostics[0].message.contains("2 hours"));
    }

    #[test]
    fn test_invalid_full_words() {
        let rule = ValidEstimateRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let invalid = vec!["2hours", "3days", "1week", "6months", "1year"];

        for estimate in invalid {
            let task = make_task_with_estimate(Some(estimate));
            let diagnostics = rule.check_task(&task, &ctx);
            assert!(
                !diagnostics.is_empty(),
                "Estimate '{estimate}' should be invalid"
            );
        }
    }

    #[test]
    fn test_invalid_uppercase() {
        let rule = ValidEstimateRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let task = make_task_with_estimate(Some("2H"));
        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_invalid_decimals() {
        let rule = ValidEstimateRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let task = make_task_with_estimate(Some("1.5h"));
        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_invalid_no_number() {
        let rule = ValidEstimateRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let task = make_task_with_estimate(Some("h"));
        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_invalid_no_unit() {
        let rule = ValidEstimateRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let task = make_task_with_estimate(Some("2"));
        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_invalid_wrong_unit() {
        let rule = ValidEstimateRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let invalid = vec!["2s", "3min", "1hr", "2x", "3z"];

        for estimate in invalid {
            let task = make_task_with_estimate(Some(estimate));
            let diagnostics = rule.check_task(&task, &ctx);
            assert!(
                !diagnostics.is_empty(),
                "Estimate '{estimate}' should be invalid"
            );
        }
    }

    #[test]
    fn test_no_estimate() {
        let rule = ValidEstimateRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let task = make_task_with_estimate(None);
        let diagnostics = rule.check_task(&task, &ctx);
        assert!(diagnostics.is_empty(), "No estimate means no error");
    }

    #[test]
    fn test_is_valid_estimate() {
        assert!(ValidEstimateRule::is_valid_estimate("2h"));
        assert!(ValidEstimateRule::is_valid_estimate("3d"));
        assert!(ValidEstimateRule::is_valid_estimate("1w"));
        assert!(ValidEstimateRule::is_valid_estimate("100d"));

        assert!(!ValidEstimateRule::is_valid_estimate("2 hours"));
        assert!(!ValidEstimateRule::is_valid_estimate("2H"));
        assert!(!ValidEstimateRule::is_valid_estimate("1.5h"));
        assert!(!ValidEstimateRule::is_valid_estimate("h"));
        assert!(!ValidEstimateRule::is_valid_estimate("2"));
        assert!(!ValidEstimateRule::is_valid_estimate(""));
    }

    #[test]
    fn test_rule_metadata() {
        let rule = ValidEstimateRule::new();
        assert_eq!(rule.code(), "E_SEM_INVALID_ESTIMATE");
        assert_eq!(rule.severity(), Severity::Error);
        assert_eq!(rule.name(), "Estimate format");
        assert!(!rule.description().is_empty());
    }
}
