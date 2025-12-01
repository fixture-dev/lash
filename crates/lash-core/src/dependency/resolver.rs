//! Dependency reference resolution
//!
//! This module provides the dependency resolution engine that transforms unresolved
//! dependency references (from `@depends-on` annotations) into concrete edges in the
//! dependency graph.
//!
//! # Resolution Process
//!
//! The resolver takes a collection of parsed task files and:
//!
//! 1. Iterates through all tasks with `@depends-on` annotations
//! 2. Parses and validates each dependency reference
//! 3. Resolves the reference to a target task ID:
//!    - Path references: Resolve relative/absolute paths to target file
//!    - ID references: Look up tasks by file-id#task-id format
//!    - Within-file references: Resolve task IDs in the same file
//! 4. Creates a `ResolvedDependency` with both source and target full IDs
//! 5. Collects any resolution errors (broken links) for reporting
//!
//! # Supported Reference Formats
//!
//! - **Relative path**: `../core/cli.md#task:parse-args`
//! - **Absolute path**: `core/cli.md#task:parse-args` (relative to project root)
//! - **Within-file ID**: `#task:parse-args` (task in same file)
//! - **File-level**: `../core/cli.md` (depends on all tasks in file)
//! - **File ID format**: `file-id#task-id` (explicit file and task IDs)
//!
//! # Error Handling
//!
//! The resolver collects all errors during resolution rather than failing fast.
//! This allows reporting all broken links in a single pass, which is more useful
//! for users fixing multiple issues.
//!
//! # Example
//!
//! ```
//! use lash_core::dependency::DependencyResolver;
//! use lash_types::{TaskFile, TaskTree, file::FileMetadata};
//! use std::collections::HashMap;
//! use std::path::PathBuf;
//! use std::time::SystemTime;
//!
//! # let mut files = HashMap::new();
//! # let mut tasks = TaskTree::new();
//! # files.insert(
//! #     PathBuf::from("test.md"),
//! #     TaskFile {
//! #         path: PathBuf::from("test.md"),
//! #         title: "Test".to_string(),
//! #         id: "test".to_string(),
//! #         metadata: FileMetadata::default(),
//! #         tasks,
//! #         hash: "abc123".to_string(),
//! #         mtime: SystemTime::now(),
//! #     }
//! # );
//! let project_root = PathBuf::from("/project");
//! let mut resolver = DependencyResolver::new(&files, project_root);
//!
//! let result = resolver.resolve_dependencies();
//! println!("Resolved {} dependencies", result.resolved.len());
//! println!("Found {} errors", result.errors.len());
//! ```

use lash_types::dependency::{make_full_id, parse_full_id, DependencyKind, DependencyRef};
use lash_types::task::Task;
use lash_types::TaskFile;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A successfully resolved dependency
///
/// Contains the full IDs of both source and target tasks, the dependency kind,
/// and optional source location information for error reporting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDependency {
    /// Source task full ID (file-id#task-id)
    pub from_full_id: String,

    /// Target task full ID (file-id#task-id)
    pub to_full_id: String,

    /// Type of dependency (`explicit_id`, `explicit_path`, etc.)
    pub kind: DependencyKind,

    /// Source location for error reporting (file path and task ID)
    pub source_location: Option<String>,
}

impl ResolvedDependency {
    /// Create a new resolved dependency
    #[must_use]
    pub fn new(
        from_full_id: String,
        to_full_id: String,
        kind: DependencyKind,
        source_location: Option<String>,
    ) -> Self {
        Self {
            from_full_id,
            to_full_id,
            kind,
            source_location,
        }
    }
}

/// Error encountered during dependency resolution
///
/// Tracks detailed information about why a dependency reference could not
/// be resolved, including source location for actionable error messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionError {
    /// Source file path
    pub source_file: PathBuf,

    /// Source task ID
    pub source_task_id: String,

    /// The dependency reference that failed
    pub dependency_ref: String,

    /// Specific error kind
    pub error_kind: ResolutionErrorKind,
}

impl ResolutionError {
    /// Create a new resolution error
    #[must_use]
    pub fn new(
        source_file: PathBuf,
        source_task_id: String,
        dependency_ref: String,
        error_kind: ResolutionErrorKind,
    ) -> Self {
        Self {
            source_file,
            source_task_id,
            dependency_ref,
            error_kind,
        }
    }

    /// Convert to a user-facing error message
    #[must_use]
    pub fn to_error_message(&self) -> String {
        let location = format!("{}#{}", self.source_file.display(), self.source_task_id);
        match &self.error_kind {
            ResolutionErrorKind::FileNotFound { path } => {
                format!(
                    "In {}: dependency reference '{}' points to non-existent file '{}'",
                    location,
                    self.dependency_ref,
                    path.display()
                )
            }
            ResolutionErrorKind::TaskNotFound { file_path, task_id } => {
                format!(
                    "In {}: dependency reference '{}' points to non-existent task '{}' in file '{}'",
                    location,
                    self.dependency_ref,
                    task_id,
                    file_path.display()
                )
            }
            ResolutionErrorKind::InvalidReference { reason } => {
                format!(
                    "In {}: invalid dependency reference '{}': {}",
                    location, self.dependency_ref, reason
                )
            }
            ResolutionErrorKind::DirectoryNotFound { path } => {
                format!(
                    "In {}: dependency reference '{}' points to directory '{}' which contains no task files",
                    location,
                    self.dependency_ref,
                    path.display()
                )
            }
        }
    }
}

