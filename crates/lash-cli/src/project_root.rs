//! Project root detection
//!
//! This module implements logic to automatically discover the Lash project root
//! by searching for marker files (`lash.index.md`, `index.lash.md`, or `.lash/` directory).

use anyhow::{Context, Result};
use std::env;
use std::path::{Path, PathBuf};

/// Project root finder
///
/// Searches upward from a starting directory to find the Lash project root.
///
/// # Example
///
/// ```
/// use lash_cli::project_root::ProjectRootFinder;
/// use std::env;
///
/// let finder = ProjectRootFinder::new();
/// // Search from current directory
/// match finder.find_from_cwd() {
///     Ok(root) => println!("Found project root: {}", root.display()),
///     Err(e) => eprintln!("No project root found: {}", e),
/// }
/// ```
#[derive(Debug)]
pub struct ProjectRootFinder {
    /// Cached root after first discovery
    cached_root: Option<PathBuf>,
}

impl ProjectRootFinder {
    /// Create a new project root finder
    ///
    /// # Example
    ///
    /// ```
    /// use lash_cli::project_root::ProjectRootFinder;
    ///
    /// let finder = ProjectRootFinder::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self { cached_root: None }
    }

    /// Find project root starting from the current working directory
    ///
    /// # Returns
    ///
    /// The path to the project root directory
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Unable to determine current working directory
    /// - No project root markers found
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_cli::project_root::ProjectRootFinder;
    ///
    /// let finder = ProjectRootFinder::new();
    /// let root = finder.find_from_cwd().expect("No project root found");
    /// println!("Project root: {}", root.display());
    /// ```
    pub fn find_from_cwd(&self) -> Result<PathBuf> {
        let cwd = env::current_dir().context("Failed to get current working directory")?;
        self.find_from(&cwd)
    }

    /// Find project root starting from a specific directory
    ///
    /// Searches upward from the given directory until a project root marker is found
    /// or the filesystem root is reached.
    ///
    /// # Arguments
    ///
    /// * `start_dir` - Directory to start searching from
    ///
    /// # Returns
    ///
    /// The path to the project root directory
    ///
    /// # Errors
    ///
    /// Returns an error if no project root markers are found before reaching
    /// the filesystem root or home directory.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_cli::project_root::ProjectRootFinder;
    /// use std::path::Path;
    ///
    /// let finder = ProjectRootFinder::new();
    /// let root = finder.find_from(Path::new("/path/to/project/subdir"))
    ///     .expect("No project root found");
    /// println!("Project root: {}", root.display());
    /// ```
    pub fn find_from(&self, start_dir: &Path) -> Result<PathBuf> {
        // Return cached result if available
        if let Some(ref root) = self.cached_root {
            return Ok(root.clone());
        }

        self.search_upward(start_dir)
    }

    /// Search upward from a directory to find project root markers
    #[allow(clippy::unused_self)] // self used for potential caching in future
    fn search_upward(&self, start_dir: &Path) -> Result<PathBuf> {
        let mut current = start_dir.canonicalize().context(format!(
            "Failed to canonicalize starting directory: {}",
            start_dir.display()
        ))?;

        let home_dir = dirs::home_dir();
        let mut search_path = Vec::new();

        loop {
            search_path.push(current.clone());

            // Check for project markers
            if has_project_markers(&current) {
                return Ok(current);
            }

            // Stop at home directory to avoid searching too far
            if let Some(ref home) = home_dir {
                if current == *home {
                    break;
                }
            }

            // Move up one directory
            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => break, // Reached filesystem root
            }
        }

        // No project root found
        anyhow::bail!(
            "No Lash project root found. Searched in:\n  {}\n\n\
             A Lash project root should contain one of:\n\
             - lash.index.md\n\
             - index.lash.md\n\
             - .lash/ directory\n\n\
             Hint: Initialize a new project with 'lash init' (when implemented)",
            search_path
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join("\n  ")
        )
    }

    /// Validate that a path is a valid project root
    ///
    /// # Arguments
    ///
    /// * `path` - Path to validate
    ///
    /// # Returns
    ///
    /// `Ok(())` if the path is a valid project root, otherwise an error
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Path doesn't exist
    /// - Path is not a directory
    /// - Path doesn't contain project markers
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_cli::project_root::ProjectRootFinder;
    /// use std::path::Path;
    ///
    /// let finder = ProjectRootFinder::new();
    /// match finder.validate_root(Path::new("/path/to/project")) {
    ///     Ok(()) => println!("Valid project root"),
    ///     Err(e) => eprintln!("Invalid: {}", e),
    /// }
    /// ```
    pub fn validate_root(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            anyhow::bail!("Path does not exist: {}", path.display());
        }

        if !path.is_dir() {
            anyhow::bail!("Path is not a directory: {}", path.display());
        }

        if !has_project_markers(path) {
            anyhow::bail!(
                "Path is not a Lash project root (missing lash.index.md, index.lash.md, or .lash/): {}",
                path.display()
            );
        }

        Ok(())
    }
}

