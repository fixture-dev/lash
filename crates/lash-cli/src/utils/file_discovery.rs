//! File discovery utilities with gitignore support
//!
//! This module provides functionality to discover Lash task files (.md files)
//! in a directory tree while respecting .gitignore and .lashignore patterns.

use anyhow::{Context, Result};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Discover all `.md` files in the given paths
///
/// If a path is a file, it's returned directly (if it's a .md file).
/// If a path is a directory, all .md files are recursively discovered.
///
/// # Arguments
///
/// * `paths` - List of files or directories to search
/// * `respect_gitignore` - Whether to respect .gitignore patterns
///
/// # Returns
///
/// A sorted list of absolute paths to .md files
///
/// # Errors
///
/// Returns an error if:
/// - A path doesn't exist
/// - Filesystem traversal fails
pub fn discover_markdown_files(paths: &[PathBuf], respect_gitignore: bool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for path in paths {
        if !path.exists() {
            anyhow::bail!("Path does not exist: {}", path.display());
        }

        if path.is_file() {
            // If it's a file, check if it's a markdown file
            if is_markdown_file(path) {
                files.push(path.canonicalize()?);
            } else {
                anyhow::bail!("File is not a markdown file: {}", path.display());
            }
        } else if path.is_dir() {
            // If it's a directory, walk it and collect all .md files
            let discovered = walk_directory(path, respect_gitignore)?;
            files.extend(discovered);
        }
    }

    // Sort for deterministic output
    files.sort();
    files.dedup();

    Ok(files)
}

/// Walk a directory and collect all markdown files
fn walk_directory(dir: &Path, respect_gitignore: bool) -> Result<Vec<PathBuf>> {
    let mut builder = WalkBuilder::new(dir);
    builder
        .follow_links(false)
        .git_ignore(respect_gitignore)
        .add_custom_ignore_filename(".lashignore");

    let mut files = Vec::new();

    for entry in builder.build() {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if path.is_file() && is_markdown_file(path) {
            files.push(path.to_path_buf());
        }
    }

    Ok(files)
}

/// Check if a path is a markdown file (has .md extension)
fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
}

/// Find the project root by looking for lash.index.md or .lash/ directory
///
/// Searches upward from the given directory until a project root marker is found.
/// If no marker is found, returns the original directory.
///
/// # Arguments
///
/// * `start_dir` - Directory to start searching from
///
/// # Returns
///
/// The project root directory
pub fn find_project_root(start_dir: &Path) -> PathBuf {
    let mut current = start_dir;

    loop {
        // Check for lash.index.md or index.lash.md
        if current.join("lash.index.md").exists() || current.join("index.lash.md").exists() {
            return current.to_path_buf();
        }

        // Check for .lash/ directory
        if current.join(".lash").is_dir() {
            return current.to_path_buf();
        }

        // Move up one directory
        match current.parent() {
            Some(parent) => current = parent,
            None => {
                // Reached root without finding project marker, return original dir
                return start_dir.to_path_buf();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_is_markdown_file() {
        assert!(is_markdown_file(Path::new("test.md")));
        assert!(is_markdown_file(Path::new("test.MD")));
        assert!(!is_markdown_file(Path::new("test.txt")));
        assert!(!is_markdown_file(Path::new("test")));
    }

    #[test]
    fn test_discover_single_file() -> Result<()> {
        let temp = TempDir::new()?;
        let file_path = temp.path().join("test.md");
        fs::write(&file_path, "# Test")?;

        let files = discover_markdown_files(&[file_path.clone()], true)?;
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("test.md"));

        Ok(())
    }

    #[test]
    fn test_discover_directory() -> Result<()> {
        let temp = TempDir::new()?;
        fs::write(temp.path().join("one.md"), "# One")?;
        fs::write(temp.path().join("two.md"), "# Two")?;
        fs::write(temp.path().join("skip.txt"), "Skip me")?;

        let subdir = temp.path().join("subdir");
        fs::create_dir(&subdir)?;
        fs::write(subdir.join("three.md"), "# Three")?;

        let files = discover_markdown_files(&[temp.path().to_path_buf()], true)?;
        assert_eq!(files.len(), 3);

        // Check that files are sorted
        assert!(files[0].file_name().unwrap() == "one.md");
        assert!(files[1].file_name().unwrap() == "three.md");
        assert!(files[2].file_name().unwrap() == "two.md");

        Ok(())
    }

    #[test]
    fn test_discover_nonexistent_path() {
        let result = discover_markdown_files(&[PathBuf::from("/nonexistent/path")], true);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_project_root() -> Result<()> {
        let temp = TempDir::new()?;
        let root = temp.path();
        let subdir = root.join("tasks");
        fs::create_dir(&subdir)?;

        // Create lash.index.md in root
        fs::write(root.join("lash.index.md"), "# Index")?;

        // Find from subdirectory should return root
        let found_root = find_project_root(&subdir);
        assert_eq!(found_root, root);

        Ok(())
    }

    #[test]
    fn test_find_project_root_with_lash_dir() -> Result<()> {
        let temp = TempDir::new()?;
        let root = temp.path();
        let subdir = root.join("nested").join("deep");
        fs::create_dir_all(&subdir)?;

        // Create .lash/ directory in root
        fs::create_dir(root.join(".lash"))?;

        // Find from deep subdirectory should return root
        let found_root = find_project_root(&subdir);
        assert_eq!(found_root, root);

        Ok(())
    }
}
