//! Context for lint rule execution
//!
//! The `LintContext` provides shared data and configuration to rules during
//! validation. This includes the project configuration, current file path,
//! and access to other files for cross-file validation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lash_types::{LashConfig, TaskFile};

/// Normalize a path by resolving `.` and `..` components
///
/// This is a simple normalization that works for relative paths.
/// It doesn't resolve symlinks or check if files exist.
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {
                // Skip current directory markers
            }
            comp => {
                components.push(comp);
            }
        }
    }

    components.iter().collect()
}

/// Context provided to lint rules during execution
///
/// The context gives rules access to:
/// - Project configuration (max depth, custom annotations, etc.)
/// - Current file being linted
/// - All files in the project (for cross-file validation)
///
/// This allows rules to make decisions based on project-wide settings
/// and validate cross-file relationships like dependencies.
///
/// # Example
///
/// ```
/// use lash_core::linter::LintContext;
/// use lash_types::LashConfig;
/// use std::collections::HashMap;
/// use std::path::PathBuf;
///
/// let config = LashConfig::default();
/// let files = HashMap::new();
/// let ctx = LintContext::new(
///     &config,
///     PathBuf::from("tasks/api.md"),
///     &files,
/// );
///
/// // Rules can now access configuration
/// let max_depth = ctx.config.max_depth;
/// assert!(max_depth > 0);
/// ```
#[derive(Debug)]
pub struct LintContext<'a> {
    /// Project configuration
    pub config: &'a LashConfig,

    /// Path to the file currently being linted (relative to project root)
    pub file_path: PathBuf,

    /// All parsed files in the project (for cross-file validation)
    ///
    /// This map is populated by the linter when validating multiple files.
    /// For single-file validation, this may be empty.
    pub all_files: &'a HashMap<PathBuf, TaskFile>,
}

impl<'a> LintContext<'a> {
    /// Create a new lint context
    #[must_use]
    pub fn new(
        config: &'a LashConfig,
        file_path: PathBuf,
        all_files: &'a HashMap<PathBuf, TaskFile>,
    ) -> Self {
        Self {
            config,
            file_path,
            all_files,
        }
    }

    /// Get a file from the context by path
    ///
    /// Returns `None` if the file is not in the context (hasn't been parsed yet).
    #[must_use]
    pub fn get_file(&self, path: &Path) -> Option<&TaskFile> {
        self.all_files.get(path)
    }

    /// Check if a custom annotation key is allowed
    ///
    /// Returns `true` if the key is in the built-in list or the custom annotation list.
    #[must_use]
    pub fn is_annotation_allowed(&self, key: &str) -> bool {
        // Built-in annotations
        const BUILTIN: &[&str] = &[
            "id",
            "labels",
            "status",
            "owner",
            "created",
            "estimate",
            "depends-on",
            "agent-note",
        ];

        BUILTIN.contains(&key)
            || self
                .config
                .custom_annotation_keys
                .contains(&key.to_string())
    }

    /// Get the maximum allowed task depth
    #[must_use]
    pub fn max_depth(&self) -> u8 {
        self.config.max_depth
    }

    /// Get the indentation size in spaces
    #[must_use]
    pub fn indent_spaces(&self) -> u8 {
        self.config.indent_spaces
    }

    /// Resolve a relative path from the current file
    ///
    /// Given a relative path in a dependency reference (e.g., "../core/api.md"),
    /// resolve it to an absolute path relative to the project root.
    ///
    /// This performs path normalization to resolve `.` and `..` components.
    #[must_use]
    pub fn resolve_path(&self, relative: &Path) -> PathBuf {
        if relative.is_absolute() {
            return relative.to_path_buf();
        }

        // Get the directory containing the current file
        let current_dir = self.file_path.parent().unwrap_or(Path::new(""));

        // Join and normalize the path
        let joined = current_dir.join(relative);

        // Normalize by removing . and .. components
        normalize_path(&joined)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::LashConfig;

    fn make_config() -> LashConfig {
        LashConfig {
            root_path: PathBuf::from("/project"),
            index_file: "lash.index.md".to_string(),
            max_depth: 3,
            indent_spaces: 2,
            db_path: PathBuf::from(".lash/lash.db"),
            custom_annotation_keys: vec!["custom".to_string()],
        }
    }

    #[test]
    fn test_context_creation() {
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        assert_eq!(ctx.file_path, PathBuf::from("test.md"));
        assert_eq!(ctx.max_depth(), 3);
        assert_eq!(ctx.indent_spaces(), 2);
    }

    #[test]
    fn test_builtin_annotations() {
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        assert!(ctx.is_annotation_allowed("id"));
        assert!(ctx.is_annotation_allowed("labels"));
        assert!(ctx.is_annotation_allowed("status"));
        assert!(ctx.is_annotation_allowed("owner"));
        assert!(ctx.is_annotation_allowed("created"));
        assert!(ctx.is_annotation_allowed("estimate"));
        assert!(ctx.is_annotation_allowed("depends-on"));
        assert!(ctx.is_annotation_allowed("agent-note"));
    }

    #[test]
    fn test_custom_annotations() {
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        assert!(ctx.is_annotation_allowed("custom"));
        assert!(!ctx.is_annotation_allowed("unknown"));
    }

    #[test]
    fn test_resolve_path() {
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("tasks/ui/login.md"), &files);

        // Relative path from tasks/ui/login.md
        let resolved = ctx.resolve_path(Path::new("../core/api.md"));
        assert_eq!(resolved, PathBuf::from("tasks/core/api.md"));

        // Same directory
        let resolved = ctx.resolve_path(Path::new("signup.md"));
        assert_eq!(resolved, PathBuf::from("tasks/ui/signup.md"));

        // Absolute path (unchanged)
        let resolved = ctx.resolve_path(Path::new("/absolute/path.md"));
        assert_eq!(resolved, PathBuf::from("/absolute/path.md"));
    }

    #[test]
    fn test_get_file() {
        use lash_types::{FileMetadata, TaskTree};
        use std::time::SystemTime;

        let config = make_config();
        let mut files = HashMap::new();

        // Create a minimal TaskFile for testing

        let file = TaskFile {
            path: PathBuf::from("test.md"),
            title: "Test".to_string(),
            id: "test".to_string(),
            metadata: FileMetadata::default(),
            tasks: TaskTree::new(),
            hash: "hash".to_string(),
            mtime: SystemTime::now(),
        };

        files.insert(PathBuf::from("test.md"), file);

        let ctx = LintContext::new(&config, PathBuf::from("other.md"), &files);

        assert!(ctx.get_file(Path::new("test.md")).is_some());
        assert!(ctx.get_file(Path::new("missing.md")).is_none());
    }
}
