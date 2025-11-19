//! Rule: No orphaned files
//!
//! Warns about .md files in the project that are not referenced in the root index.
//!
//! Error code: `W_INDEX_ORPHAN`

use lash_types::{Severity, TaskFile};
use std::path::Path;

use crate::linter::{LintContext, LintDiagnostic, LintRule};

/// Rule that detects orphaned files not in the index
///
/// This rule checks that all task files in the project are referenced in the root
/// index. Files not in the index are considered "orphaned" and generate a warning.
///
/// # Examples
///
/// Valid (no orphaned files):
/// ```markdown
/// # Index (lash.index.md)
/// - [ ] tasks.md
/// - [ ] notes.md
/// ```
///
/// Warning (W_INDEX_ORPHAN):
/// ```markdown
/// # Index (lash.index.md)
/// - [ ] tasks.md
/// // notes.md exists in project but not in index
/// ```
pub struct OrphanedFilesRule;

impl OrphanedFilesRule {
    /// Create a new orphaned files rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Check if this file is a root index file
    fn is_root_index(&self, file_path: &Path) -> bool {
        let file_name = file_path.file_name().and_then(|n| n.to_str());
        matches!(file_name, Some("lash.index.md") | Some("index.lash.md"))
    }

    /// Extract file references from the index file
    fn extract_file_references(&self, file: &TaskFile) -> Vec<String> {
        let mut references = Vec::new();

        for task in file.tasks.tasks() {
            let title = task.title.trim();
            if title.ends_with(".md") {
                references.push(title.to_string());
            }
        }

        references
    }

    /// Find the root index file in the context
    fn find_root_index<'a>(&self, ctx: &'a LintContext) -> Option<&'a TaskFile> {
        ctx.all_files
            .iter()
            .find(|(path, _)| self.is_root_index(path))
            .map(|(_, file)| file)
    }
}

