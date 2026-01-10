//! Documentation reference validation rule
//!
//! Validates that `@doc` annotations reference existing files and follow
//! proper security constraints:
//! - Referenced file exists on filesystem
//! - Path doesn't escape project root
//! - Path is not absolute

use lash_types::{dependency::DocRef, Severity, Task, TaskFile};
use std::path::{Component, Path, PathBuf};

use crate::linter::{LintContext, LintDiagnostic, LintRule};

/// Rule that validates documentation references
///
/// This rule ensures that all `@doc` annotations point to valid documentation files:
/// - Files must exist relative to the file containing the annotation
/// - Paths cannot be absolute (must be relative)
/// - Paths cannot escape the project root (no `../../../` outside project)
///
/// **Code:** `E_SEM_INVALID_DOC`
/// **Severity:** Error
///
/// # Examples
///
/// Valid:
/// ```markdown
/// @doc: ../docs/design.md
/// @doc: ./README.md#section
/// @doc: docs/api.md
/// ```
///
/// Invalid (`E_SEM_INVALID_DOC`):
/// ```markdown
/// @doc: /absolute/path/file.md
/// @doc: ../../../outside-project.md
/// @doc: missing-file.md
/// ```
pub struct ValidDocReferenceRule;

impl ValidDocReferenceRule {
    /// Create a new valid doc reference rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Validate a single documentation reference
    ///
    /// Returns a diagnostic if the reference is invalid, None otherwise.
    fn validate_doc_ref(&self, doc: &DocRef, ctx: &LintContext) -> Option<LintDiagnostic> {
        let doc_path = Path::new(&doc.path);

        // Check 1: Reject absolute paths
        if doc_path.is_absolute() {
            return Some(
                LintDiagnostic::error(
                    self.code(),
                    format!("Documentation reference uses absolute path: '{}'", doc.path),
                    ctx.file_path.clone(),
                    0,
                    0,
                )
                .with_help(
                    "Use relative paths for documentation references (e.g., '../docs/design.md')",
                ),
            );
        }

        // Resolve the doc path to an absolute path
        // The file_path may be absolute (from file discovery) or relative (from tests)
        let absolute_path = if ctx.file_path.is_absolute() {
            // file_path is absolute, join with its parent directory
            ctx.file_path
                .parent()
                .unwrap_or(Path::new(""))
                .join(doc_path)
        } else {
            // file_path is relative, resolve from project root
            ctx.config
                .root_path
                .join(ctx.file_path.parent().unwrap_or(Path::new("")))
                .join(doc_path)
        };

        // Normalize the path to resolve .. components
        let normalized = Self::normalize_path(&absolute_path);

        // Check if the normalized path is within the project root
        if !Self::is_inside_project_root(&normalized, &ctx.config.root_path) {
            return Some(
                LintDiagnostic::error(
                    self.code(),
                    format!(
                        "Documentation reference escapes project root: '{}'",
                        doc.path
                    ),
                    ctx.file_path.clone(),
                    0,
                    0,
                )
                .with_help("Documentation references must stay within the project root directory"),
            );
        }

        // Check if file exists using the normalized path
        if !normalized.exists() {
            return Some(
                LintDiagnostic::error(
                    self.code(),
                    format!(
                        "Documentation file '{}' not found (resolved to: {})",
                        doc.path,
                        normalized.display()
                    ),
                    ctx.file_path.clone(),
                    0,
                    0,
                )
                .with_help(format!(
                    "Check that the file exists at: {}",
                    normalized.display()
                )),
            );
        }

        None
    }

    /// Normalize a path by resolving `.` and `..` components
    ///
    /// Unlike `canonicalize()`, this doesn't require the path to exist
    /// and doesn't resolve symlinks.
    fn normalize_path(path: &Path) -> PathBuf {
        let mut components = Vec::new();

        for component in path.components() {
            match component {
                Component::ParentDir => {
                    // Only pop if we have a Normal component to pop
                    if matches!(components.last(), Some(Component::Normal(_))) {
                        components.pop();
                    } else if !matches!(
                        components.last(),
                        Some(Component::RootDir | Component::Prefix(_))
                    ) {
                        // Keep ParentDir if we can't go up further (relative path)
                        components.push(component);
                    }
                    // If last is RootDir/Prefix, we're at the root, can't go higher
                }
                Component::CurDir => {
                    // Skip current directory markers
                }
                comp => {
                    components.push(comp);
                }
            }
        }

        components.iter().collect()
    }

