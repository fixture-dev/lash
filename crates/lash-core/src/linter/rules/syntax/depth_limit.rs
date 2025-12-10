//! Depth limit validation rule
//!
//! Enforces maximum task nesting depth to keep hierarchies manageable.
//! Deep nesting makes tasks harder to understand and should be split
//! into separate files or flattened.

use lash_types::{Severity, Task};

use crate::linter::{LintContext, LintDiagnostic, LintRule};

/// Rule that enforces maximum task depth
///
/// This rule checks that tasks don't exceed the configured maximum depth.
/// Deep hierarchies are harder to understand and should be split into
/// separate files or reorganized.
///
/// **Code:** `E_SYNTAX_DEPTH`
/// **Severity:** Error
///
/// # Configuration
///
/// The maximum depth is configured via `LashConfig::max_depth` (default: 2,
/// meaning 3 levels: 0, 1, 2).
///
/// # Examples
///
/// Valid (depth ≤ 2):
/// ```markdown
/// - [ ] Top level (depth 0)
///   - [ ] Second level (depth 1)
///     - [ ] Third level (depth 2)
/// ```
///
/// Invalid (depth 3):
/// ```markdown
/// - [ ] Top level (depth 0)
///   - [ ] Second level (depth 1)
///     - [ ] Third level (depth 2)
///       - [ ] Fourth level (depth 3) ← ERROR
/// ```
pub struct DepthLimitRule;

impl DepthLimitRule {
    /// Create a new depth limit rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for DepthLimitRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRule for DepthLimitRule {
    fn code(&self) -> &'static str {
        "E_SYNTAX_DEPTH"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> String {
        "Depth limit".to_string()
    }

    fn description(&self) -> &'static str {
        "Enforces maximum task nesting depth to keep hierarchies manageable"
    }

    fn check_task(&self, task: &Task, ctx: &LintContext) -> Vec<LintDiagnostic> {
        let max_depth = ctx.max_depth();

        if task.depth > max_depth {
            vec![LintDiagnostic::error(
                self.code(),
                format!(
                    "Task depth {} exceeds maximum depth {}",
                    task.depth, max_depth
                ),
                ctx.file_path.clone(),
                0, // Line number not available in Task struct
                0,
            )
            .with_help("Split deep hierarchies into separate files or flatten the structure")]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::{LashConfig, Task, TaskMetadata, TaskStatus};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn make_config(max_depth: u8) -> LashConfig {
        LashConfig {
            root_path: PathBuf::from("/test"),
            index_file: "index.md".to_string(),
            max_depth,
            indent_spaces: 2,
            db_path: PathBuf::from(".lash/test.db"),
            custom_annotation_keys: vec![],
        }
    }

    fn make_task(depth: u8) -> Task {
        Task {
            id: format!("task-{depth}"),
            title: format!("Task at depth {depth}"),
            status: TaskStatus::Open,
            depth,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            metadata: TaskMetadata::default(),
            body: None,
        }
    }

    #[test]
    fn test_valid_depth() {
        let rule = DepthLimitRule::new();
        let config = make_config(2);
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // Depth 0, 1, 2 should all be valid
        for depth in 0..=2 {
            let task = make_task(depth);
            let diagnostics = rule.check_task(&task, &ctx);
            assert!(
                diagnostics.is_empty(),
                "Depth {depth} should be valid (max_depth=2)"
            );
        }
    }

    #[test]
    fn test_depth_at_limit() {
        let rule = DepthLimitRule::new();
        let config = make_config(2);
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let task = make_task(2);
        let diagnostics = rule.check_task(&task, &ctx);
        assert!(diagnostics.is_empty(), "Depth at limit should be valid");
    }

    #[test]
    fn test_depth_exceeds_limit() {
        let rule = DepthLimitRule::new();
        let config = make_config(2);
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let task = make_task(3);
        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_SYNTAX_DEPTH");
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert!(diagnostics[0].message.contains("exceeds maximum depth"));
        assert!(diagnostics[0].help.is_some());
    }

    #[test]
    fn test_depth_far_exceeds_limit() {
        let rule = DepthLimitRule::new();
        let config = make_config(2);
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let task = make_task(10);
        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("10"));
    }

    #[test]
    fn test_different_max_depths() {
        let rule = DepthLimitRule::new();

        // Test max_depth = 0 (only top-level tasks)
        let config = make_config(0);
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);
        let task = make_task(1);
        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);

        // Test max_depth = 3
        let config = make_config(3);
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);
        let task = make_task(3);
        let diagnostics = rule.check_task(&task, &ctx);
        assert!(diagnostics.is_empty());

        let task = make_task(4);
        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_rule_metadata() {
        let rule = DepthLimitRule::new();
        assert_eq!(rule.code(), "E_SYNTAX_DEPTH");
        assert_eq!(rule.severity(), Severity::Error);
        assert_eq!(rule.name(), "Depth limit");
        assert!(!rule.description().is_empty());
    }
}
