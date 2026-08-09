//! Rule: Root index file references must exist
//!
//! Validates that all file paths referenced in the root index file exist.
//!
//! Error code: `E_INDEX_FILE_MISSING`

use lash_types::{Severity, TaskFile};
use std::path::Path;

use crate::linter::{LintContext, LintDiagnostic, LintRule};

/// Rule that checks root index file references exist
///
/// This rule validates that all files referenced in the root index exist in the project.
/// The root index is the entry point (lash.index.md or index.lash.md) that lists all
/// task files.
///
/// # Examples
///
/// Valid:
/// ```markdown
/// # Project Index
/// - [ ] tasks/api.md
/// - [ ] tasks/ui.md
/// ```
///
/// Invalid (`E_INDEX_FILE_MISSING`):
/// ```markdown
/// # Project Index
/// - [ ] tasks/missing.md  // File doesn't exist
/// ```
pub struct IndexFileRefsRule;

impl IndexFileRefsRule {
    /// Create a new index file references rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Check if this file is a root index file
    fn is_root_index(file: &TaskFile) -> bool {
        let file_name = file.path.file_name().and_then(|n| n.to_str());
        matches!(file_name, Some("lash.index.md" | "index.lash.md"))
    }

    /// Parse index entries from the file
    ///
    /// This extracts file paths from checkbox items in the file.
    /// The `RootIndex` type would normally be created by a proper parser,
    /// but for linting purposes we can extract paths directly from tasks.
    fn extract_file_references(file: &TaskFile) -> Vec<String> {
        let mut references = Vec::new();

        for task in file.tasks.tasks() {
            // Check if the task title looks like a file reference
            let title = task.title.trim();
            if std::path::Path::new(title)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            {
                references.push(title.to_string());
            }
        }

        references
    }
}