    /// Check if a path is inside the project root
    ///
    /// Returns true if the normalized path starts with the project root.
    fn is_inside_project_root(path: &Path, root: &Path) -> bool {
        // Normalize both paths for comparison
        let normalized_path = Self::normalize_path(path);
        let normalized_root = Self::normalize_path(root);

        // Check if path starts with root
        normalized_path.starts_with(&normalized_root)
    }
}

impl Default for ValidDocReferenceRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRule for ValidDocReferenceRule {
    fn code(&self) -> &'static str {
        "E_SEM_INVALID_DOC"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> String {
        "Documentation reference validation".to_string()
    }

    fn description(&self) -> &'static str {
        "Validates that @doc annotations reference existing files within the project"
    }

    fn check_file(&self, file: &TaskFile, ctx: &LintContext) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        // Check file-level doc references
        for doc in &file.metadata.docs {
            if let Some(diag) = self.validate_doc_ref(doc, ctx) {
                diagnostics.push(diag);
            }
        }

        diagnostics
    }

    fn check_task(&self, task: &Task, ctx: &LintContext) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        // Check task-level doc references
        for doc in &task.metadata.docs {
            if let Some(diag) = self.validate_doc_ref(doc, ctx) {
                diagnostics.push(diag);
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::{
        dependency::DocRef, task::TaskBuilder, FileMetadata, LashConfig, TaskStatus, TaskTree,
    };
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn make_config_with_root(root: PathBuf) -> LashConfig {
        LashConfig {
            root_path: root,
            index_file: "index.md".to_string(),
            max_depth: 3,
            indent_spaces: 2,
            db_path: PathBuf::from(".lash/test.db"),
            custom_annotation_keys: vec![],
        }
    }

    fn make_context_with_config<'a>(
        config: &'a LashConfig,
        current_file: PathBuf,
        files: &'a HashMap<PathBuf, TaskFile>,
    ) -> LintContext<'a> {
        LintContext::new(config, current_file, files)
    }

    #[test]
    fn test_normalize_path() {
        // Simple paths stay the same
        assert_eq!(
            ValidDocReferenceRule::normalize_path(Path::new("docs/file.md")),
            PathBuf::from("docs/file.md")
        );

        // Current dir markers are removed
        assert_eq!(
            ValidDocReferenceRule::normalize_path(Path::new("./docs/file.md")),
            PathBuf::from("docs/file.md")
        );

        // Parent dirs are resolved
        assert_eq!(
            ValidDocReferenceRule::normalize_path(Path::new("a/../b/file.md")),
            PathBuf::from("b/file.md")
        );

        // Multiple parent dirs
        assert_eq!(
            ValidDocReferenceRule::normalize_path(Path::new("a/b/../../c")),
            PathBuf::from("c")
        );

        // Absolute paths work too
        #[cfg(unix)]
        {
            assert_eq!(
                ValidDocReferenceRule::normalize_path(Path::new("/project/tasks/../docs/file.md")),
                PathBuf::from("/project/docs/file.md")
            );
        }
    }

    #[test]
    fn test_is_inside_project_root() {
        #[cfg(unix)]
        {
            let root = Path::new("/project");

            // Inside project
            assert!(ValidDocReferenceRule::is_inside_project_root(
                Path::new("/project/docs/file.md"),
                root
            ));
            assert!(ValidDocReferenceRule::is_inside_project_root(
                Path::new("/project/tasks/../docs/file.md"),
                root
            ));

            // Outside project
            assert!(!ValidDocReferenceRule::is_inside_project_root(
                Path::new("/other/file.md"),
                root
            ));
            assert!(!ValidDocReferenceRule::is_inside_project_root(
                Path::new("/project/../outside/file.md"),
                root
            ));
        }
    }

    #[test]
    fn test_absolute_path_rejected() {
        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());
        let files = HashMap::new();
        let ctx = make_context_with_config(&config, PathBuf::from("test.md"), &files);

        let rule = ValidDocReferenceRule::new();

        // Use a path that's guaranteed to be absolute on the current platform
        // On Unix: /absolute/path/file.md
        // On Windows: C:\absolute\path\file.md
        #[cfg(unix)]
        let absolute_path = "/absolute/path/file.md";
        #[cfg(windows)]
        let absolute_path = "C:\\absolute\\path\\file.md";

        let doc = DocRef::new(absolute_path, None);

        let result = rule.validate_doc_ref(&doc, &ctx);
        assert!(result.is_some());
        let diag = result.unwrap();
        assert_eq!(diag.code, "E_SEM_INVALID_DOC");
        assert!(diag.message.contains("absolute path"));
    }

    #[test]
    fn test_escaping_path_rejected() {
        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());
        let files = HashMap::new();
        let ctx = make_context_with_config(&config, PathBuf::from("test.md"), &files);

        let rule = ValidDocReferenceRule::new();
        let doc = DocRef::new("../../outside.md", None);

        let result = rule.validate_doc_ref(&doc, &ctx);
        assert!(result.is_some());
        let diag = result.unwrap();
        assert_eq!(diag.code, "E_SEM_INVALID_DOC");
        assert!(diag.message.contains("escapes project root"));
    }

    #[test]
    fn test_missing_file_rejected() {
        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());
        let files = HashMap::new();
        let ctx = make_context_with_config(&config, PathBuf::from("test.md"), &files);

        let rule = ValidDocReferenceRule::new();
        let doc = DocRef::new("missing.md", None);

        let result = rule.validate_doc_ref(&doc, &ctx);
        assert!(result.is_some());
        let diag = result.unwrap();
        assert_eq!(diag.code, "E_SEM_INVALID_DOC");
        assert!(diag.message.contains("not found"));
    }

    #[test]
    fn test_existing_file_passes() {
        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());

        // Create a test documentation file
        let doc_path = temp_dir.path().join("docs").join("design.md");
        fs::create_dir_all(doc_path.parent().unwrap()).unwrap();
        fs::write(&doc_path, "# Design Document").unwrap();

        let files = HashMap::new();
        let ctx = make_context_with_config(&config, PathBuf::from("test.md"), &files);

        let rule = ValidDocReferenceRule::new();
        let doc = DocRef::new("docs/design.md", None);

        let result = rule.validate_doc_ref(&doc, &ctx);
        assert!(result.is_none(), "Valid doc reference should pass");
    }

    #[test]
    fn test_relative_path_with_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());

        // Create structure:
        // - tasks/feature.md (current file)
        // - docs/design.md (doc reference)
        let tasks_dir = temp_dir.path().join("tasks");
        let docs_dir = temp_dir.path().join("docs");
        fs::create_dir_all(&tasks_dir).unwrap();
        fs::create_dir_all(&docs_dir).unwrap();
        fs::write(docs_dir.join("design.md"), "# Design").unwrap();

        let files = HashMap::new();
        let ctx = make_context_with_config(&config, PathBuf::from("tasks/feature.md"), &files);

        let rule = ValidDocReferenceRule::new();
        let doc = DocRef::new("../docs/design.md", None);

        let result = rule.validate_doc_ref(&doc, &ctx);
        assert!(result.is_none(), "Valid relative path should pass");
    }

    #[test]
    fn test_file_level_doc_validation() {
        use lash_types::TaskFile;
        use std::time::SystemTime;

        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());

        // Create a valid doc file
        fs::write(temp_dir.path().join("README.md"), "# README").unwrap();

        let files = HashMap::new();
        let ctx = make_context_with_config(&config, PathBuf::from("test.md"), &files);

        let rule = ValidDocReferenceRule::new();

        // Create a file with both valid and invalid doc refs

        let mut metadata = FileMetadata::default();
        metadata.docs.push(DocRef::new("README.md", None)); // Valid
        metadata.docs.push(DocRef::new("missing.md", None)); // Invalid

        let file = TaskFile {
            path: PathBuf::from("test.md"),
            title: "Test".to_string(),
            id: "test".to_string(),
            metadata,
            description: None,
            description_agent_notes: Vec::new(),
            tasks: TaskTree::new(),
            hash: "hash".to_string(),
            mtime: SystemTime::now(),
        };

        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1, "Should have one error for missing.md");
        assert_eq!(diagnostics[0].code, "E_SEM_INVALID_DOC");
        assert!(diagnostics[0].message.contains("missing.md"));
    }

    #[test]
    fn test_task_level_doc_validation() {
        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());

        // Create a valid doc file
        fs::write(temp_dir.path().join("guide.md"), "# Guide").unwrap();

        let files = HashMap::new();
        let ctx = make_context_with_config(&config, PathBuf::from("test.md"), &files);

        let rule = ValidDocReferenceRule::new();

        // Create a task with doc refs
        let mut task = TaskBuilder::new("Test task")
            .id("task-1")
            .status(TaskStatus::Open)
            .build()
            .unwrap();

        task.metadata.docs.push(DocRef::new("guide.md", None)); // Valid

        // Use platform-specific absolute path
        #[cfg(unix)]
        let absolute_doc_path = "/absolute.md";
        #[cfg(windows)]
        let absolute_doc_path = "C:\\absolute.md";
        task.metadata
            .docs
            .push(DocRef::new(absolute_doc_path, None)); // Invalid

        let diagnostics = rule.check_task(&task, &ctx);
        assert_eq!(
            diagnostics.len(),
            1,
            "Should have one error for absolute path"
        );
        assert_eq!(diagnostics[0].code, "E_SEM_INVALID_DOC");
        assert!(diagnostics[0].message.contains("absolute"));
    }

    #[test]
    fn test_doc_with_fragment() {
        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());

        // Create a doc file
        fs::write(temp_dir.path().join("design.md"), "# Design\n## Section").unwrap();

        let files = HashMap::new();
        let ctx = make_context_with_config(&config, PathBuf::from("test.md"), &files);

        let rule = ValidDocReferenceRule::new();
        let doc = DocRef::new("design.md", Some("section".to_string()));

        let result = rule.validate_doc_ref(&doc, &ctx);
        assert!(
            result.is_none(),
            "Doc with fragment should pass if file exists"
        );
    }

    #[test]
    fn test_rule_metadata() {
        let rule = ValidDocReferenceRule::new();
        assert_eq!(rule.code(), "E_SEM_INVALID_DOC");
        assert_eq!(rule.severity(), Severity::Error);
        assert_eq!(rule.name(), "Documentation reference validation");
        assert!(!rule.description().is_empty());
    }

    #[test]
    fn test_absolute_file_path_with_relative_doc_reference() {
        // This tests the scenario where file discovery returns absolute paths
        // and the doc reference uses ".." to go up directories
        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());

        // Create structure:
        // - tasks/feature.md (current file, with ABSOLUTE path in context)
        // - docs/design.md (doc reference using ../docs/design.md)
        let tasks_dir = temp_dir.path().join("tasks");
        let docs_dir = temp_dir.path().join("docs");
        fs::create_dir_all(&tasks_dir).unwrap();
        fs::create_dir_all(&docs_dir).unwrap();
        fs::write(docs_dir.join("design.md"), "# Design").unwrap();

        // Use ABSOLUTE path for the file (like file discovery returns)
        let absolute_file_path = tasks_dir.join("feature.md");

        let files = HashMap::new();
        let ctx = make_context_with_config(&config, absolute_file_path, &files);

        let rule = ValidDocReferenceRule::new();
        let doc = DocRef::new("../docs/design.md", None);

        let result = rule.validate_doc_ref(&doc, &ctx);
        assert!(
            result.is_none(),
            "Valid relative doc reference from absolute file path should pass"
        );
    }
}
