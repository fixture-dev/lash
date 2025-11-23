//! Rule: Valid dependency path resolution
//!
//! Validates that relative paths in dependencies resolve correctly:
//! - No paths escaping project root
//! - No malformed paths (double //, etc.)
//! - Path separators are normalized
//!
//! Error code: `E_LINK_INVALID_PATH`

use lash_types::{dependency::DependencyKind, Severity, Task};
use std::path::{Component, Path};

use crate::linter::{Fix, LintContext, LintDiagnostic, LintRule, Replacement};

/// Rule that validates dependency path resolution
///
/// This rule checks that dependency paths are:
/// 1. Well-formed (no malformed path components)
/// 2. Don't escape the project root (no excessive `..` navigation)
/// 3. Have normalized separators
///
/// # Examples
///
/// Valid:
/// ```markdown
/// @depends-on: ../core/api.md
/// @depends-on: tasks/ui/login.md
/// ```
///
/// Invalid (`E_LINK_INVALID_PATH`):
/// ```markdown
/// @depends-on: ../../../../../../etc/passwd  // Escapes project root
/// @depends-on: tasks//double-slash.md        // Malformed path
/// @depends-on: tasks/./redundant.md          // Unnecessary . components
/// ```
pub struct ValidPathResolutionRule;

impl ValidPathResolutionRule {
    /// Create a new valid path resolution rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Check if a path has malformed components
    fn has_malformed_components(path: &str) -> bool {
        // Check for double slashes
        if path.contains("//") {
            return true;
        }

        // Check for trailing slash (except for directory deps which should have it)
        if path.ends_with('/') && !path.ends_with(".md/") {
            return false; // Directory deps are ok
        }

        false
    }

    /// Check if a path string has unnecessary . components
    fn has_unnecessary_dots(path_str: &str) -> bool {
        // Check the string directly since Path normalizes automatically
        path_str.contains("/./") || path_str.starts_with("./")
    }

    /// Check if a path would escape the project root
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    fn escapes_project_root(path_str: &str, current_file: &Path) -> bool {
        // Count the depth of the current file
        let current_depth = if let Some(parent) = current_file.parent() {
            parent.components().count() as i32
        } else {
            0
        };

        // Count how many .. we have in the path
        let mut depth = current_depth;
        for part in path_str.split('/') {
            if part == ".." {
                depth -= 1;
                if depth < 0 {
                    return true; // Escaped project root
                }
            } else if !part.is_empty() && part != "." {
                depth += 1;
            }
        }

        false
    }

    /// Normalize a path by removing unnecessary components
    fn normalize_path(path: &str) -> String {
        let p = Path::new(path);
        let mut components = Vec::new();

        for component in p.components() {
            match component {
                Component::CurDir => {
                    // Skip current directory markers
                }
                Component::ParentDir => {
                    if components.last() != Some(&Component::ParentDir) && !components.is_empty() {
                        components.pop();
                    } else {
                        components.push(component);
                    }
                }
                comp => {
                    components.push(comp);
                }
            }
        }

        let normalized: std::path::PathBuf = components.iter().collect();
        // Always use forward slashes for cross-platform consistency in markdown files
        normalized.to_string_lossy().replace('\\', "/")
    }
}

