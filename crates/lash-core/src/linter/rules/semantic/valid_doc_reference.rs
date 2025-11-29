//! Documentation reference validation rule
//!
//! Validates that `@doc` annotations reference existing files and follow
//! proper security constraints:
//! - Referenced file exists on filesystem
//! - Path doesn't escape project root
//! - Path is not absolute

use lash_types::{dependency::DocRef, Severity, Task, TaskFile};
use std::path::{Component, Path};

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

        // Check 2 & 3: Resolve path and verify it doesn't escape project root
        // We need to check if the path escapes BEFORE normalization removes the ".." components
        let current_dir = ctx.file_path.parent().unwrap_or(Path::new(""));
        let joined = current_dir.join(doc_path);

        if Self::escapes_project_root(&joined) {
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

        // Now get the normalized/resolved path for file existence check
        let resolved_path = ctx.resolve_path(doc_path);

        // Check 4: Verify file exists
        // Prepend project root to get absolute path for filesystem check
        let absolute_path = ctx.config.root_path.join(&resolved_path);

        if !absolute_path.exists() {
            return Some(
                LintDiagnostic::error(
                    self.code(),
                    format!(
                        "Documentation file '{}' not found (resolved to: {})",
                        doc.path,
                        absolute_path.display()
                    ),
                    ctx.file_path.clone(),
                    0,
                    0,
                )
                .with_help(format!(
                    "Check that the file exists at: {}",
                    absolute_path.display()
                )),
            );
        }

        None
    }

    /// Check if a normalized path escapes the project root
    ///
    /// A path escapes the project root if it contains ".." components
    /// that would navigate above the implicit root.
    fn escapes_project_root(path: &Path) -> bool {
        let mut depth = 0i32;

        for component in path.components() {
            match component {
                Component::ParentDir => {
                    depth -= 1;
                    if depth < 0 {
                        return true; // Escaped root
                    }
                }
                Component::Normal(_) => {
                    depth += 1;
                }
                Component::CurDir => {
                    // Current directory, no change
                }
                Component::RootDir | Component::Prefix(_) => {
                    // Absolute paths shouldn't reach here (caught earlier)
                    return true;
                }
            }
        }

        false
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
    fn test_escapes_project_root() {
        // These should escape
        assert!(ValidDocReferenceRule::escapes_project_root(Path::new(
            "../outside"
        )));
        assert!(ValidDocReferenceRule::escapes_project_root(Path::new(
            "../../outside"
        )));
        assert!(ValidDocReferenceRule::escapes_project_root(Path::new(
            "a/b/../../.."
        )));

        // These should NOT escape
        assert!(!ValidDocReferenceRule::escapes_project_root(Path::new(
            "docs/file.md"
        )));
        assert!(!ValidDocReferenceRule::escapes_project_root(Path::new(
            "./docs/file.md"
        )));
        assert!(!ValidDocReferenceRule::escapes_project_root(Path::new(
            "a/../b/file.md"
        )));
        assert!(!ValidDocReferenceRule::escapes_project_root(Path::new(
            "a/b/../../c"
        )));
    }

    #[test]
    fn test_absolute_path_rejected() {
        let temp_dir = TempDir::new().unwrap();
        let config = make_config_with_root(temp_dir.path().to_path_buf());
        let files = HashMap::new();
        let ctx = make_context_with_config(&config, PathBuf::from("test.md"), &files);

        let rule = ValidDocReferenceRule::new();
        let doc = DocRef::new("/absolute/path/file.md", None);

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
        task.metadata.docs.push(DocRef::new("/absolute.md", None)); // Invalid

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
}