/// Specific kind of resolution error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionErrorKind {
    /// Referenced file does not exist
    FileNotFound { path: PathBuf },

    /// Referenced task does not exist in the target file
    TaskNotFound { file_path: PathBuf, task_id: String },

    /// Reference has invalid syntax or format
    InvalidReference { reason: String },

    /// Referenced directory does not exist or contains no task files
    DirectoryNotFound { path: PathBuf },
}

/// Result of dependency resolution
///
/// Contains both successfully resolved dependencies and any errors encountered.
/// The resolver does not fail fast - it attempts to resolve all dependencies
/// and collects errors for comprehensive reporting.
#[derive(Debug, Clone)]
pub struct ResolverResult {
    /// Successfully resolved dependencies
    pub resolved: Vec<ResolvedDependency>,

    /// Resolution errors (broken links, invalid references)
    pub errors: Vec<ResolutionError>,
}

impl ResolverResult {
    /// Create a new empty result
    #[must_use]
    pub fn new() -> Self {
        Self {
            resolved: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Check if there are any resolution errors
    #[must_use]
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get total count of resolved dependencies
    #[must_use]
    pub fn resolved_count(&self) -> usize {
        self.resolved.len()
    }

    /// Get total count of errors
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }
}

impl Default for ResolverResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Dependency reference resolver
///
/// Resolves dependency references from `@depends-on` annotations to concrete
/// task IDs. Handles path resolution, ID lookups, and error collection.
///
/// The resolver operates on a collection of parsed `TaskFile` structures and
/// produces a set of resolved dependencies suitable for building a dependency graph.
pub struct DependencyResolver<'a> {
    /// Map from file path to `TaskFile` for lookups
    files: &'a HashMap<PathBuf, TaskFile>,

    /// Map from file ID to file path for reverse lookups
    file_id_to_path: HashMap<String, PathBuf>,

    /// Project root directory for absolute path resolution
    project_root: PathBuf,

    /// Accumulated resolution errors
    errors: Vec<ResolutionError>,
}

impl<'a> DependencyResolver<'a> {
    /// Create a new dependency resolver
    ///
    /// # Arguments
    ///
    /// * `files` - Collection of parsed task files indexed by path
    /// * `project_root` - Project root directory for resolving absolute paths
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_core::dependency::DependencyResolver;
    /// use lash_types::{TaskFile, TaskTree, file::FileMetadata};
    /// use std::collections::HashMap;
    /// use std::path::PathBuf;
    /// use std::time::SystemTime;
    ///
    /// # let mut files = HashMap::new();
    /// # let mut tasks = TaskTree::new();
    /// # files.insert(
    /// #     PathBuf::from("test.md"),
    /// #     TaskFile {
    /// #         path: PathBuf::from("test.md"),
    /// #         title: "Test".to_string(),
    /// #         id: "test".to_string(),
    /// #         metadata: FileMetadata::default(),
    /// #         tasks,
    /// #         hash: "abc123".to_string(),
    /// #         mtime: SystemTime::now(),
    /// #     }
    /// # );
    /// let project_root = PathBuf::from("/project");
    /// let resolver = DependencyResolver::new(&files, project_root);
    /// ```
    #[must_use]
    pub fn new(files: &'a HashMap<PathBuf, TaskFile>, project_root: PathBuf) -> Self {
        // Build file ID to path index
        let file_id_to_path: HashMap<String, PathBuf> = files
            .iter()
            .map(|(path, file)| (file.id.clone(), path.clone()))
            .collect();

        Self {
            files,
            file_id_to_path,
            project_root,
            errors: Vec::new(),
        }
    }

    /// Resolve all dependencies in the file collection
    ///
    /// Iterates through all files and tasks, resolving `@depends-on` annotations
    /// to concrete dependency edges. Collects errors rather than failing fast.
    ///
    /// # Returns
    ///
    /// A `ResolverResult` containing both resolved dependencies and any errors.
    ///
    /// # Examples
    ///
    /// ```
    /// # use lash_core::dependency::DependencyResolver;
    /// # use lash_types::{TaskFile, TaskTree, file::FileMetadata};
    /// # use std::collections::HashMap;
    /// # use std::path::PathBuf;
    /// # use std::time::SystemTime;
    /// # let mut files = HashMap::new();
    /// # let mut tasks = TaskTree::new();
    /// # files.insert(
    /// #     PathBuf::from("test.md"),
    /// #     TaskFile {
    /// #         path: PathBuf::from("test.md"),
    /// #         title: "Test".to_string(),
    /// #         id: "test".to_string(),
    /// #         metadata: FileMetadata::default(),
    /// #         tasks,
    /// #         hash: "abc123".to_string(),
    /// #         mtime: SystemTime::now(),
    /// #     }
    /// # );
    /// # let project_root = PathBuf::from("/project");
    /// let mut resolver = DependencyResolver::new(&files, project_root);
    /// let result = resolver.resolve_dependencies();
    ///
    /// for dep in &result.resolved {
    ///     println!("{} depends on {}", dep.from_full_id, dep.to_full_id);
    /// }
    ///
    /// for error in &result.errors {
    ///     eprintln!("{}", error.to_error_message());
    /// }
    /// ```
    pub fn resolve_dependencies(&mut self) -> ResolverResult {
        let mut resolved = Vec::new();

        // Iterate through all files and tasks
        for (file_path, task_file) in self.files {
            for task in task_file.tasks.tasks() {
                // Resolve explicit @depends-on annotations
                for dep_ref in &task.metadata.depends_on {
                    match self.resolve_reference(file_path, &task_file.id, task, dep_ref) {
                        Ok(resolved_deps) => resolved.extend(resolved_deps),
                        Err(error) => self.errors.push(error),
                    }
                }
            }
        }

        ResolverResult {
            resolved,
            errors: std::mem::take(&mut self.errors),
        }
    }

