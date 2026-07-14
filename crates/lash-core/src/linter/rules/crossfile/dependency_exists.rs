//! Rule: Dependency references must exist
//!
//! Validates that all `@depends-on` targets exist:
//! - File references: Check file exists in project
//! - Task references: Check file exists AND contains task ID
//!
//! Error code: `E_LINK_NOT_FOUND`

use lash_types::{dependency::DependencyKind, Severity, Task};
use std::path::Path;

use crate::dependency::reference::{resolve_reference, RefError};
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

    /// Best-effort lookup of the source line that authored a `@depends-on`
    /// reference. `DependencyRef` doesn't carry its origin line, so we reopen
    /// the source markdown and scan for a `@depends-on:` annotation whose
    /// (comma-split) values include `target`. Returns 1-indexed `(line, col)`
    /// when the source is readable; falls back to `(0, 0)` otherwise.
    ///
    /// Mirrors `BrokenDocFragmentRule::locate_doc_annotation` so that
    /// `E_LINK_NOT_FOUND` reports the offending line like `W_SEM_DOC_FRAGMENT`
    /// does (GitHub issue #18).
    fn locate_depends_on(ctx: &LintContext, target: &str) -> (usize, usize) {
        let absolute_source = ctx.config.root_path.join(&ctx.file_path);
        let Ok(content) = std::fs::read_to_string(&absolute_source) else {
            return (0, 0);
        };

        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim_start();
            let Some(value) = trimmed.strip_prefix("@depends-on:") else {
                continue;
            };
            let has_target = value.split(',').map(str::trim).any(|part| part == target);
            if has_target {
                let col = line.find('@').unwrap_or(0).saturating_add(1);
                return (idx.saturating_add(1), col);
            }
        }

        (0, 0)
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

        // Check all dependencies in this task's metadata. All reference forms
        // are resolved through the shared resolver so that `lint`,
        // `check-links`, and the CLI agree (GitHub issues #15, #19).
        for dep_ref in &task.metadata.depends_on {
            // Hierarchy deps are implicit; directory deps aren't validated here.
            if matches!(
                dep_ref.kind,
                DependencyKind::Hierarchy | DependencyKind::Directory
            ) {
                continue;
            }

            let resolve_path = |rel: &str| ctx.resolve_path(Path::new(rel));
            let result = resolve_reference(
                &dep_ref.target,
                &ctx.file_path,
                "",
                ctx.all_files,
                resolve_path,
            );

            if let Err(err) = result {
                let (line, column) = Self::locate_depends_on(ctx, &dep_ref.target);
                diagnostics.push(Self::diagnostic_for_error(
                    self.code(),
                    ctx,
                    &dep_ref.target,
                    &err,
                    line,
                    column,
                ));
            }
        }

        diagnostics
    }
}