impl Default for ValidPathResolutionRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRule for ValidPathResolutionRule {
    fn code(&self) -> &'static str {
        "E_LINK_INVALID_PATH"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn description(&self) -> &'static str {
        "Validates that dependency paths are well-formed and don't escape project root"
    }

    fn check_task(&self, task: &Task, ctx: &LintContext) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        for dep_ref in &task.metadata.depends_on {
            // Only check path-based dependencies
            if !matches!(dep_ref.kind, DependencyKind::ExplicitPath) {
                continue;
            }

            let path_str = &dep_ref.target;

            // Check for malformed components
            if Self::has_malformed_components(path_str) {
                diagnostics.push(
                    LintDiagnostic::error(
                        self.code(),
                        format!("Malformed path in dependency: '{path_str}'"),
                        ctx.file_path.clone(),
                        0,
                        0,
                    )
                    .with_help("Remove double slashes and other malformed components"),
                );
                continue;
            }

            // Check for unnecessary . components
            if Self::has_unnecessary_dots(path_str) {
                let normalized = Self::normalize_path(path_str);
                diagnostics.push(
                    LintDiagnostic::error(
                        self.code(),
                        format!("Path contains unnecessary '.' components: '{path_str}'"),
                        ctx.file_path.clone(),
                        0,
                        0,
                    )
                    .with_help(format!("Use normalized path: '{normalized}'"))
                    .with_fix(Fix {
                        description: format!("Normalize path to '{normalized}'"),
                        replacement: Replacement::TextReplace {
                            old: path_str.clone(),
                            new: normalized,
                        },
                    }),
                );
                continue;
            }

            // Check if path would escape project root
            if Self::escapes_project_root(path_str, &ctx.file_path) {
                let path = Path::new(path_str);
                let resolved = ctx.resolve_path(path);
                diagnostics.push(
                    LintDiagnostic::error(
                        self.code(),
                        format!(
                            "Path escapes project root: '{path_str}' (resolved to: {})",
                            resolved.display()
                        ),
                        ctx.file_path.clone(),
                        0,
                        0,
                    )
                    .with_help("Dependency paths must stay within the project root"),
                );
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::{
        dependency::{DependencyKind, DependencyRef},
        task::{Task, TaskMetadata},
        LashConfig, TaskFile, TaskStatus,
    };
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn make_task_with_dep(dep_target: &str, kind: DependencyKind) -> Task {
        let metadata = TaskMetadata {
            depends_on: vec![DependencyRef::new(dep_target.to_string(), kind)],
            ..Default::default()
        };

        Task {
            id: "test-task".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            metadata,
            body: None,
        }
    }

    fn make_context<'a>(
        config: &'a LashConfig,
        current_file: PathBuf,
        files: &'a HashMap<PathBuf, TaskFile>,
    ) -> LintContext<'a> {
        LintContext::new(config, current_file, files)
    }

    #[test]
    fn test_valid_relative_path() {
        let rule = ValidPathResolutionRule::new();
        let config = LashConfig::default();
        let files = HashMap::new();
        let ctx = make_context(&config, PathBuf::from("tasks/ui/login.md"), &files);
        let task = make_task_with_dep("../core/api.md", DependencyKind::ExplicitPath);

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_valid_forward_path() {
        let rule = ValidPathResolutionRule::new();
        let config = LashConfig::default();
        let files = HashMap::new();
        let ctx = make_context(&config, PathBuf::from("tasks.md"), &files);
        let task = make_task_with_dep("subtasks/details.md", DependencyKind::ExplicitPath);

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_malformed_double_slash() {
        let rule = ValidPathResolutionRule::new();
        let config = LashConfig::default();
        let files = HashMap::new();
        let ctx = make_context(&config, PathBuf::from("tasks.md"), &files);
        let task = make_task_with_dep("tasks//broken.md", DependencyKind::ExplicitPath);

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_LINK_INVALID_PATH");
        assert!(diagnostics[0].message.contains("Malformed"));
    }

    #[test]
    fn test_unnecessary_dot_components() {
        let rule = ValidPathResolutionRule::new();
        let config = LashConfig::default();
        let files = HashMap::new();
        let ctx = make_context(&config, PathBuf::from("tasks.md"), &files);
        let task = make_task_with_dep("./tasks/api.md", DependencyKind::ExplicitPath);

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_LINK_INVALID_PATH");
        assert!(diagnostics[0].message.contains("unnecessary"));
        // Should have auto-fix
        assert!(diagnostics[0].fix.is_some());
    }

    #[test]
    fn test_escapes_project_root() {
        let rule = ValidPathResolutionRule::new();
        let config = LashConfig::default();
        let files = HashMap::new();
        let ctx = make_context(&config, PathBuf::from("tasks.md"), &files);
        // Too many .. would escape the project root
        let task = make_task_with_dep("../../../etc/passwd", DependencyKind::ExplicitPath);

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_LINK_INVALID_PATH");
        assert!(diagnostics[0].message.contains("escapes project root"));
    }

    #[test]
    fn test_escapes_from_nested_file() {
        let rule = ValidPathResolutionRule::new();
        let config = LashConfig::default();
        let files = HashMap::new();
        let ctx = make_context(
            &config,
            PathBuf::from("tasks/ui/components/button.md"),
            &files,
        );
        // Going up 4 levels from button.md would escape
        let task = make_task_with_dep("../../../../outside.md", DependencyKind::ExplicitPath);

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_LINK_INVALID_PATH");
    }

    #[test]
    fn test_explicit_id_not_checked() {
        let rule = ValidPathResolutionRule::new();
        let config = LashConfig::default();
        let files = HashMap::new();
        let ctx = make_context(&config, PathBuf::from("tasks.md"), &files);
        // ExplicitId dependencies shouldn't be path-validated
        let task = make_task_with_dep("file#task", DependencyKind::ExplicitId);

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_directory_dependency_not_checked() {
        let rule = ValidPathResolutionRule::new();
        let config = LashConfig::default();
        let files = HashMap::new();
        let ctx = make_context(&config, PathBuf::from("tasks.md"), &files);
        let task = make_task_with_dep("core/", DependencyKind::Directory);

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_normalize_path_simple() {
        assert_eq!(
            ValidPathResolutionRule::normalize_path("./tasks/api.md"),
            "tasks/api.md"
        );
        assert_eq!(
            ValidPathResolutionRule::normalize_path("tasks/./ui/./login.md"),
            "tasks/ui/login.md"
        );
    }

    #[test]
    fn test_normalize_path_with_parent_dirs() {
        assert_eq!(
            ValidPathResolutionRule::normalize_path("tasks/ui/../core/api.md"),
            "tasks/core/api.md"
        );
    }

    #[test]
    fn test_has_malformed_components() {
        assert!(ValidPathResolutionRule::has_malformed_components(
            "tasks//api.md"
        ));
        assert!(ValidPathResolutionRule::has_malformed_components(
            "//tasks/api.md"
        ));
        assert!(!ValidPathResolutionRule::has_malformed_components(
            "tasks/api.md"
        ));
        assert!(!ValidPathResolutionRule::has_malformed_components(
            "../tasks/api.md"
        ));
    }

    #[test]
    fn test_has_unnecessary_dots() {
        assert!(ValidPathResolutionRule::has_unnecessary_dots(
            "./tasks/api.md"
        ));
        assert!(ValidPathResolutionRule::has_unnecessary_dots(
            "tasks/./api.md"
        ));
        assert!(!ValidPathResolutionRule::has_unnecessary_dots(
            "tasks/api.md"
        ));
        assert!(!ValidPathResolutionRule::has_unnecessary_dots(
            "../tasks/api.md"
        ));
    }

    #[test]
    fn test_multiple_issues_in_one_path() {
        let rule = ValidPathResolutionRule::new();
        let config = LashConfig::default();
        let files = HashMap::new();
        let ctx = make_context(&config, PathBuf::from("tasks.md"), &files);
        // Path with both unnecessary dots
        let task = make_task_with_dep("././tasks/api.md", DependencyKind::ExplicitPath);

        let diagnostics = rule.check_task(&task, &ctx);
        // Should report the first issue found
        assert!(!diagnostics.is_empty());
        assert_eq!(diagnostics[0].code, "E_LINK_INVALID_PATH");
    }

    #[test]
    fn test_complex_valid_path() {
        let rule = ValidPathResolutionRule::new();
        let config = LashConfig::default();
        let files = HashMap::new();
        let ctx = make_context(
            &config,
            PathBuf::from("tasks/ui/components/form/input.md"),
            &files,
        );
        // Navigate up and down in a valid way
        let task = make_task_with_dep("../../core/api.md", DependencyKind::ExplicitPath);

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_edge_case_root_level_file() {
        let rule = ValidPathResolutionRule::new();
        let config = LashConfig::default();
        let files = HashMap::new();
        let ctx = make_context(&config, PathBuf::from("tasks.md"), &files);
        // Single .. from root level file
        let task = make_task_with_dep("../outside.md", DependencyKind::ExplicitPath);

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("escapes"));
    }

    #[test]
    fn test_task_with_multiple_dependencies() {
        let rule = ValidPathResolutionRule::new();
        let config = LashConfig::default();
        let files = HashMap::new();
        let ctx = make_context(&config, PathBuf::from("tasks.md"), &files);

        let metadata = TaskMetadata {
            depends_on: vec![
                DependencyRef::new("valid/path.md".to_string(), DependencyKind::ExplicitPath),
                DependencyRef::new("./invalid.md".to_string(), DependencyKind::ExplicitPath),
                DependencyRef::new("tasks//broken.md".to_string(), DependencyKind::ExplicitPath),
            ],
            ..Default::default()
        };

        let task = Task {
            id: "test-task".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            metadata,
            body: None,
        };

        let diagnostics = rule.check_task(&task, &ctx);
        // Should report 2 issues (invalid and broken)
        assert_eq!(diagnostics.len(), 2);
    }

    #[test]
    fn test_auto_fix_provides_normalized_path() {
        let rule = ValidPathResolutionRule::new();
        let config = LashConfig::default();
        let files = HashMap::new();
        let ctx = make_context(&config, PathBuf::from("tasks.md"), &files);
        let task = make_task_with_dep("./path/./to/file.md", DependencyKind::ExplicitPath);

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);

        let fix = diagnostics[0].fix.as_ref().unwrap();
        if let Replacement::TextReplace { old, new } = &fix.replacement {
            assert_eq!(old, "./path/./to/file.md");
            assert_eq!(new, "path/to/file.md");
        } else {
            panic!("Expected text replacement");
        }
    }
}
