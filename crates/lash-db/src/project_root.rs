//! Project root discovery for Lash projects
//!
//! This module provides functionality to locate the root directory of a Lash project
//! by searching upward from the current directory for project markers.
//!
//! # Project Markers
//!
//! The following markers indicate a Lash project root (in order of precedence):
//!
//! 1. `.lash/` directory (explicit marker, highest precedence)
//! 2. `lash.index.md` file (conventional marker)
//!
//! # Example
//!
//! ```no_run
//! use lash_db::project_root::find_project_root;
//!
//! // Find project root from current directory
//! match find_project_root() {
//!     Ok(root) => println!("Project root: {}", root.display()),
//!     Err(e) => eprintln!("Error: {}", e),
//! }
//! ```

use crate::error::{DbError, DbResult};
use std::path::{Path, PathBuf};

/// Configuration options for project root discovery
#[derive(Debug, Clone, Default)]
pub struct ProjectRootConfig {
    /// Optional explicit root path override (useful for testing)
    pub explicit_root: Option<PathBuf>,

    /// Maximum search depth (None = unlimited, stops at filesystem root)
    pub max_depth: Option<usize>,
}

impl ProjectRootConfig {
    /// Create a new config with default settings
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::project_root::ProjectRootConfig;
    ///
    /// let config = ProjectRootConfig::new();
    /// assert!(config.explicit_root.is_none());
    /// assert!(config.max_depth.is_none());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set an explicit root path (bypasses search)
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::project_root::ProjectRootConfig;
    /// use std::path::PathBuf;
    ///
    /// let config = ProjectRootConfig::new()
    ///     .with_explicit_root(PathBuf::from("/tmp/my-project"));
    /// ```
    #[must_use]
    pub fn with_explicit_root(mut self, path: PathBuf) -> Self {
        self.explicit_root = Some(path);
        self
    }

    /// Set maximum search depth
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::project_root::ProjectRootConfig;
    ///
    /// let config = ProjectRootConfig::new()
    ///     .with_max_depth(10);
    /// ```
    #[must_use]
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }
}

/// Find the Lash project root directory
///
/// Searches upward from the current directory for project markers:
/// - `.lash/` directory (highest precedence)
/// - `lash.index.md` file
///
/// # Example
///
/// ```no_run
/// use lash_db::project_root::find_project_root;
///
/// let root = find_project_root()?;
/// println!("Found project root at: {}", root.display());
/// # Ok::<(), lash_db::DbError>(())
/// ```
///
/// # Errors
///
/// Returns `DbError::ProjectRootNotFound` if:
/// - No project markers found before reaching filesystem root
/// - Maximum search depth exceeded
/// - I/O errors during directory traversal
pub fn find_project_root() -> DbResult<PathBuf> {
    find_project_root_with_config(&ProjectRootConfig::default())
}

/// Find the Lash project root directory with custom configuration
///
/// # Example
///
/// ```no_run
/// use lash_db::project_root::{find_project_root_with_config, ProjectRootConfig};
/// use std::path::PathBuf;
///
/// let config = ProjectRootConfig::new()
///     .with_max_depth(10);
/// let root = find_project_root_with_config(&config)?;
/// # Ok::<(), lash_db::DbError>(())
/// ```
///
/// # Errors
///
/// Returns error if:
/// - No project markers found
/// - Maximum search depth exceeded
/// - I/O errors during directory traversal
/// - Explicit root path doesn't exist
pub fn find_project_root_with_config(config: &ProjectRootConfig) -> DbResult<PathBuf> {
    // If explicit root is provided, validate and return it
    if let Some(ref explicit_root) = config.explicit_root {
        if !explicit_root.exists() {
            return Err(DbError::ProjectRootNotFound(format!(
                "Explicit root path does not exist: {}",
                explicit_root.display()
            )));
        }
        return Ok(explicit_root.clone());
    }

    // Start from current directory
    let start_dir = std::env::current_dir().map_err(|e| {
        DbError::ProjectRootNotFound(format!("Failed to get current directory: {e}"))
    })?;

    find_project_root_from(&start_dir, config)
}

