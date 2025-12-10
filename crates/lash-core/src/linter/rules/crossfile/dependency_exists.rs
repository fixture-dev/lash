//! Rule: Dependency references must exist
//!
//! Validates that all `@depends-on` targets exist:
//! - File references: Check file exists in project
//! - Task references: Check file exists AND contains task ID
//!
//! Error code: `E_LINK_NOT_FOUND`

use lash_types::{dependency::DependencyKind, Severity, Task, TaskFile};
use std::path::Path;

use crate::linter::{LintContext, LintDiagnostic, LintRule};

/// Rule that checks dependency references exist
///
/// This rule validates that all `@depends-on` annotations reference valid targets:
/// - For file references (`path/to/file.md`): File must exist in the project
/// - For task references (`path/to/file.md#task:id`): File must exist AND contain the task ID
///
/// # Examples
///
/// Valid:
/// ```markdown
/// @depends-on: existing-file.md
/// @depends-on: existing-file.md#task:existing-task
/// ```
///
/// Invalid (`E_LINK_NOT_FOUND`):
/// ```markdown
/// @depends-on: missing-file.md
/// @depends-on: existing-file.md#task:missing-task
/// ```
pub struct DependencyExistsRule;

impl DependencyExistsRule {
    /// Create a new dependency exists rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Check if a file exists in the context
    fn file_exists(ctx: &LintContext, file_path: &Path) -> bool {
        ctx.get_file(file_path).is_some()
    }

    /// Check if a task exists in a file
    fn task_exists_in_file(file: &TaskFile, task_id: &str) -> bool {
        file.tasks.tasks().iter().any(|t| t.id == task_id)
    }
}