    /// Resolve a single dependency reference
    ///
    /// Handles all supported reference formats and returns either resolved
    /// dependencies (may be multiple for file/directory refs) or a detailed error.
    fn resolve_reference(
        &self,
        source_file: &Path,
        source_file_id: &str,
        source_task: &Task,
        dep_ref: &DependencyRef,
    ) -> std::result::Result<Vec<ResolvedDependency>, ResolutionError> {
        let source_full_id = make_full_id(source_file_id, &source_task.id);
        let source_location = Some(format!("{}#{}", source_file.display(), source_task.id));

        // Handle different dependency kinds
        match &dep_ref.kind {
            DependencyKind::ExplicitPath => self.resolve_path_reference(
                source_file,
                &source_full_id,
                &source_task.id,
                dep_ref,
                source_location,
            ),
            DependencyKind::ExplicitId => self.resolve_id_reference(
                source_file,
                source_file_id,
                &source_full_id,
                &source_task.id,
                dep_ref,
                source_location,
            ),
            DependencyKind::Hierarchy => {
                // Hierarchy dependencies are handled separately (during indexing)
                // We don't process them here
                Err(ResolutionError::new(
                    source_file.to_path_buf(),
                    source_task.id.clone(),
                    dep_ref.target.clone(),
                    ResolutionErrorKind::InvalidReference {
                        reason: "Hierarchy dependencies should not appear in @depends-on"
                            .to_string(),
                    },
                ))
            }
            DependencyKind::Directory => self.resolve_directory_reference(
                source_file,
                &source_full_id,
                &source_task.id,
                dep_ref,
                source_location,
            ),
        }
    }

    /// Resolve a path-based dependency reference
    ///
    /// Handles references like:
    /// - `../core/cli.md#task:parse-args` (with task ID)
    /// - `../core/cli.md` (file-level, depends on all top-level tasks)
    fn resolve_path_reference(
        &self,
        source_file: &Path,
        source_full_id: &str,
        source_task_id: &str,
        dep_ref: &DependencyRef,
        source_location: Option<String>,
    ) -> std::result::Result<Vec<ResolvedDependency>, ResolutionError> {
        // Split reference into path and optional task fragment
        let (path_str, task_fragment) = if let Some(pos) = dep_ref.target.find('#') {
            let (path, fragment) = dep_ref.target.split_at(pos);
            (path, Some(&fragment[1..])) // Skip the '#'
        } else {
            (dep_ref.target.as_str(), None)
        };

        // Resolve the file path
        let target_path = self.resolve_path(source_file, path_str);

        // Look up the target file
        let target_file = self.files.get(&target_path).ok_or_else(|| {
            ResolutionError::new(
                source_file.to_path_buf(),
                source_task_id.to_string(),
                dep_ref.target.clone(),
                ResolutionErrorKind::FileNotFound {
                    path: target_path.clone(),
                },
            )
        })?;

        // If there's a task fragment, resolve to a specific task
        if let Some(fragment) = task_fragment {
            // Strip "task:" prefix if present
            let task_id = fragment.strip_prefix("task:").unwrap_or(fragment);

            // Look up the task in the target file
            let _target_task = target_file.tasks.get_task(task_id).ok_or_else(|| {
                ResolutionError::new(
                    source_file.to_path_buf(),
                    source_task_id.to_string(),
                    dep_ref.target.clone(),
                    ResolutionErrorKind::TaskNotFound {
                        file_path: target_path.clone(),
                        task_id: task_id.to_string(),
                    },
                )
            })?;

            let target_full_id = make_full_id(&target_file.id, task_id);

            Ok(vec![ResolvedDependency::new(
                source_full_id.to_string(),
                target_full_id,
                dep_ref.kind.clone(),
                source_location,
            )])
        } else {
            // File-level dependency - depends on all top-level tasks in the file
            Ok(self.create_file_level_dependencies(
                source_full_id,
                target_file,
                &dep_ref.kind,
                source_location,
            ))
        }
    }