/// Find the Lash project root starting from a specific directory
///
/// This is useful for testing and for tools that operate on specific paths.
///
/// # Example
///
/// ```no_run
/// use lash_db::project_root::{find_project_root_from, ProjectRootConfig};
/// use std::path::Path;
///
/// let config = ProjectRootConfig::default();
/// let root = find_project_root_from(Path::new("/some/nested/dir"), &config)?;
/// # Ok::<(), lash_db::DbError>(())
/// ```
///
/// # Errors
///
/// Returns error if no project root is found or I/O errors occur.
pub fn find_project_root_from(start_path: &Path, config: &ProjectRootConfig) -> DbResult<PathBuf> {
    let mut current = start_path.canonicalize().map_err(|e| {
        DbError::ProjectRootNotFound(format!(
            "Failed to canonicalize start path {}: {}",
            start_path.display(),
            e
        ))
    })?;

    // Cap the upward walk at the enclosing git repo (if any) so leftover
    // state in ancestor directories cannot hijack the lookup. See
    // `lash_types::path_utils::find_git_root` for the rationale.
    let git_root = lash_types::path_utils::find_git_root(&current);

    let mut depth = 0;

    loop {
        // Check max depth limit
        if let Some(max_depth) = config.max_depth {
            if depth >= max_depth {
                return Err(DbError::ProjectRootNotFound(format!(
                    "Maximum search depth ({max_depth}) exceeded without finding project root"
                )));
            }
        }

        // Check for project markers (in order of precedence)

        // 1. Check for .lash/ directory (highest precedence)
        let lash_dir = current.join(".lash");
        if lash_dir.is_dir() {
            return Ok(current);
        }

        // 2. Check for lash.index.md file
        let index_file = current.join("lash.index.md");
        if index_file.is_file() {
            return Ok(current);
        }

        // Stop at the git root — markers above it are not accepted.
        if let Some(ref gr) = git_root {
            if current == *gr {
                return Err(DbError::ProjectRootNotFound(
                    "No Lash project root found within the current git repository. \
                     Looking for .lash/ directory or lash.index.md file."
                        .to_string(),
                ));
            }
        }

        // Move up to parent directory
        match current.parent() {
            Some(parent) => {
                current = parent.to_path_buf();
                depth += 1;
            }
            None => {
                // Reached filesystem root without finding project markers
                return Err(DbError::ProjectRootNotFound(
                    "No Lash project root found. Looking for .lash/ directory or lash.index.md file."
                        .to_string(),
                ));
            }
        }
    }
}