impl Default for DependencyExistsRule {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(clippy::too_many_lines)]
impl LintRule for DependencyExistsRule {
    fn code(&self) -> &'static str {
        "E_LINK_NOT_FOUND"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn description(&self) -> &'static str {
        "Validates that all dependency references point to existing files and tasks"
    }

    fn check_task(&self, task: &Task, ctx: &LintContext) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        // Check all dependencies in this task's metadata
        for dep_ref in &task.metadata.depends_on {
            match dep_ref.kind {
                DependencyKind::ExplicitPath => {
                    // Check if this is a task reference with path format (file.md#task:id)
                    if let Some((path_part, task_part)) = dep_ref.target.split_once("#task:") {
                        // This is a task reference
                        let target_path = ctx.resolve_path(Path::new(path_part));

                        if !Self::file_exists(ctx, &target_path) {
                            diagnostics.push(
                                LintDiagnostic::error(
                                    self.code(),
                                    format!(
                                        "File '{path_part}' not found (resolved to: {})",
                                        target_path.display()
                                    ),
                                    ctx.file_path.clone(),
                                    0,
                                    0,
                                )
                                .with_help(format!(
                                    "Check that the file exists. Expected file at: {}",
                                    target_path.display()
                                )),
                            );
                        } else if let Some(target_file) = ctx.get_file(&target_path) {
                            if !Self::task_exists_in_file(target_file, task_part) {
                                diagnostics.push(
                                    LintDiagnostic::error(
                                        self.code(),
                                        format!(
                                            "Task '{task_part}' not found in file '{}'",
                                            target_path.display()
                                        ),
                                        ctx.file_path.clone(),
                                        0,
                                        0,
                                    )
                                    .with_help(format!(
                                        "Check that the task ID exists in {}. Available tasks: {}",
                                        target_path.display(),
                                        target_file
                                            .tasks
                                            .tasks()
                                            .iter()
                                            .map(|t| t.id.as_str())
                                            .take(5)
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    )),
                                );
                            }
                        }
                    } else {
                        // This is just a file reference
                        let target_path = ctx.resolve_path(Path::new(&dep_ref.target));

                        if !Self::file_exists(ctx, &target_path) {
                            diagnostics.push(LintDiagnostic::error(
                                self.code(),
                                format!(
                                    "Dependency reference to file '{}' not found (resolved to: {})",
                                    dep_ref.target,
                                    target_path.display()
                                ),
                                ctx.file_path.clone(),
                                0,
                                0,
                            ).with_help(
                                format!(
                                    "Check that the file exists, or fix the path. Expected file at: {}",
                                    target_path.display()
                                )
                            ));
                        }
                    }
                }
                DependencyKind::ExplicitId => {
                    // For ID references, check if it contains a task reference
                    if let Some((file_id, task_id)) = dep_ref.target.split_once('#') {
                        // Find file by ID or path
                        // First try to find by file ID (match file's id field)
                        let target_file = ctx.all_files.values().find(|f| f.id == file_id);

                        if let Some(target_file) = target_file {
                            // File found, check if task exists
                            if !Self::task_exists_in_file(target_file, task_id) {
                                diagnostics.push(LintDiagnostic::error(
                                    self.code(),
                                    format!(
                                        "Task '{task_id}' not found in file with ID '{file_id}'"
                                    ),
                                    ctx.file_path.clone(),
                                    0,
                                    0,
                                ).with_help(
                                    format!(
                                        "Check that the task ID exists in file '{}'. Available tasks: {}",
                                        target_file.path.display(),
                                        target_file.tasks.tasks().iter()
                                            .map(|t| t.id.as_str())
                                            .take(5)
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    )
                                ));
                            }
                        } else {
                            // Try as a path reference
                            let as_path = Path::new(file_id);
                            if let Some(target_file) = ctx.get_file(as_path) {
                                if !Self::task_exists_in_file(target_file, task_id) {
                                    diagnostics.push(LintDiagnostic::error(
                                        self.code(),
                                        format!(
                                            "Task '{task_id}' not found in file '{}'",
                                            as_path.display()
                                        ),
                                        ctx.file_path.clone(),
                                        0,
                                        0,
                                    ).with_help(
                                        format!(
                                            "Check that the task ID exists in {}. Available tasks: {}",
                                            as_path.display(),
                                            target_file.tasks.tasks().iter()
                                                .map(|t| t.id.as_str())
                                                .take(5)
                                                .collect::<Vec<_>>()
                                                .join(", ")
                                        )
                                    ));
                                }
                            } else {
                                // Neither file ID nor path found
                                diagnostics.push(
                                    LintDiagnostic::error(
                                        self.code(),
                                        format!("File with ID '{file_id}' not found in project"),
                                        ctx.file_path.clone(),
                                        0,
                                        0,
                                    )
                                    .with_help(
                                        "Check the file ID or path in the dependency reference",
                                    ),
                                );
                            }
                        }
                    } else {
                        // Bare file ID reference (no task specified)
                        let target_file = ctx.all_files.values().find(|f| f.id == dep_ref.target);

                        if target_file.is_none() {
                            // Try as a path
                            let as_path = Path::new(&dep_ref.target);
                            if !Self::file_exists(ctx, as_path) {
                                diagnostics.push(
                                    LintDiagnostic::error(
                                        self.code(),
                                        format!(
                                            "File with ID or path '{}' not found in project",
                                            dep_ref.target
                                        ),
                                        ctx.file_path.clone(),
                                        0,
                                        0,
                                    )
                                    .with_help(
                                        "Check the file ID or path in the dependency reference",
                                    ),
                                );
                            }
                        }
                    }
                }
                DependencyKind::Hierarchy | DependencyKind::Directory => {
                    // Hierarchy dependencies are implicit and always valid
                    // Directory dependencies are not currently validated for existence
                    // (validated during parsing / could be added in future if needed)
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::{
        dependency::{parse_dependency_ref, DependencyKind},
        task::{Task, TaskMetadata, TaskTree},
        FileMetadata, LashConfig, TaskStatus,
    };
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::SystemTime;

    fn make_task_with_dependency(dep_target: &str) -> Task {
        let dep_ref = parse_dependency_ref(dep_target).unwrap();
        let metadata = TaskMetadata {
            depends_on: vec![dep_ref],
            ..Default::default()
        };

        Task {
            id: "test-task".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            metadata,
            body: None,
        }
    }

    fn make_test_file(path: &str, id: &str, task_ids: &[&str]) -> TaskFile {
        let mut tasks = TaskTree::new();
        for (i, task_id) in task_ids.iter().enumerate() {
            let _ = tasks.add_task(Task {
                id: (*task_id).to_string(),
                title: format!("Task {task_id}"),
                status: TaskStatus::Open,
                depth: 0,
                parent_id: None,
                order_index: i,
                line_number: 0,
                metadata: TaskMetadata::default(),
                body: None,
            });
        }

        TaskFile {
            path: PathBuf::from(path),
            title: "Test File".to_string(),
            id: id.to_string(),
            metadata: FileMetadata::default(),
            description: None,
            description_agent_notes: Vec::new(),
            tasks,
            hash: "test-hash".to_string(),
            mtime: SystemTime::now(),
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
    fn test_valid_file_reference() {
        let rule = DependencyExistsRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();
        files.insert(
            PathBuf::from("other.md"),
            make_test_file("other.md", "other", &[]),
        );

        let ctx = make_context(&config, PathBuf::from("current.md"), &files);
        let task = make_task_with_dependency("other.md");

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_missing_file_reference() {
        let rule = DependencyExistsRule::new();
        let config = LashConfig::default();
        let files = HashMap::new();

        let ctx = make_context(&config, PathBuf::from("current.md"), &files);
        let task = make_task_with_dependency("missing.md");

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_LINK_NOT_FOUND");
        assert!(diagnostics[0].message.contains("missing.md"));
    }

    #[test]
    fn test_valid_task_reference_with_path() {
        let rule = DependencyExistsRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();
        files.insert(
            PathBuf::from("other.md"),
            make_test_file("other.md", "other", &["setup", "cleanup"]),
        );

        let ctx = make_context(&config, PathBuf::from("current.md"), &files);

        // Create a task with path-style task reference
        let metadata = TaskMetadata {
            depends_on: vec![lash_types::dependency::DependencyRef::new(
                "other.md#task:setup".to_string(),
                DependencyKind::ExplicitPath,
            )],
            ..Default::default()
        };

        let task = Task {
            id: "test-task".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            metadata,
            body: None,
        };

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_missing_task_reference_with_path() {
        let rule = DependencyExistsRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();
        files.insert(
            PathBuf::from("other.md"),
            make_test_file("other.md", "other", &["setup"]),
        );

        let ctx = make_context(&config, PathBuf::from("current.md"), &files);

        let metadata = TaskMetadata {
            depends_on: vec![lash_types::dependency::DependencyRef::new(
                "other.md#task:missing".to_string(),
                DependencyKind::ExplicitPath,
            )],
            ..Default::default()
        };

        let task = Task {
            id: "test-task".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            metadata,
            body: None,
        };

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_LINK_NOT_FOUND");
        assert!(diagnostics[0].message.contains("missing"));
    }

    #[test]
    fn test_valid_task_reference_with_id() {
        let rule = DependencyExistsRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();
        files.insert(
            PathBuf::from("other.md"),
            make_test_file("other.md", "other", &["setup"]),
        );

        let ctx = make_context(&config, PathBuf::from("current.md"), &files);

        let metadata = TaskMetadata {
            depends_on: vec![lash_types::dependency::DependencyRef::new(
                "other#setup".to_string(),
                DependencyKind::ExplicitId,
            )],
            ..Default::default()
        };

        let task = Task {
            id: "test-task".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            metadata,
            body: None,
        };

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_missing_task_with_valid_file_id() {
        let rule = DependencyExistsRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();
        files.insert(
            PathBuf::from("other.md"),
            make_test_file("other.md", "other", &["setup"]),
        );

        let ctx = make_context(&config, PathBuf::from("current.md"), &files);

        let metadata = TaskMetadata {
            depends_on: vec![lash_types::dependency::DependencyRef::new(
                "other#missing".to_string(),
                DependencyKind::ExplicitId,
            )],
            ..Default::default()
        };

        let task = Task {
            id: "test-task".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            metadata,
            body: None,
        };

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_LINK_NOT_FOUND");
        assert!(diagnostics[0].message.contains("missing"));
    }

    #[test]
    fn test_missing_file_id() {
        let rule = DependencyExistsRule::new();
        let config = LashConfig::default();
        let files = HashMap::new();

        let ctx = make_context(&config, PathBuf::from("current.md"), &files);

        let metadata = TaskMetadata {
            depends_on: vec![lash_types::dependency::DependencyRef::new(
                "missing#task".to_string(),
                DependencyKind::ExplicitId,
            )],
            ..Default::default()
        };

        let task = Task {
            id: "test-task".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            metadata,
            body: None,
        };

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_LINK_NOT_FOUND");
    }

    #[test]
    fn test_hierarchy_dependency_always_valid() {
        let rule = DependencyExistsRule::new();
        let config = LashConfig::default();
        let files = HashMap::new();

        let ctx = make_context(&config, PathBuf::from("current.md"), &files);

        let metadata = TaskMetadata {
            depends_on: vec![lash_types::dependency::DependencyRef::new(
                String::new(),
                DependencyKind::Hierarchy,
            )],
            ..Default::default()
        };

        let task = Task {
            id: "test-task".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            metadata,
            body: None,
        };

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_multiple_dependencies_mixed_validity() {
        let rule = DependencyExistsRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();
        files.insert(
            PathBuf::from("exists.md"),
            make_test_file("exists.md", "exists", &["task1"]),
        );

        let ctx = make_context(&config, PathBuf::from("current.md"), &files);

        let metadata = TaskMetadata {
            depends_on: vec![
                lash_types::dependency::DependencyRef::new(
                    "exists.md".to_string(),
                    DependencyKind::ExplicitPath,
                ),
                lash_types::dependency::DependencyRef::new(
                    "missing.md".to_string(),
                    DependencyKind::ExplicitPath,
                ),
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
            line_number: 0,
            metadata,
            body: None,
        };

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("missing.md"));
    }

    #[test]
    fn test_relative_path_resolution() {
        let rule = DependencyExistsRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();
        files.insert(
            PathBuf::from("core/api.md"),
            make_test_file("core/api.md", "core.api", &[]),
        );

        let ctx = make_context(&config, PathBuf::from("tasks/ui/login.md"), &files);
        let task = make_task_with_dependency("../../core/api.md");

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_bare_file_id_reference() {
        let rule = DependencyExistsRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();
        files.insert(
            PathBuf::from("other.md"),
            make_test_file("other.md", "other-file", &[]),
        );

        let ctx = make_context(&config, PathBuf::from("current.md"), &files);

        let metadata = TaskMetadata {
            depends_on: vec![lash_types::dependency::DependencyRef::new(
                "other-file".to_string(),
                DependencyKind::ExplicitId,
            )],
            ..Default::default()
        };

        let task = Task {
            id: "test-task".to_string(),
            title: "Test Task".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            metadata,
            body: None,
        };

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 0);
    }
}