    /// Resolve an ID-based dependency reference
    ///
    /// Handles references like:
    /// - `file-id#task-id` (explicit file and task IDs)
    /// - `#task-id` (within-file reference)
    /// - `file-id` (file-level, depends on all top-level tasks)
    #[allow(clippy::too_many_lines)]
    fn resolve_id_reference(
        &self,
        source_file: &Path,
        source_file_id: &str,
        source_full_id: &str,
        source_task_id: &str,
        dep_ref: &DependencyRef,
        source_location: Option<String>,
    ) -> std::result::Result<Vec<ResolvedDependency>, ResolutionError> {
        // Check if it's a within-file reference (starts with #)
        if dep_ref.target.starts_with('#') {
            // Within-file reference
            let task_id = dep_ref
                .target
                .strip_prefix('#')
                .and_then(|s| s.strip_prefix("task:"))
                .unwrap_or_else(|| dep_ref.target.strip_prefix('#').unwrap_or(&dep_ref.target));

            // Look up in the same file
            let source_file_obj = self.files.get(source_file).ok_or_else(|| {
                ResolutionError::new(
                    source_file.to_path_buf(),
                    source_task_id.to_string(),
                    dep_ref.target.clone(),
                    ResolutionErrorKind::InvalidReference {
                        reason: "Source file not found in collection".to_string(),
                    },
                )
            })?;

            let _target_task = source_file_obj.tasks.get_task(task_id).ok_or_else(|| {
                ResolutionError::new(
                    source_file.to_path_buf(),
                    source_task_id.to_string(),
                    dep_ref.target.clone(),
                    ResolutionErrorKind::TaskNotFound {
                        file_path: source_file.to_path_buf(),
                        task_id: task_id.to_string(),
                    },
                )
            })?;

            let target_full_id = make_full_id(source_file_id, task_id);

            Ok(vec![ResolvedDependency::new(
                source_full_id.to_string(),
                target_full_id,
                dep_ref.kind.clone(),
                source_location,
            )])
        } else if dep_ref.target.contains('#') {
            // Full ID reference: file-id#task-id
            let (file_id, task_id) = parse_full_id(&dep_ref.target).map_err(|_| {
                ResolutionError::new(
                    source_file.to_path_buf(),
                    source_task_id.to_string(),
                    dep_ref.target.clone(),
                    ResolutionErrorKind::InvalidReference {
                        reason: "Invalid full ID format".to_string(),
                    },
                )
            })?;

            // Look up file by file ID
            let target_path = self.file_id_to_path.get(&file_id).ok_or_else(|| {
                ResolutionError::new(
                    source_file.to_path_buf(),
                    source_task_id.to_string(),
                    dep_ref.target.clone(),
                    ResolutionErrorKind::InvalidReference {
                        reason: format!("Unknown file ID: '{file_id}'"),
                    },
                )
            })?;

            let target_file = self.files.get(target_path).ok_or_else(|| {
                ResolutionError::new(
                    source_file.to_path_buf(),
                    source_task_id.to_string(),
                    dep_ref.target.clone(),
                    ResolutionErrorKind::FileNotFound {
                        path: target_path.clone(),
                    },
                )
            })?;

            // Strip "task:" prefix if present
            let task_id_clean = task_id.strip_prefix("task:").unwrap_or(&task_id);

            // Look up task in target file
            let _target_task = target_file.tasks.get_task(task_id_clean).ok_or_else(|| {
                ResolutionError::new(
                    source_file.to_path_buf(),
                    source_task_id.to_string(),
                    dep_ref.target.clone(),
                    ResolutionErrorKind::TaskNotFound {
                        file_path: target_path.clone(),
                        task_id: task_id_clean.to_string(),
                    },
                )
            })?;

            Ok(vec![ResolvedDependency::new(
                source_full_id.to_string(),
                dep_ref.target.clone(),
                dep_ref.kind.clone(),
                source_location,
            )])
        } else {
            // Bare file ID - file-level dependency on all top-level tasks
            let target_path = self.file_id_to_path.get(&dep_ref.target).ok_or_else(|| {
                ResolutionError::new(
                    source_file.to_path_buf(),
                    source_task_id.to_string(),
                    dep_ref.target.clone(),
                    ResolutionErrorKind::InvalidReference {
                        reason: format!("Unknown file ID: '{}'", dep_ref.target),
                    },
                )
            })?;

            let target_file = self.files.get(target_path).ok_or_else(|| {
                ResolutionError::new(
                    source_file.to_path_buf(),
                    source_task_id.to_string(),
                    dep_ref.target.clone(),
                    ResolutionErrorKind::FileNotFound {
                        path: target_path.clone(),
                    },
                )
            })?;

            Ok(self.create_file_level_dependencies(
                source_full_id,
                target_file,
                &dep_ref.kind,
                source_location,
            ))
        }
    }

    /// Resolve a directory-level dependency reference
    ///
    /// Creates dependencies from the source task to all top-level tasks in files
    /// within the referenced directory. Directory references must end with `/`.
    ///
    /// # Arguments
    ///
    /// * `source_file` - Path of the file containing the dependency reference
    /// * `source_full_id` - Full ID of the source task (file-id#task-id)
    /// * `source_task_id` - ID of the source task (for error reporting)
    /// * `dep_ref` - The directory dependency reference (e.g., "core/")
    /// * `source_location` - Location string for error messages (passed by value as it's cloned in loop)
    #[allow(clippy::needless_pass_by_value)]
    fn resolve_directory_reference(
        &self,
        source_file: &Path,
        source_full_id: &str,
        source_task_id: &str,
        dep_ref: &DependencyRef,
        source_location: Option<String>,
    ) -> std::result::Result<Vec<ResolvedDependency>, ResolutionError> {
        // Remove trailing / from directory path
        let dir_path_str = dep_ref.target.trim_end_matches('/');

        // Resolve the directory path (relative to source file or project root)
        let target_dir = self.resolve_path(source_file, dir_path_str);

        // Find all files in this directory (not recursive - only immediate children)
        let files_in_dir: Vec<_> = self
            .files
            .iter()
            .filter(|(path, _)| {
                // Check if file is directly in target_dir (not in subdirectory)
                if let Some(parent) = path.parent() {
                    parent == target_dir
                } else {
                    false
                }
            })
            .collect();

        if files_in_dir.is_empty() {
            return Err(ResolutionError::new(
                source_file.to_path_buf(),
                source_task_id.to_string(),
                dep_ref.target.clone(),
                ResolutionErrorKind::DirectoryNotFound { path: target_dir },
            ));
        }

        // Create dependencies to all top-level tasks in all files in the directory
        let mut resolved = Vec::new();
        for (_path, target_file) in files_in_dir {
            let deps = self.create_file_level_dependencies(
                source_full_id,
                target_file,
                &dep_ref.kind,
                source_location.clone(),
            );
            resolved.extend(deps);
        }

        Ok(resolved)
    }