impl DependencyExistsRule {
    /// Build an `E_LINK_NOT_FOUND` diagnostic from a resolution failure.
    fn diagnostic_for_error(
        code: &'static str,
        ctx: &LintContext,
        target: &str,
        err: &RefError,
        line: usize,
        column: usize,
    ) -> LintDiagnostic {
        match err {
            RefError::FileNotFound { reference } => LintDiagnostic::error(
                code,
                format!("Dependency reference '{reference}' not found in project"),
                ctx.file_path.clone(),
                line,
                column,
            )
            .with_help(format!(
                "'{target}' does not match any file id, file path, or task @id in the project"
            )),
            RefError::TaskNotFound {
                file_label,
                task,
                available,
            } => LintDiagnostic::error(
                code,
                format!("Task '{task}' not found in file '{file_label}'"),
                ctx.file_path.clone(),
                line,
                column,
            )
            .with_help(format!(
                "Available tasks in '{file_label}': {}",
                available.join(", ")
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::{
        dependency::{parse_dependency_ref, DependencyKind},
        task::{Task, TaskMetadata, TaskTree},
        FileMetadata, LashConfig, TaskFile, TaskStatus,
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
            has_explicit_id: false,
            title: "Test Task".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            metadata,
            body: None,
            contextual_notes: Vec::new(),
        }
    }

    fn make_test_file(path: &str, id: &str, task_ids: &[&str]) -> TaskFile {
        let mut tasks = TaskTree::new();
        for (i, task_id) in task_ids.iter().enumerate() {
            let _ = tasks.add_task(Task {
                id: (*task_id).to_string(),
                has_explicit_id: false,
                title: format!("Task {task_id}"),
                status: TaskStatus::Open,
                depth: 0,
                parent_id: None,
                order_index: i,
                line_number: 0,
                metadata: TaskMetadata::default(),
                body: None,
                contextual_notes: Vec::new(),
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
            has_explicit_id: false,
            title: "Test Task".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            metadata,
            body: None,
            contextual_notes: Vec::new(),
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
            has_explicit_id: false,
            title: "Test Task".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            metadata,
            body: None,
            contextual_notes: Vec::new(),
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
            has_explicit_id: false,
            title: "Test Task".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            metadata,
            body: None,
            contextual_notes: Vec::new(),
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
            has_explicit_id: false,
            title: "Test Task".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            metadata,
            body: None,
            contextual_notes: Vec::new(),
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
            has_explicit_id: false,
            title: "Test Task".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            metadata,
            body: None,
            contextual_notes: Vec::new(),
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
            has_explicit_id: false,
            title: "Test Task".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            metadata,
            body: None,
            contextual_notes: Vec::new(),
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
            has_explicit_id: false,
            title: "Test Task".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            metadata,
            body: None,
            contextual_notes: Vec::new(),
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

    /// Build a task carrying an arbitrary dependency kind + target.
    fn task_with_dep(target: &str, kind: DependencyKind) -> Task {
        let metadata = TaskMetadata {
            depends_on: vec![lash_types::dependency::DependencyRef::new(
                target.to_string(),
                kind,
            )],
            ..Default::default()
        };
        Task {
            id: "dependent".to_string(),
            has_explicit_id: true,
            title: "Dependent".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 1,
            line_number: 0,
            metadata,
            body: None,
            contextual_notes: Vec::new(),
        }
    }

    // Issue #15: the documented same-file form `#task:<id>` must resolve.
    #[test]
    fn test_samefile_task_form_resolves() {
        let rule = DependencyExistsRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();
        files.insert(
            PathBuf::from("current.md"),
            make_test_file("current.md", "repro", &["base-task"]),
        );

        let ctx = make_context(&config, PathBuf::from("current.md"), &files);
        let task = task_with_dep("#task:base-task", DependencyKind::ExplicitId);
        assert_eq!(rule.check_task(&task, &ctx).len(), 0);
    }

    // Issue #15: the documented cross-file form `file-id#task:<id>` must resolve.
    #[test]
    fn test_file_id_task_prefix_form_resolves() {
        let rule = DependencyExistsRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();
        files.insert(
            PathBuf::from("other.md"),
            make_test_file("other.md", "repro-file", &["base-task"]),
        );

        let ctx = make_context(&config, PathBuf::from("current.md"), &files);
        let task = task_with_dep("repro-file#task:base-task", DependencyKind::ExplicitId);
        assert_eq!(rule.check_task(&task, &ctx).len(), 0);
    }

    // Issue #15: a bare `@id` naming a task (not a file) must resolve.
    #[test]
    fn test_bare_task_id_resolves() {
        let rule = DependencyExistsRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();
        files.insert(
            PathBuf::from("current.md"),
            make_test_file("current.md", "repro", &["base-task"]),
        );

        let ctx = make_context(&config, PathBuf::from("current.md"), &files);
        let task = task_with_dep("base-task", DependencyKind::ExplicitId);
        assert_eq!(rule.check_task(&task, &ctx).len(), 0);
    }

    // Issue #18: a broken reference reports the @depends-on: line, not :0:0.
    #[test]
    fn test_broken_reference_reports_annotation_line() {
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let source = "# Repro\n\n@id: repro\n\n## Tasks\n\n\
                      - [ ] Dependent\n  \
                      @id: dep\n  \
                      @depends-on: does-not-exist\n";
        std::fs::write(root.join("tasks.md"), source).unwrap();

        let config = LashConfig {
            root_path: root.to_path_buf(),
            index_file: "index.md".to_string(),
            max_depth: 3,
            indent_spaces: 2,
            db_path: PathBuf::from(".lash/test.db"),
            custom_annotation_keys: vec![],
        };

        let rule = DependencyExistsRule::new();
        let files = HashMap::new();
        let ctx = make_context(&config, PathBuf::from("tasks.md"), &files);
        let task = task_with_dep("does-not-exist", DependencyKind::ExplicitId);

        let diags = rule.check_task(&task, &ctx);
        assert_eq!(diags.len(), 1);
        assert_eq!(
            diags[0].location.line,
            Some(9),
            "should point at the @depends-on: line, got {:?}",
            diags[0].location.line
        );
        assert!(diags[0].location.column.unwrap_or(0) > 0);
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
            has_explicit_id: false,
            title: "Test Task".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            metadata,
            body: None,
            contextual_notes: Vec::new(),
        };

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(diagnostics.len(), 0);
    }
}