impl Default for OrphanedFilesRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRule for OrphanedFilesRule {
    fn code(&self) -> &'static str {
        "W_INDEX_ORPHAN"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn description(&self) -> &'static str {
        "Warns about task files not referenced in the root index"
    }

    fn check_file(&self, file: &TaskFile, ctx: &LintContext) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        // Skip check if this is the index file itself
        if self.is_root_index(&file.path) {
            return diagnostics;
        }

        // Skip check if we don't have the full context
        if ctx.all_files.is_empty() {
            return diagnostics;
        }

        // Find the root index file
        let Some(index_file) = self.find_root_index(ctx) else {
            // No index file in project, can't check for orphans
            return diagnostics;
        };

        // Extract file references from the index
        let referenced_files = self.extract_file_references(index_file);

        // Check if the current file is referenced in the index
        let is_referenced = referenced_files.iter().any(|ref_path| {
            // Try exact match first
            if ref_path == file.path.to_str().unwrap_or("") {
                return true;
            }

            // Try resolving relative to index directory
            if let Some(index_dir) = index_file.path.parent() {
                let resolved = if index_dir == Path::new("") {
                    Path::new(ref_path).to_path_buf()
                } else {
                    index_dir.join(ref_path)
                };
                if resolved == file.path {
                    return true;
                }
            }

            // Try as relative path from project root
            let resolved = Path::new(ref_path);
            resolved == file.path
        });

        if !is_referenced {
            diagnostics.push(
                LintDiagnostic::warning(
                    self.code(),
                    format!(
                        "File '{}' is not referenced in the root index",
                        file.path.display()
                    ),
                    file.path.clone(),
                    0,
                    0,
                )
                .with_help(format!(
                    "Add '{}' to {} or move to an archive directory",
                    file.path.display(),
                    index_file.path.display()
                )),
            );
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::{
        task::{Task, TaskMetadata, TaskTree},
        FileMetadata, LashConfig, TaskStatus,
    };
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::SystemTime;

    fn make_index_file(path: &str, referenced_files: Vec<&str>) -> TaskFile {
        let mut tasks = TaskTree::new();

        for (i, file_ref) in referenced_files.iter().enumerate() {
            tasks.add_task(Task {
                id: format!("entry-{}", i),
                title: file_ref.to_string(),
                status: TaskStatus::Open,
                depth: 0,
                parent_id: None,
                order_index: i,
                metadata: TaskMetadata::default(),
                body: None,
            });
        }

        TaskFile {
            path: PathBuf::from(path),
            title: "Project Index".to_string(),
            id: "index".to_string(),
            metadata: FileMetadata::default(),
            tasks,
            hash: "test-hash".to_string(),
            mtime: SystemTime::now(),
        }
    }

    fn make_regular_file(path: &str, id: &str) -> TaskFile {
        TaskFile {
            path: PathBuf::from(path),
            title: "Regular File".to_string(),
            id: id.to_string(),
            metadata: FileMetadata::default(),
            tasks: TaskTree::new(),
            hash: "test-hash".to_string(),
            mtime: SystemTime::now(),
        }
    }

    #[test]
    fn test_no_orphans_all_files_in_index() {
        let rule = OrphanedFilesRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();

        files.insert(
            PathBuf::from("lash.index.md"),
            make_index_file("lash.index.md", vec!["tasks.md", "notes.md"]),
        );
        files.insert(
            PathBuf::from("tasks.md"),
            make_regular_file("tasks.md", "tasks"),
        );
        files.insert(
            PathBuf::from("notes.md"),
            make_regular_file("notes.md", "notes"),
        );

        let ctx = LintContext::new(&config, PathBuf::from("tasks.md"), &files);
        let tasks_file = files.get(&PathBuf::from("tasks.md")).unwrap();

        let diagnostics = rule.check_file(tasks_file, &ctx);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_orphaned_file() {
        let rule = OrphanedFilesRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();

        files.insert(
            PathBuf::from("lash.index.md"),
            make_index_file("lash.index.md", vec!["tasks.md"]),
        );
        files.insert(
            PathBuf::from("tasks.md"),
            make_regular_file("tasks.md", "tasks"),
        );
        files.insert(
            PathBuf::from("orphan.md"),
            make_regular_file("orphan.md", "orphan"),
        );

        let ctx = LintContext::new(&config, PathBuf::from("orphan.md"), &files);
        let orphan_file = files.get(&PathBuf::from("orphan.md")).unwrap();

        let diagnostics = rule.check_file(orphan_file, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "W_INDEX_ORPHAN");
        assert!(diagnostics[0].message.contains("orphan.md"));
    }

    #[test]
    fn test_index_file_not_checked() {
        let rule = OrphanedFilesRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();

        let index = make_index_file("lash.index.md", vec![]);
        files.insert(PathBuf::from("lash.index.md"), index.clone());

        let ctx = LintContext::new(&config, PathBuf::from("lash.index.md"), &files);

        let diagnostics = rule.check_file(&index, &ctx);
        // Index file itself should not be checked
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_no_index_file_no_warnings() {
        let rule = OrphanedFilesRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();

        let regular = make_regular_file("tasks.md", "tasks");
        files.insert(PathBuf::from("tasks.md"), regular.clone());

        let ctx = LintContext::new(&config, PathBuf::from("tasks.md"), &files);

        let diagnostics = rule.check_file(&regular, &ctx);
        // No index file, so can't determine orphans
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_relative_path_in_index() {
        let rule = OrphanedFilesRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();

        files.insert(
            PathBuf::from("lash.index.md"),
            make_index_file("lash.index.md", vec!["tasks/api.md"]),
        );
        files.insert(
            PathBuf::from("tasks/api.md"),
            make_regular_file("tasks/api.md", "api"),
        );

        let ctx = LintContext::new(&config, PathBuf::from("tasks/api.md"), &files);
        let api_file = files.get(&PathBuf::from("tasks/api.md")).unwrap();

        let diagnostics = rule.check_file(api_file, &ctx);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_empty_context() {
        let rule = OrphanedFilesRule::new();
        let config = LashConfig::default();
        let files = HashMap::new();

        let file = make_regular_file("orphan.md", "orphan");
        let ctx = LintContext::new(&config, PathBuf::from("orphan.md"), &files);

        let diagnostics = rule.check_file(&file, &ctx);
        // Empty context, can't check
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_alternate_index_name() {
        let rule = OrphanedFilesRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();

        files.insert(
            PathBuf::from("index.lash.md"),
            make_index_file("index.lash.md", vec!["tasks.md"]),
        );
        files.insert(
            PathBuf::from("tasks.md"),
            make_regular_file("tasks.md", "tasks"),
        );
        files.insert(
            PathBuf::from("orphan.md"),
            make_regular_file("orphan.md", "orphan"),
        );

        let ctx = LintContext::new(&config, PathBuf::from("orphan.md"), &files);
        let orphan_file = files.get(&PathBuf::from("orphan.md")).unwrap();

        let diagnostics = rule.check_file(orphan_file, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "W_INDEX_ORPHAN");
    }

    #[test]
    fn test_multiple_files_some_orphaned() {
        let rule = OrphanedFilesRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();

        files.insert(
            PathBuf::from("lash.index.md"),
            make_index_file("lash.index.md", vec!["tasks.md", "notes.md"]),
        );
        files.insert(
            PathBuf::from("tasks.md"),
            make_regular_file("tasks.md", "tasks"),
        );
        files.insert(
            PathBuf::from("notes.md"),
            make_regular_file("notes.md", "notes"),
        );
        files.insert(
            PathBuf::from("orphan1.md"),
            make_regular_file("orphan1.md", "orphan1"),
        );
        files.insert(
            PathBuf::from("orphan2.md"),
            make_regular_file("orphan2.md", "orphan2"),
        );

        // Check first orphan
        let ctx = LintContext::new(&config, PathBuf::from("orphan1.md"), &files);
        let orphan1 = files.get(&PathBuf::from("orphan1.md")).unwrap();
        let diagnostics = rule.check_file(orphan1, &ctx);
        assert_eq!(diagnostics.len(), 1);

        // Check second orphan
        let ctx = LintContext::new(&config, PathBuf::from("orphan2.md"), &files);
        let orphan2 = files.get(&PathBuf::from("orphan2.md")).unwrap();
        let diagnostics = rule.check_file(orphan2, &ctx);
        assert_eq!(diagnostics.len(), 1);

        // Check non-orphan
        let ctx = LintContext::new(&config, PathBuf::from("tasks.md"), &files);
        let tasks = files.get(&PathBuf::from("tasks.md")).unwrap();
        let diagnostics = rule.check_file(tasks, &ctx);
        assert_eq!(diagnostics.len(), 0);
    }
}