    /// Create dependencies from a source task to all top-level tasks in a target file
    ///
    /// This is used for both file-level dependencies (e.g., `../core/api.md`) and
    /// directory-level dependencies (creates deps to top-level tasks in each file).
    ///
    /// Only top-level tasks (depth == 0) are included as targets to avoid bloating
    /// the dependency graph.
    #[allow(clippy::unused_self, clippy::needless_pass_by_value)]
    fn create_file_level_dependencies(
        &self,
        source_full_id: &str,
        target_file: &TaskFile,
        kind: &DependencyKind,
        source_location: Option<String>,
    ) -> Vec<ResolvedDependency> {
        target_file
            .tasks
            .tasks()
            .iter()
            .filter(|task| task.depth == 0) // Only top-level tasks
            .map(|task| {
                let target_full_id = make_full_id(&target_file.id, &task.id);
                ResolvedDependency::new(
                    source_full_id.to_string(),
                    target_full_id,
                    kind.clone(),
                    source_location.clone(),
                )
            })
            .collect()
    }

    /// Resolve a path relative to the source file or project root
    ///
    /// Handles both relative paths (starting with `..` or `.`) and absolute
    /// paths (relative to project root).
    fn resolve_path(&self, source_file: &Path, reference_path: &str) -> PathBuf {
        let ref_path = Path::new(reference_path);

        // If the path is relative (starts with . or ..), resolve relative to source file
        if reference_path.starts_with("..") || reference_path.starts_with("./") {
            // Get the directory containing the source file
            let source_dir = source_file.parent().unwrap_or_else(|| Path::new(""));

            // Join and normalize
            let joined = source_dir.join(ref_path);

            // Normalize the path (resolve . and ..)
            normalize_path(&joined)
        } else {
            // Absolute path (relative to project root)
            normalize_path(&self.project_root.join(ref_path))
        }
    }
}

