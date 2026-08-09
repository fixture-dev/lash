//! Rule: No orphaned files
//!
//! Warns about .md files in the project that are not referenced in the root index.
//!
//! Error code: `W_INDEX_ORPHAN`

use lash_types::{Severity, TaskFile};
use std::fs;
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
/// Warning (`W_INDEX_ORPHAN`):
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
    fn is_root_index(file_path: &Path) -> bool {
        let file_name = file_path.file_name().and_then(|n| n.to_str());
        matches!(file_name, Some("lash.index.md" | "index.lash.md"))
    }

    /// Check if this file should be excluded from orphan checking
    ///
    /// Excludes common non-task files like documentation and configuration files.
    fn should_skip_orphan_check(file_path: &Path) -> bool {
        // Common documentation/config file names to exclude
        const EXCLUDED_FILENAMES: &[&str] = &[
            "README.md",
            "CLAUDE.md",
            "AGENTS.md",
            "CHANGELOG.md",
            "CONTRIBUTING.md",
            "LICENSE.md",
            "CODE_OF_CONDUCT.md",
            "SECURITY.md",
            "SUPPORT.md",
            "AUTHORS.md",
            "HISTORY.md",
            "ROADMAP.md",
            "CODEOWNERS",
            "devlog.md",
        ];

        // Common documentation directory names to exclude
        const EXCLUDED_DIRS: &[&str] = &["docs", "documentation", "doc", ".github"];

        // Check filename
        if let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) {
            // Case-insensitive check for excluded filenames
            let file_name_upper = file_name.to_uppercase();
            if EXCLUDED_FILENAMES
                .iter()
                .any(|ex| file_name_upper == ex.to_uppercase())
            {
                return true;
            }
        }

        // Check if file is in an excluded directory
        for component in file_path.components() {
            if let std::path::Component::Normal(dir_name) = component {
                if let Some(dir_str) = dir_name.to_str() {
                    let dir_lower = dir_str.to_lowercase();
                    if EXCLUDED_DIRS.iter().any(|ex| dir_lower == *ex) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Extract file references from the index file
    ///
    /// Handles both plain path titles and markdown link format:
    /// - Plain: `tasks.md` or `path/to/file.md`
    /// - Markdown link: `[Title](path/to/file.md)`
    /// - Directory reference: `[Title](path/to/dir/)` (ends with /)
    ///
    /// Also scans the raw file content for markdown links in non-task content
    /// (e.g., bullet points that aren't checkboxes).
    fn extract_file_references(file: &TaskFile) -> Vec<String> {
        let mut references = Vec::new();

        // Extract from tasks (checkbox items)
        for task in file.tasks.tasks() {
            let title = task.title.trim();

            // Try to extract path from markdown link [text](path)
            if let Some(path) = Self::extract_markdown_link_path(title) {
                references.push(path);
                continue;
            }

            // Fall back to checking if the title itself is a path
            if std::path::Path::new(title)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            {
                references.push(title.to_string());
            }
        }

        // Also extract from raw content (for non-task markdown links)
        if let Ok(content) = fs::read_to_string(&file.path) {
            for line in content.lines() {
                let line = line.trim();
                // Skip checkbox items (already handled above as tasks)
                if line.starts_with("- [ ]")
                    || line.starts_with("- [x]")
                    || line.starts_with("- [-]")
                    || line.starts_with("- [!]")
                {
                    continue;
                }
                // Check for markdown link on this line
                if let Some(path) = Self::extract_markdown_link_path(line) {
                    if !references.contains(&path) {
                        references.push(path);
                    }
                }
            }
        }

        references
    }

    /// Extract path from a markdown link format: `\[text\](path)`
    ///
    /// Returns Some(path) if found, None otherwise
    fn extract_markdown_link_path(text: &str) -> Option<String> {
        // Look for pattern [...](...)
        let open_paren = text.find("](")?;
        let close_paren = text.rfind(')')?;

        if open_paren + 2 < close_paren {
            let path = &text[open_paren + 2..close_paren];
            // Accept paths ending in .md or directories ending in /
            let is_md = std::path::Path::new(path)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"));
            if is_md || path.ends_with('/') {
                return Some(path.to_string());
            }
        }

        None
    }

    /// Find the root index file in the context
    fn find_root_index<'a>(ctx: &'a LintContext) -> Option<&'a TaskFile> {
        ctx.all_files
            .iter()
            .find(|(path, _)| Self::is_root_index(path))
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
        if Self::is_root_index(&file.path) {
            return diagnostics;
        }

        // Skip check for common non-task files (README, docs, etc.)
        if Self::should_skip_orphan_check(&file.path) {
            return diagnostics;
        }

        // Skip check if we don't have the full context
        if ctx.all_files.is_empty() {
            return diagnostics;
        }

        // Find the root index file
        let Some(index_file) = Self::find_root_index(ctx) else {
            // No index file in project, can't check for orphans
            return diagnostics;
        };

        // Extract file references from the index
        let referenced_files = Self::extract_file_references(index_file);

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
    fn test_no_orphans_all_files_in_index() {
        let rule = OrphanedFilesRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();

        files.insert(
            PathBuf::from("lash.index.md"),
            make_index_file("lash.index.md", &["tasks.md", "notes.md"]),
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
            make_index_file("lash.index.md", &["tasks.md"]),
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

        let index = make_index_file("lash.index.md", &[]);
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
            make_index_file("lash.index.md", &["tasks/api.md"]),
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
            make_index_file("index.lash.md", &["tasks.md"]),
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
            make_index_file("lash.index.md", &["tasks.md", "notes.md"]),
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

    #[test]
    fn test_extract_markdown_link_path() {
        // Standard markdown link
        assert_eq!(
            OrphanedFilesRule::extract_markdown_link_path("[Title](path/to/file.md)"),
            Some("path/to/file.md".to_string())
        );

        // Link with annotations after
        assert_eq!(
            OrphanedFilesRule::extract_markdown_link_path(
                "[Physics](systems/physics.md) @id:`systems.physics`"
            ),
            Some("systems/physics.md".to_string())
        );

        // Directory reference
        assert_eq!(
            OrphanedFilesRule::extract_markdown_link_path("[World 1](worlds/forest/)"),
            Some("worlds/forest/".to_string())
        );

        // Plain text (no link)
        assert_eq!(
            OrphanedFilesRule::extract_markdown_link_path("Just plain text"),
            None
        );

        // Plain path (no markdown link format)
        assert_eq!(
            OrphanedFilesRule::extract_markdown_link_path("path/to/file.md"),
            None
        );
    }

    #[test]
    fn test_markdown_link_references_in_index() {
        let rule = OrphanedFilesRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();

        // Create index with markdown links
        files.insert(
            PathBuf::from("lash.index.md"),
            make_index_file(
                "lash.index.md",
                &[
                    "[Physics](systems/physics.md)",
                    "[Input](systems/input.md)",
                    "[World 1](worlds/forest/)",
                ],
            ),
        );
        files.insert(
            PathBuf::from("systems/physics.md"),
            make_regular_file("systems/physics.md", "physics"),
        );
        files.insert(
            PathBuf::from("systems/input.md"),
            make_regular_file("systems/input.md", "input"),
        );
        files.insert(
            PathBuf::from("orphan.md"),
            make_regular_file("orphan.md", "orphan"),
        );

        // Physics should not be orphaned
        let ctx = LintContext::new(&config, PathBuf::from("systems/physics.md"), &files);
        let physics = files.get(&PathBuf::from("systems/physics.md")).unwrap();
        let diagnostics = rule.check_file(physics, &ctx);
        assert_eq!(diagnostics.len(), 0);

        // Input should not be orphaned
        let ctx = LintContext::new(&config, PathBuf::from("systems/input.md"), &files);
        let input = files.get(&PathBuf::from("systems/input.md")).unwrap();
        let diagnostics = rule.check_file(input, &ctx);
        assert_eq!(diagnostics.len(), 0);

        // Orphan should still be orphaned
        let ctx = LintContext::new(&config, PathBuf::from("orphan.md"), &files);
        let orphan = files.get(&PathBuf::from("orphan.md")).unwrap();
        let diagnostics = rule.check_file(orphan, &ctx);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_should_skip_orphan_check_excluded_files() {
        // Common documentation files should be skipped
        assert!(OrphanedFilesRule::should_skip_orphan_check(Path::new(
            "README.md"
        )));
        assert!(OrphanedFilesRule::should_skip_orphan_check(Path::new(
            "CLAUDE.md"
        )));
        assert!(OrphanedFilesRule::should_skip_orphan_check(Path::new(
            "AGENTS.md"
        )));
        assert!(OrphanedFilesRule::should_skip_orphan_check(Path::new(
            "CHANGELOG.md"
        )));
        assert!(OrphanedFilesRule::should_skip_orphan_check(Path::new(
            "devlog.md"
        )));

        // Case-insensitive
        assert!(OrphanedFilesRule::should_skip_orphan_check(Path::new(
            "readme.md"
        )));
        assert!(OrphanedFilesRule::should_skip_orphan_check(Path::new(
            "Readme.MD"
        )));
    }

    #[test]
    fn test_should_skip_orphan_check_excluded_directories() {
        // Files in docs/ directory should be skipped
        assert!(OrphanedFilesRule::should_skip_orphan_check(Path::new(
            "docs/design.md"
        )));
        assert!(OrphanedFilesRule::should_skip_orphan_check(Path::new(
            "docs/api/reference.md"
        )));

        // Files in documentation/ directory should be skipped
        assert!(OrphanedFilesRule::should_skip_orphan_check(Path::new(
            "documentation/guide.md"
        )));

        // Files in .github/ directory should be skipped
        assert!(OrphanedFilesRule::should_skip_orphan_check(Path::new(
            ".github/ISSUE_TEMPLATE.md"
        )));
    }

    #[test]
    fn test_should_skip_orphan_check_task_files() {
        // Regular task files should NOT be skipped
        assert!(!OrphanedFilesRule::should_skip_orphan_check(Path::new(
            "tasks.md"
        )));
        assert!(!OrphanedFilesRule::should_skip_orphan_check(Path::new(
            "tasks/api.md"
        )));
        assert!(!OrphanedFilesRule::should_skip_orphan_check(Path::new(
            "tasks/milestone-1.md"
        )));
        assert!(!OrphanedFilesRule::should_skip_orphan_check(Path::new(
            "notes.md"
        )));
    }

    #[test]
    fn test_excluded_files_not_flagged_as_orphan() {
        let rule = OrphanedFilesRule::new();
        let config = LashConfig::default();

        let mut files = HashMap::new();

        // Create index with only one task file
        files.insert(
            PathBuf::from("lash.index.md"),
            make_index_file("lash.index.md", &["tasks.md"]),
        );
        files.insert(
            PathBuf::from("tasks.md"),
            make_regular_file("tasks.md", "tasks"),
        );
        // Add files that should be excluded
        files.insert(
            PathBuf::from("README.md"),
            make_regular_file("README.md", "readme"),
        );
        files.insert(
            PathBuf::from("CLAUDE.md"),
            make_regular_file("CLAUDE.md", "claude"),
        );
        files.insert(
            PathBuf::from("docs/design.md"),
            make_regular_file("docs/design.md", "design"),
        );

        // README.md should not be flagged (excluded by filename)
        let ctx = LintContext::new(&config, PathBuf::from("README.md"), &files);
        let readme = files.get(&PathBuf::from("README.md")).unwrap();
        let diagnostics = rule.check_file(readme, &ctx);
        assert_eq!(
            diagnostics.len(),
            0,
            "README.md should not be flagged as orphan"
        );

        // CLAUDE.md should not be flagged (excluded by filename)
        let ctx = LintContext::new(&config, PathBuf::from("CLAUDE.md"), &files);
        let claude = files.get(&PathBuf::from("CLAUDE.md")).unwrap();
        let diagnostics = rule.check_file(claude, &ctx);
        assert_eq!(
            diagnostics.len(),
            0,
            "CLAUDE.md should not be flagged as orphan"
        );

        // docs/design.md should not be flagged (excluded by directory)
        let ctx = LintContext::new(&config, PathBuf::from("docs/design.md"), &files);
        let design = files.get(&PathBuf::from("docs/design.md")).unwrap();
        let diagnostics = rule.check_file(design, &ctx);
        assert_eq!(
            diagnostics.len(),
            0,
            "docs/design.md should not be flagged as orphan"
        );
    }
}