impl Default for ProjectRootFinder {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a directory contains Lash project markers
///
/// Returns true if the directory contains:
/// - `lash.index.md` file
/// - `index.lash.md` file
/// - `.lash/` directory
fn has_project_markers(dir: &Path) -> bool {
    dir.join("lash.index.md").exists()
        || dir.join("index.lash.md").exists()
        || dir.join(".lash").is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_has_project_markers_with_lash_index() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("lash.index.md"), "# Index").unwrap();
        assert!(has_project_markers(temp.path()));
    }

    #[test]
    fn test_has_project_markers_with_index_lash() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("index.lash.md"), "# Index").unwrap();
        assert!(has_project_markers(temp.path()));
    }

    #[test]
    fn test_has_project_markers_with_lash_dir() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join(".lash")).unwrap();
        assert!(has_project_markers(temp.path()));
    }

    #[test]
    fn test_has_project_markers_without_markers() {
        let temp = TempDir::new().unwrap();
        assert!(!has_project_markers(temp.path()));
    }

    #[test]
    fn test_find_from_project_root() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("lash.index.md"), "# Index").unwrap();

        let finder = ProjectRootFinder::new();
        let root = finder.find_from(temp.path()).unwrap();
        assert_eq!(
            root.canonicalize().unwrap(),
            temp.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn test_find_from_subdirectory() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        fs::write(root.join("lash.index.md"), "# Index").unwrap();

        // Create nested subdirectories
        let subdir = root.join("tasks").join("backend");
        fs::create_dir_all(&subdir).unwrap();

        let finder = ProjectRootFinder::new();
        let found_root = finder.find_from(&subdir).unwrap();
        assert_eq!(
            found_root.canonicalize().unwrap(),
            root.canonicalize().unwrap()
        );
    }

    #[test]
    fn test_find_from_no_markers() {
        let temp = TempDir::new().unwrap();
        let finder = ProjectRootFinder::new();
        let result = finder.find_from(temp.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("No Lash project root found"));
    }

    #[test]
    fn test_validate_root_valid() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("lash.index.md"), "# Index").unwrap();

        let finder = ProjectRootFinder::new();
        assert!(finder.validate_root(temp.path()).is_ok());
    }

    #[test]
    fn test_validate_root_nonexistent() {
        let finder = ProjectRootFinder::new();
        let result = finder.validate_root(Path::new("/nonexistent/path"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_validate_root_not_directory() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("file.txt");
        fs::write(&file_path, "content").unwrap();

        let finder = ProjectRootFinder::new();
        let result = finder.validate_root(&file_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a directory"));
    }

    #[test]
    fn test_validate_root_no_markers() {
        let temp = TempDir::new().unwrap();
        let finder = ProjectRootFinder::new();
        let result = finder.validate_root(temp.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not a Lash project root"));
    }

    #[test]
    fn test_finder_default() {
        let _finder = ProjectRootFinder::default();
        // Just ensure it compiles and doesn't panic
    }
}