/// Normalize a path by resolving `.` and `..` components
///
/// This is a simplified path normalization that doesn't require the path to exist
/// (unlike `std::fs::canonicalize`).
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                // Pop the last component if possible
                if !components.is_empty() {
                    components.pop();
                }
            }
            std::path::Component::CurDir => {
                // Skip current directory markers
            }
            other => {
                components.push(other);
            }
        }
    }

    components.iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::dependency::parse_dependency_ref;
    use lash_types::file::FileMetadata;
    use lash_types::task::{TaskBuilder, TaskTree};
    use std::time::SystemTime;

    fn create_test_file(path: &str, id: &str, title: &str, tasks: TaskTree) -> (PathBuf, TaskFile) {
        let path_buf = PathBuf::from(path);
        let file = TaskFile {
            path: path_buf.clone(),
            title: title.to_string(),
            id: id.to_string(),
            metadata: FileMetadata::default(),
            tasks,
            hash: "test-hash".to_string(),
            mtime: SystemTime::now(),
        };
        (path_buf, file)
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path(Path::new("a/b/../c")), PathBuf::from("a/c"));
        assert_eq!(
            normalize_path(Path::new("a/./b/./c")),
            PathBuf::from("a/b/c")
        );
        assert_eq!(normalize_path(Path::new("a/b/../../c")), PathBuf::from("c"));
    }

    #[test]
    fn test_resolve_within_file_reference() {
        let mut tasks = TaskTree::new();
        tasks
            .add_task(TaskBuilder::new("Task 1").id("task-1").build().unwrap())
            .unwrap();

        tasks
            .add_task(TaskBuilder::new("Task 2").id("task-2").build().unwrap())
            .unwrap();

        // Manually update the second task's metadata
        let (path, mut file) = create_test_file("test.md", "test", "Test File", tasks);

        if let Some(task2) = file.tasks.get_task_mut("task-2") {
            task2
                .metadata
                .depends_on
                .push(parse_dependency_ref("#task-1").unwrap());
        }

        let mut files = HashMap::new();
        files.insert(path.clone(), file);

        let mut resolver = DependencyResolver::new(&files, PathBuf::from("/project"));
        let result = resolver.resolve_dependencies();

        assert_eq!(result.resolved.len(), 1);
        assert_eq!(result.errors.len(), 0);

        let dep = &result.resolved[0];
        assert_eq!(dep.from_full_id, "test#task-2");
        assert_eq!(dep.to_full_id, "test#task-1");
    }

    #[test]
    fn test_resolve_missing_task() {
        let mut tasks = TaskTree::new();
        tasks
            .add_task(TaskBuilder::new("Task 1").id("task-1").build().unwrap())
            .unwrap();

        let (path, mut file) = create_test_file("test.md", "test", "Test File", tasks);

        if let Some(task1) = file.tasks.get_task_mut("task-1") {
            task1
                .metadata
                .depends_on
                .push(parse_dependency_ref("#missing-task").unwrap());
        }

        let mut files = HashMap::new();
        files.insert(path.clone(), file);

        let mut resolver = DependencyResolver::new(&files, PathBuf::from("/project"));
        let result = resolver.resolve_dependencies();

        assert_eq!(result.resolved.len(), 0);
        assert_eq!(result.errors.len(), 1);

        let error = &result.errors[0];
        assert!(matches!(
            error.error_kind,
            ResolutionErrorKind::TaskNotFound { .. }
        ));
    }

    #[test]
    fn test_resolve_cross_file_id_reference() {
        // Create two files
        let mut tasks1 = TaskTree::new();
        tasks1
            .add_task(TaskBuilder::new("Task A").id("task-a").build().unwrap())
            .unwrap();

        let mut tasks2 = TaskTree::new();
        tasks2
            .add_task(TaskBuilder::new("Task B").id("task-b").build().unwrap())
            .unwrap();

        let (path1, task_file1) = create_test_file("file1.md", "file1", "File 1", tasks1);
        let (path2, mut task_file2) = create_test_file("file2.md", "file2", "File 2", tasks2);

        if let Some(task_b) = task_file2.tasks.get_task_mut("task-b") {
            task_b
                .metadata
                .depends_on
                .push(parse_dependency_ref("file1#task-a").unwrap());
        }

        let mut files = HashMap::new();
        files.insert(path1, task_file1);
        files.insert(path2, task_file2);

        let mut resolver = DependencyResolver::new(&files, PathBuf::from("/project"));
        let result = resolver.resolve_dependencies();

        assert_eq!(result.resolved.len(), 1);
        assert_eq!(result.errors.len(), 0);

        let dep = &result.resolved[0];
        assert_eq!(dep.from_full_id, "file2#task-b");
        assert_eq!(dep.to_full_id, "file1#task-a");
    }

    #[test]
    fn test_resolve_path_reference_relative() {
        let mut tasks1 = TaskTree::new();
        tasks1
            .add_task(TaskBuilder::new("Task A").id("task-a").build().unwrap())
            .unwrap();

        let mut tasks2 = TaskTree::new();
        tasks2
            .add_task(TaskBuilder::new("Task B").id("task-b").build().unwrap())
            .unwrap();

        let (path1, task_file1) = create_test_file("dir1/file1.md", "dir1.file1", "File 1", tasks1);
        let (path2, mut task_file2) =
            create_test_file("dir2/file2.md", "dir2.file2", "File 2", tasks2);

        if let Some(task_b) = task_file2.tasks.get_task_mut("task-b") {
            task_b
                .metadata
                .depends_on
                .push(parse_dependency_ref("../dir1/file1.md#task-a").unwrap());
        }

        let mut files = HashMap::new();
        files.insert(path1.clone(), task_file1);
        files.insert(path2.clone(), task_file2);

        let mut resolver = DependencyResolver::new(&files, PathBuf::from("/project"));
        let result = resolver.resolve_dependencies();

        // Debug output
        if !result.errors.is_empty() {
            eprintln!("Errors:");
            for error in &result.errors {
                eprintln!("  {}", error.to_error_message());
            }
        }

        assert_eq!(result.errors.len(), 0, "Expected no errors");
        assert_eq!(result.resolved.len(), 1, "Expected 1 resolved dependency");

        let dep = &result.resolved[0];
        assert_eq!(dep.from_full_id, "dir2.file2#task-b");
        assert_eq!(dep.to_full_id, "dir1.file1#task-a");
    }

    #[test]
    fn test_resolve_path_reference_missing_file() {
        let mut tasks = TaskTree::new();
        tasks
            .add_task(TaskBuilder::new("Task B").id("task-b").build().unwrap())
            .unwrap();

        let (path, mut file) = create_test_file("dir/file.md", "dir.file", "File", tasks);

        if let Some(task_b) = file.tasks.get_task_mut("task-b") {
            task_b
                .metadata
                .depends_on
                .push(parse_dependency_ref("../missing/file.md#task-a").unwrap());
        }

        let mut files = HashMap::new();
        files.insert(path, file);

        let mut resolver = DependencyResolver::new(&files, PathBuf::from("/project"));
        let result = resolver.resolve_dependencies();

        assert_eq!(result.resolved.len(), 0);
        assert_eq!(result.errors.len(), 1);

        let error = &result.errors[0];
        assert!(matches!(
            error.error_kind,
            ResolutionErrorKind::FileNotFound { .. }
        ));
    }

    #[test]
    fn test_resolver_result_helpers() {
        let mut result = ResolverResult::new();
        assert!(!result.has_errors());
        assert_eq!(result.resolved_count(), 0);
        assert_eq!(result.error_count(), 0);

        result.resolved.push(ResolvedDependency::new(
            "a#1".to_string(),
            "b#2".to_string(),
            DependencyKind::ExplicitId,
            None,
        ));

        result.errors.push(ResolutionError::new(
            PathBuf::from("test.md"),
            "task-1".to_string(),
            "broken-ref".to_string(),
            ResolutionErrorKind::FileNotFound {
                path: PathBuf::from("missing.md"),
            },
        ));

        assert!(result.has_errors());
        assert_eq!(result.resolved_count(), 1);
        assert_eq!(result.error_count(), 1);
    }

    #[test]
    fn test_resolution_error_message() {
        let error = ResolutionError::new(
            PathBuf::from("source.md"),
            "task-1".to_string(),
            "../missing.md#task-a".to_string(),
            ResolutionErrorKind::FileNotFound {
                path: PathBuf::from("missing.md"),
            },
        );

        let msg = error.to_error_message();
        assert!(msg.contains("source.md#task-1"));
        assert!(msg.contains("../missing.md#task-a"));
        assert!(msg.contains("missing.md"));
    }

    // ==================== File-Level Dependency Tests ====================

    #[test]
    fn test_file_level_dependency_path_reference() {
        // Create a target file with multiple top-level tasks
        let mut target_tasks = TaskTree::new();
        target_tasks
            .add_task(TaskBuilder::new("Top Task A").id("top-a").build().unwrap())
            .unwrap();
        target_tasks
            .add_task(TaskBuilder::new("Top Task B").id("top-b").build().unwrap())
            .unwrap();
        // Add a nested task (depth > 0) - should NOT be included
        target_tasks
            .add_task(
                TaskBuilder::new("Nested Task")
                    .id("nested-task")
                    .depth(1)
                    .parent("top-a".to_string())
                    .build()
                    .unwrap(),
            )
            .unwrap();

        // Create a source file with a task that depends on the entire target file
        let mut source_tasks = TaskTree::new();
        source_tasks
            .add_task(
                TaskBuilder::new("Source Task")
                    .id("source")
                    .build()
                    .unwrap(),
            )
            .unwrap();

        let (target_path, target_file) =
            create_test_file("core/api.md", "core.api", "Core API", target_tasks);
        let (source_path, mut source_file) = create_test_file(
            "features/feature.md",
            "features.feature",
            "Feature",
            source_tasks,
        );

        // Add file-level dependency (no #task fragment)
        if let Some(source_task) = source_file.tasks.get_task_mut("source") {
            source_task
                .metadata
                .depends_on
                .push(parse_dependency_ref("../core/api.md").unwrap());
        }

        let mut files = HashMap::new();
        files.insert(target_path, target_file);
        files.insert(source_path, source_file);

        let mut resolver = DependencyResolver::new(&files, PathBuf::from("/project"));
        let result = resolver.resolve_dependencies();

        // Should have 2 dependencies (one for each top-level task, NOT the nested task)
        assert_eq!(result.errors.len(), 0, "Expected no errors");
        assert_eq!(
            result.resolved.len(),
            2,
            "Expected 2 dependencies (top-level tasks only)"
        );

        // Verify the dependencies are to top-level tasks
        let target_ids: Vec<_> = result.resolved.iter().map(|d| &d.to_full_id).collect();
        assert!(target_ids.contains(&&"core.api#top-a".to_string()));
        assert!(target_ids.contains(&&"core.api#top-b".to_string()));
        // nested-task should NOT be included
        assert!(!target_ids.contains(&&"core.api#nested-task".to_string()));
    }

    #[test]
    fn test_file_level_dependency_bare_file_id() {
        // Test bare file ID (no # or path) as file-level dependency
        let mut target_tasks = TaskTree::new();
        target_tasks
            .add_task(TaskBuilder::new("Task 1").id("task-1").build().unwrap())
            .unwrap();
        target_tasks
            .add_task(TaskBuilder::new("Task 2").id("task-2").build().unwrap())
            .unwrap();

        let mut source_tasks = TaskTree::new();
        source_tasks
            .add_task(TaskBuilder::new("Source").id("source").build().unwrap())
            .unwrap();

        let (target_path, target_file) =
            create_test_file("target.md", "target-file", "Target", target_tasks);
        let (source_path, mut source_file) =
            create_test_file("source.md", "source-file", "Source", source_tasks);

        // Add bare file ID dependency
        if let Some(source_task) = source_file.tasks.get_task_mut("source") {
            source_task.metadata.depends_on.push(DependencyRef::new(
                "target-file".to_string(),
                DependencyKind::ExplicitId,
            ));
        }

        let mut files = HashMap::new();
        files.insert(target_path, target_file);
        files.insert(source_path, source_file);

        let mut resolver = DependencyResolver::new(&files, PathBuf::from("/project"));
        let result = resolver.resolve_dependencies();

        assert_eq!(result.errors.len(), 0);
        assert_eq!(result.resolved.len(), 2);

        let target_ids: Vec<_> = result.resolved.iter().map(|d| &d.to_full_id).collect();
        assert!(target_ids.contains(&&"target-file#task-1".to_string()));
        assert!(target_ids.contains(&&"target-file#task-2".to_string()));
    }

    // ==================== Directory-Level Dependency Tests ====================

    #[test]
    fn test_directory_dependency_basic() {
        // Create files in a "core" directory
        let mut core_api_tasks = TaskTree::new();
        core_api_tasks
            .add_task(TaskBuilder::new("API Task").id("api-task").build().unwrap())
            .unwrap();

        let mut core_db_tasks = TaskTree::new();
        core_db_tasks
            .add_task(TaskBuilder::new("DB Task").id("db-task").build().unwrap())
            .unwrap();

        // Create source file that depends on the entire "core" directory
        let mut source_tasks = TaskTree::new();
        source_tasks
            .add_task(TaskBuilder::new("Feature").id("feature").build().unwrap())
            .unwrap();

        let (api_path, api_file) =
            create_test_file("core/api.md", "core.api", "Core API", core_api_tasks);
        let (db_path, db_file) =
            create_test_file("core/db.md", "core.db", "Core DB", core_db_tasks);
        let (source_path, mut source_file) = create_test_file(
            "features/feature.md",
            "features.feature",
            "Feature",
            source_tasks,
        );

        // Add directory dependency
        if let Some(feature_task) = source_file.tasks.get_task_mut("feature") {
            feature_task
                .metadata
                .depends_on
                .push(parse_dependency_ref("../core/").unwrap());
        }

        let mut files = HashMap::new();
        files.insert(api_path, api_file);
        files.insert(db_path, db_file);
        files.insert(source_path, source_file);

        let mut resolver = DependencyResolver::new(&files, PathBuf::from("/project"));
        let result = resolver.resolve_dependencies();

        // Debug output
        if !result.errors.is_empty() {
            for error in &result.errors {
                eprintln!("Error: {}", error.to_error_message());
            }
        }

        assert_eq!(result.errors.len(), 0, "Expected no errors");
        assert_eq!(
            result.resolved.len(),
            2,
            "Expected 2 dependencies (one for each file in core/)"
        );

        let target_ids: Vec<_> = result.resolved.iter().map(|d| &d.to_full_id).collect();
        assert!(target_ids.contains(&&"core.api#api-task".to_string()));
        assert!(target_ids.contains(&&"core.db#db-task".to_string()));
    }

    #[test]
    fn test_directory_dependency_top_level_only() {
        // Verify that directory dependencies only include top-level tasks
        let mut core_tasks = TaskTree::new();
        core_tasks
            .add_task(TaskBuilder::new("Top Task").id("top").build().unwrap())
            .unwrap();
        core_tasks
            .add_task(
                TaskBuilder::new("Nested Task")
                    .id("nested")
                    .depth(1)
                    .parent("top".to_string())
                    .build()
                    .unwrap(),
            )
            .unwrap();

        let mut source_tasks = TaskTree::new();
        source_tasks
            .add_task(TaskBuilder::new("Feature").id("feature").build().unwrap())
            .unwrap();

        let (core_path, core_file) =
            create_test_file("core/tasks.md", "core.tasks", "Core Tasks", core_tasks);
        let (source_path, mut source_file) = create_test_file(
            "features/feature.md",
            "features.feature",
            "Feature",
            source_tasks,
        );

        if let Some(feature_task) = source_file.tasks.get_task_mut("feature") {
            feature_task
                .metadata
                .depends_on
                .push(parse_dependency_ref("../core/").unwrap());
        }

        let mut files = HashMap::new();
        files.insert(core_path, core_file);
        files.insert(source_path, source_file);

        let mut resolver = DependencyResolver::new(&files, PathBuf::from("/project"));
        let result = resolver.resolve_dependencies();

        assert_eq!(result.errors.len(), 0);
        // Only 1 dependency (the top-level task, not the nested one)
        assert_eq!(
            result.resolved.len(),
            1,
            "Should only depend on top-level tasks"
        );
        assert_eq!(result.resolved[0].to_full_id, "core.tasks#top");
    }

    #[test]
    fn test_directory_dependency_not_recursive() {
        // Verify that directory dependencies only include immediate children, not nested dirs
        let mut immediate_tasks = TaskTree::new();
        immediate_tasks
            .add_task(
                TaskBuilder::new("Immediate")
                    .id("immediate")
                    .build()
                    .unwrap(),
            )
            .unwrap();

        let mut nested_tasks = TaskTree::new();
        nested_tasks
            .add_task(TaskBuilder::new("Nested Dir").id("nested").build().unwrap())
            .unwrap();

        let mut source_tasks = TaskTree::new();
        source_tasks
            .add_task(TaskBuilder::new("Source").id("source").build().unwrap())
            .unwrap();

        // immediate is in core/, nested is in core/subdir/
        let (immediate_path, immediate_file) = create_test_file(
            "core/immediate.md",
            "core.immediate",
            "Immediate",
            immediate_tasks,
        );
        let (nested_path, nested_file) = create_test_file(
            "core/subdir/nested.md",
            "core.subdir.nested",
            "Nested",
            nested_tasks,
        );
        let (source_path, mut source_file) = create_test_file(
            "features/feature.md",
            "features.feature",
            "Feature",
            source_tasks,
        );

        if let Some(source_task) = source_file.tasks.get_task_mut("source") {
            source_task
                .metadata
                .depends_on
                .push(parse_dependency_ref("../core/").unwrap());
        }

        let mut files = HashMap::new();
        files.insert(immediate_path, immediate_file);
        files.insert(nested_path, nested_file);
        files.insert(source_path, source_file);

        let mut resolver = DependencyResolver::new(&files, PathBuf::from("/project"));
        let result = resolver.resolve_dependencies();

        assert_eq!(result.errors.len(), 0);
        // Only 1 dependency (immediate child, not nested directory)
        assert_eq!(
            result.resolved.len(),
            1,
            "Should only depend on immediate children"
        );
        assert_eq!(result.resolved[0].to_full_id, "core.immediate#immediate");
    }

    #[test]
    fn test_directory_dependency_not_found() {
        let mut source_tasks = TaskTree::new();
        source_tasks
            .add_task(TaskBuilder::new("Source").id("source").build().unwrap())
            .unwrap();

        let (source_path, mut source_file) = create_test_file(
            "features/feature.md",
            "features.feature",
            "Feature",
            source_tasks,
        );

        if let Some(source_task) = source_file.tasks.get_task_mut("source") {
            source_task
                .metadata
                .depends_on
                .push(parse_dependency_ref("../nonexistent/").unwrap());
        }

        let mut files = HashMap::new();
        files.insert(source_path, source_file);

        let mut resolver = DependencyResolver::new(&files, PathBuf::from("/project"));
        let result = resolver.resolve_dependencies();

        assert_eq!(result.resolved.len(), 0);
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(
            result.errors[0].error_kind,
            ResolutionErrorKind::DirectoryNotFound { .. }
        ));
    }

    #[test]
    fn test_directory_dependency_kind() {
        // Verify that directory deps have DependencyKind::Directory
        let mut core_tasks = TaskTree::new();
        core_tasks
            .add_task(
                TaskBuilder::new("Core Task")
                    .id("core-task")
                    .build()
                    .unwrap(),
            )
            .unwrap();

        let mut source_tasks = TaskTree::new();
        source_tasks
            .add_task(TaskBuilder::new("Source").id("source").build().unwrap())
            .unwrap();

        let (core_path, core_file) =
            create_test_file("core/tasks.md", "core.tasks", "Core", core_tasks);
        let (source_path, mut source_file) = create_test_file(
            "features/feature.md",
            "features.feature",
            "Feature",
            source_tasks,
        );

        if let Some(source_task) = source_file.tasks.get_task_mut("source") {
            source_task
                .metadata
                .depends_on
                .push(parse_dependency_ref("../core/").unwrap());
        }

        let mut files = HashMap::new();
        files.insert(core_path, core_file);
        files.insert(source_path, source_file);

        let mut resolver = DependencyResolver::new(&files, PathBuf::from("/project"));
        let result = resolver.resolve_dependencies();

        assert_eq!(result.resolved.len(), 1);
        assert_eq!(result.resolved[0].kind, DependencyKind::Directory);
    }
}