impl Default for IndexFileRefsRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRule for IndexFileRefsRule {
    fn code(&self) -> &'static str {
        "E_INDEX_FILE_MISSING"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn description(&self) -> &'static str {
        "Validates that files referenced in the root index exist"
    }

    fn check_file(&self, file: &TaskFile, ctx: &LintContext) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        // Only check if this is the root index file
        if !Self::is_root_index(file) {
            return diagnostics;
        }

        // Extract file references from the index
        let file_refs = Self::extract_file_references(file);

        // Check each reference
        for file_ref in file_refs {
            // Resolve the path relative to the index file's directory
            let ref_path = Path::new(&file_ref);
            let resolved = if ref_path.is_absolute() {
                ref_path.to_path_buf()
            } else {
                // Resolve relative to the index file's directory (usually project root)
                if let Some(index_dir) = file.path.parent() {
                    index_dir.join(ref_path)
                } else {
                    ref_path.to_path_buf()
                }
            };

            // Check if the file exists in the context
            if ctx.get_file(&resolved).is_none() {
                diagnostics.push(
                    LintDiagnostic::error(
                        self.code(),
                        format!(
                            "File '{file_ref}' referenced in index does not exist (resolved to: {})",
                            resolved.display()
                        ),
                        file.path.clone(),
                        0,
                        0,
                    )
                    .with_help(format!(
                        "Create the file at {} or remove it from the index",
                        resolved.display()
                    )),
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
        task::{Task, TaskMetadata, TaskTree},
        FileMetadata, LashConfig, TaskStatus,
    };
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::SystemTime;

    fn make_index_file(path: &str, referenced_files: &[&str]) -> TaskFile {
        let mut tasks = TaskTree::new();

        for (i, file_ref) in referenced_files.iter().enumerate() {
            let _ = tasks.add_task(Task {
                id: format!("entry-{i}"),
                has_explicit_id: false,
                title: (*file_ref).to_string(),
                status: TaskStatus::Open,
                depth: 0,
                parent_id: None,
                order_index: i,
                line_number: 0,
                annotation_line_count: 0,
                metadata: TaskMetadata::default(),
                body: None,
                contextual_notes: Vec::new(),
            });
        }

        TaskFile {
            path: PathBuf::from(path),
            title: "Project Index".to_string(),
            id: "index".to_string(),
            metadata: FileMetadata::default(),
            description: None,
            description_agent_notes: Vec::new(),
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
            description: None,
            description_agent_notes: Vec::new(),
            tasks: TaskTree::new(),
            hash: "test-hash".to_string(),
            mtime: SystemTime::now(),
        }
    }

    #[test]
    fn test_valid_index_all_files_exist() {
        let rule = IndexFileRefsRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();

        // Add index file
        let index = make_index_file("lash.index.md", &["tasks.md", "notes.md"]);
        files.insert(PathBuf::from("lash.index.md"), index.clone());

        // Add referenced files
        files.insert(
            PathBuf::from("tasks.md"),
            make_regular_file("tasks.md", "tasks"),
        );
        files.insert(
            PathBuf::from("notes.md"),
            make_regular_file("notes.md", "notes"),
        );

        let ctx = LintContext::new(&config, PathBuf::from("lash.index.md"), &files);

        let diagnostics = rule.check_file(&index, &ctx);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_missing_file_in_index() {
        let rule = IndexFileRefsRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();

        // Add index file referencing a missing file
        let index = make_index_file("lash.index.md", &["tasks.md", "missing.md"]);
        files.insert(PathBuf::from("lash.index.md"), index.clone());

        // Add only one of the referenced files
        files.insert(
            PathBuf::from("tasks.md"),
            make_regular_file("tasks.md", "tasks"),
        );

        let ctx = LintContext::new(&config, PathBuf::from("lash.index.md"), &files);

        let diagnostics = rule.check_file(&index, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_INDEX_FILE_MISSING");
        assert!(diagnostics[0].message.contains("missing.md"));
    }

    #[test]
    fn test_multiple_missing_files() {
        let rule = IndexFileRefsRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();

        let index = make_index_file(
            "lash.index.md",
            &["exists.md", "missing1.md", "missing2.md"],
        );
        files.insert(PathBuf::from("lash.index.md"), index.clone());
        files.insert(
            PathBuf::from("exists.md"),
            make_regular_file("exists.md", "exists"),
        );

        let ctx = LintContext::new(&config, PathBuf::from("lash.index.md"), &files);

        let diagnostics = rule.check_file(&index, &ctx);
        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics[0].message.contains("missing1.md"));
        assert!(diagnostics[1].message.contains("missing2.md"));
    }

    #[test]
    fn test_non_index_file_not_checked() {
        let rule = IndexFileRefsRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();

        // Regular file that happens to reference other files
        let regular = make_index_file("regular.md", &["missing.md"]);
        files.insert(PathBuf::from("regular.md"), regular.clone());

        let ctx = LintContext::new(&config, PathBuf::from("regular.md"), &files);

        let diagnostics = rule.check_file(&regular, &ctx);
        // Should not check non-index files
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_index_lash_md_variant() {
        let rule = IndexFileRefsRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();

        // Use the alternate index file name
        let index = make_index_file("index.lash.md", &["missing.md"]);
        files.insert(PathBuf::from("index.lash.md"), index.clone());

        let ctx = LintContext::new(&config, PathBuf::from("index.lash.md"), &files);

        let diagnostics = rule.check_file(&index, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_INDEX_FILE_MISSING");
    }

    #[test]
    fn test_relative_path_in_index() {
        let rule = IndexFileRefsRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();

        let index = make_index_file("lash.index.md", &["tasks/api.md", "tasks/ui.md"]);
        files.insert(PathBuf::from("lash.index.md"), index.clone());

        // Add referenced files with relative paths
        files.insert(
            PathBuf::from("tasks/api.md"),
            make_regular_file("tasks/api.md", "api"),
        );
        files.insert(
            PathBuf::from("tasks/ui.md"),
            make_regular_file("tasks/ui.md", "ui"),
        );

        let ctx = LintContext::new(&config, PathBuf::from("lash.index.md"), &files);

        let diagnostics = rule.check_file(&index, &ctx);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_empty_index() {
        let rule = IndexFileRefsRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();

        let index = make_index_file("lash.index.md", &[]);
        files.insert(PathBuf::from("lash.index.md"), index.clone());

        let ctx = LintContext::new(&config, PathBuf::from("lash.index.md"), &files);

        let diagnostics = rule.check_file(&index, &ctx);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_is_root_index() {
        let index1 = make_regular_file("lash.index.md", "index");
        assert!(IndexFileRefsRule::is_root_index(&index1));

        let index2 = make_regular_file("index.lash.md", "index");
        assert!(IndexFileRefsRule::is_root_index(&index2));

        let regular = make_regular_file("tasks.md", "tasks");
        assert!(!IndexFileRefsRule::is_root_index(&regular));
    }

    #[test]
    fn test_non_md_entries_ignored() {
        let rule = IndexFileRefsRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();

        // Index with non-.md entries (should be ignored)
        let mut tasks = TaskTree::new();
        let _ = tasks.add_task(Task {
            id: "entry-1".to_string(),
            has_explicit_id: false,
            title: "tasks.md".to_string(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            annotation_line_count: 0,
            metadata: TaskMetadata::default(),
            body: None,
            contextual_notes: Vec::new(),
        });
        let _ = tasks.add_task(Task {
            id: "entry-2".to_string(),
            has_explicit_id: false,
            title: "Some regular text entry".to_string(), // Not a file reference
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 1,
            line_number: 0,
            annotation_line_count: 0,
            metadata: TaskMetadata::default(),
            body: None,
            contextual_notes: Vec::new(),
        });

        let index = TaskFile {
            path: PathBuf::from("lash.index.md"),
            title: "Project Index".to_string(),
            id: "index".to_string(),
            metadata: FileMetadata::default(),
            description: None,
            description_agent_notes: Vec::new(),
            tasks,
            hash: "test-hash".to_string(),
            mtime: SystemTime::now(),
        };

        files.insert(PathBuf::from("lash.index.md"), index.clone());
        files.insert(
            PathBuf::from("tasks.md"),
            make_regular_file("tasks.md", "tasks"),
        );

        let ctx = LintContext::new(&config, PathBuf::from("lash.index.md"), &files);

        let diagnostics = rule.check_file(&index, &ctx);
        // Should only check the .md entry, not the regular text
        assert_eq!(diagnostics.len(), 0);
    }
}
