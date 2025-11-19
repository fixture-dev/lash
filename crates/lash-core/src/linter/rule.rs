//! Core trait and types for linting rules
//!
//! The `LintRule` trait defines the interface that all linting rules must implement.
//! Rules can validate at the file level or the task level, and can provide auto-fix
//! suggestions where appropriate.

use lash_types::{Severity, Task, TaskFile};

use crate::linter::{LintContext, LintDiagnostic};

/// Core trait that all linting rules must implement
///
/// Linting rules can operate at two levels:
/// - **File level**: Validate overall file structure, header format, etc.
/// - **Task level**: Validate individual tasks and their relationships
///
/// Rules should be independent and composable. Each rule focuses on a single
/// validation concern and returns a list of diagnostics for issues found.
///
/// # Example
///
/// ```rust,ignore
/// struct DepthLimitRule {
///     max_depth: u8,
/// }
///
/// impl LintRule for DepthLimitRule {
///     fn code(&self) -> &'static str {
///         "E_SYNTAX_DEPTH"
///     }
///
///     fn severity(&self) -> Severity {
///         Severity::Error
///     }
///
///     fn check_task(&self, task: &Task, ctx: &LintContext) -> Vec<LintDiagnostic> {
///         if task.depth > self.max_depth {
///             vec![LintDiagnostic::error(
///                 self.code(),
///                 format!("Task depth {} exceeds maximum {}", task.depth, self.max_depth),
///                 ctx.file_path.clone(),
///                 /* line */ 0,
///                 /* column */ 0,
///             )]
///         } else {
///             vec![]
///         }
///     }
/// }
/// ```
pub trait LintRule: Send + Sync {
    /// Stable rule code (e.g., `E_DEPTH_EXCEEDED`)
    ///
    /// Rule codes should follow the convention:
    /// - `E_SYNTAX_*` - Syntax/formatting errors
    /// - `E_SEM_*` - Semantic errors
    /// - `W_SYNTAX_*` - Syntax warnings
    /// - `W_SEM_*` - Semantic warnings
    /// - `I_SYNTAX_*` - Syntax info/hints
    /// - `I_SEM_*` - Semantic info/hints
    fn code(&self) -> &'static str;

    /// Default severity for this rule
    ///
    /// This can be overridden in configuration via `severity_overrides`.
    fn severity(&self) -> Severity;

    /// Check the entire file for issues
    ///
    /// This method is called once per file and should validate file-level
    /// concerns like:
    /// - Header structure
    /// - File-level annotation validity
    /// - Overall task structure
    ///
    /// Default implementation returns no diagnostics (file-level validation optional).
    #[allow(unused_variables)]
    fn check_file(&self, file: &TaskFile, ctx: &LintContext) -> Vec<LintDiagnostic> {
        Vec::new()
    }

    /// Check an individual task for issues
    ///
    /// This method is called once per task in the file and should validate
    /// task-level concerns like:
    /// - Task depth
    /// - Annotation validity
    /// - Label format
    /// - Status consistency
    ///
    /// Default implementation returns no diagnostics (task-level validation optional).
    #[allow(unused_variables)]
    fn check_task(&self, task: &Task, ctx: &LintContext) -> Vec<LintDiagnostic> {
        Vec::new()
    }

    /// Human-readable name for this rule
    ///
    /// Used in help text and documentation. Default implementation converts
    /// the rule code to a human-readable name.
    fn name(&self) -> String {
        self.code()
            .to_lowercase()
            .replace('_', " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Detailed description of what this rule checks
    ///
    /// Used in help text and documentation. Rules should override this to
    /// provide helpful context.
    fn description(&self) -> &'static str {
        "No description available"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::Severity;

    // Test rule implementation
    struct TestRule;

    impl LintRule for TestRule {
        fn code(&self) -> &'static str {
            "E_TEST_RULE"
        }

        fn severity(&self) -> Severity {
            Severity::Error
        }
    }

    #[test]
    fn test_rule_code() {
        let rule = TestRule;
        assert_eq!(rule.code(), "E_TEST_RULE");
    }

    #[test]
    fn test_rule_severity() {
        let rule = TestRule;
        assert_eq!(rule.severity(), Severity::Error);
    }

    #[test]
    fn test_default_name() {
        let rule = TestRule;
        assert_eq!(rule.name(), "e test rule");
    }

    #[test]
    fn test_default_description() {
        let rule = TestRule;
        assert_eq!(rule.description(), "No description available");
    }
}