/// Check if a directory is a valid Lash project root
///
/// Returns true if the directory contains any project markers.
///
/// # Example
///
/// ```no_run
/// use lash_db::project_root::is_project_root;
/// use std::path::Path;
///
/// if is_project_root(Path::new("/my/project")) {
///     println!("Valid Lash project root");
/// }
/// ```
#[must_use]
pub fn is_project_root(path: &Path) -> bool {
    let lash_dir = path.join(".lash");
    if lash_dir.is_dir() {
        return true;
    }

    let index_file = path.join("lash.index.md");
    index_file.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper to create a test project structure
    fn create_test_project(marker_type: &str) -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        match marker_type {
            "lash_dir" => {
                fs::create_dir(project_root.join(".lash")).unwrap();
            }
            "index_file" => {
                fs::write(project_root.join("lash.index.md"), "# Index").unwrap();
            }
            "both" => {
                fs::create_dir(project_root.join(".lash")).unwrap();
                fs::write(project_root.join("lash.index.md"), "# Index").unwrap();
            }
            "none" => {
                // No markers
            }
            _ => panic!("Unknown marker type: {marker_type}"),
        }

        temp_dir
    }

    #[test]
    fn test_find_root_with_lash_dir() {
        let temp_dir = create_test_project("lash_dir");
        let config = ProjectRootConfig::default();

        let result = find_project_root_from(temp_dir.path(), &config);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().canonicalize().unwrap(),
            temp_dir.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn test_find_root_with_index_file() {
        let temp_dir = create_test_project("index_file");
        let config = ProjectRootConfig::default();

        let result = find_project_root_from(temp_dir.path(), &config);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().canonicalize().unwrap(),
            temp_dir.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn test_precedence_lash_dir_over_index_file() {
        // When both markers present, .lash/ takes precedence
        // This test verifies that we find the root correctly
        // (The precedence is implicit in the search order)
        let temp_dir = create_test_project("both");
        let config = ProjectRootConfig::default();

        let result = find_project_root_from(temp_dir.path(), &config);
        assert!(result.is_ok());

        let root = result.unwrap();
        assert_eq!(
            root.canonicalize().unwrap(),
            temp_dir.path().canonicalize().unwrap()
        );

        // Verify both markers exist
        assert!(root.join(".lash").is_dir());
        assert!(root.join("lash.index.md").is_file());
    }

    #[test]
    fn test_no_markers_returns_error() {
        let temp_dir = create_test_project("none");
        let config = ProjectRootConfig::default();

        let result = find_project_root_from(temp_dir.path(), &config);
        assert!(result.is_err());

        if let Err(DbError::ProjectRootNotFound(msg)) = result {
            assert!(msg.contains("No Lash project root found"));
        } else {
            panic!("Expected ProjectRootNotFound error");
        }
    }

    #[test]
    fn test_nested_directory_search() {
        let temp_dir = create_test_project("lash_dir");

        // Create nested directories
        let nested = temp_dir.path().join("a").join("b").join("c");
        fs::create_dir_all(&nested).unwrap();

        let config = ProjectRootConfig::default();
        let result = find_project_root_from(&nested, &config);

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().canonicalize().unwrap(),
            temp_dir.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn test_max_depth_limit() {
        let temp_dir = create_test_project("lash_dir");

        // Create nested directories deeper than max_depth
        let nested = temp_dir
            .path()
            .join("a")
            .join("b")
            .join("c")
            .join("d")
            .join("e");
        fs::create_dir_all(&nested).unwrap();

        let config = ProjectRootConfig::new().with_max_depth(2);
        let result = find_project_root_from(&nested, &config);

        assert!(result.is_err());
        if let Err(DbError::ProjectRootNotFound(msg)) = result {
            assert!(msg.contains("Maximum search depth"));
        } else {
            panic!("Expected ProjectRootNotFound error with max depth message");
        }
    }

    #[test]
    fn test_explicit_root_override() {
        let temp_dir = create_test_project("lash_dir");

        let config = ProjectRootConfig::new().with_explicit_root(temp_dir.path().to_path_buf());

        let result = find_project_root_with_config(&config);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().canonicalize().unwrap(),
            temp_dir.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn test_explicit_root_nonexistent() {
        let config = ProjectRootConfig::new()
            .with_explicit_root(PathBuf::from("/nonexistent/path/that/does/not/exist"));

        let result = find_project_root_with_config(&config);
        assert!(result.is_err());

        if let Err(DbError::ProjectRootNotFound(msg)) = result {
            assert!(msg.contains("Explicit root path does not exist"));
        } else {
            panic!("Expected ProjectRootNotFound error");
        }
    }

    #[test]
    fn test_is_project_root() {
        let temp_lash_dir = create_test_project("lash_dir");
        assert!(is_project_root(temp_lash_dir.path()));

        let temp_index = create_test_project("index_file");
        assert!(is_project_root(temp_index.path()));

        let temp_none = create_test_project("none");
        assert!(!is_project_root(temp_none.path()));
    }

    #[test]
    fn test_nested_projects_stops_at_nearest() {
        // Create outer project with .lash/ directory
        let outer_temp = TempDir::new().unwrap();
        fs::create_dir(outer_temp.path().join(".lash")).unwrap();

        // Create inner project with lash.index.md
        let inner_path = outer_temp.path().join("subproject");
        fs::create_dir(&inner_path).unwrap();
        fs::write(inner_path.join("lash.index.md"), "# Inner").unwrap();

        // Create deeply nested directory inside inner project
        let nested = inner_path.join("src").join("components");
        fs::create_dir_all(&nested).unwrap();

        let config = ProjectRootConfig::default();
        let result = find_project_root_from(&nested, &config);

        assert!(result.is_ok());
        // Should find inner project root, not outer
        assert_eq!(
            result.unwrap().canonicalize().unwrap(),
            inner_path.canonicalize().unwrap()
        );
    }

    #[test]
    fn test_config_builder_pattern() {
        let config = ProjectRootConfig::new()
            .with_max_depth(5)
            .with_explicit_root(PathBuf::from("/tmp"));

        assert_eq!(config.max_depth, Some(5));
        assert_eq!(config.explicit_root, Some(PathBuf::from("/tmp")));
    }
}
