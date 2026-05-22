//! File system walker for discovering Markdown files in Lash projects
//!
//! This module provides functionality to recursively traverse a project directory
//! and discover all Markdown files, respecting `.gitignore` and custom exclusion patterns.
//!
//! # Example
//!
//! ```no_run
//! use lash_db::walker::{FileWalker, FileWalkerConfig};
//! use std::path::PathBuf;
//!
//! let config = FileWalkerConfig::new(PathBuf::from("/path/to/project"));
//! let walker = FileWalker::new(config);
//!
//! let files = walker.discover_files()?;
//! println!("Found {} markdown files", files.len());
//! # Ok::<(), lash_db::DbError>(())
//! ```

use crate::error::{DbError, DbResult};
use ignore::WalkBuilder;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Metadata for a discovered file
///
/// Contains all the information needed to track a file in the index,
/// including paths, size, modification time, and content hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMetadata {
    /// Absolute path to the file
    pub absolute_path: PathBuf,

    /// Path relative to the project root
    pub relative_path: PathBuf,

    /// File size in bytes
    pub size: u64,

    /// Last modification time (Unix timestamp in seconds)
    pub mtime: i64,

    /// BLAKE3 hash of file contents (hex-encoded)
    pub content_hash: String,
}

impl FileMetadata {
    /// Compute metadata for a file
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_db::walker::FileMetadata;
    /// use std::path::{Path, PathBuf};
    ///
    /// let metadata = FileMetadata::from_path(
    ///     Path::new("/project/tasks.md"),
    ///     Path::new("/project")
    /// )?;
    /// # Ok::<(), lash_db::DbError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - File cannot be read
    /// - File metadata cannot be accessed
    /// - Path cannot be made relative to project root
    pub fn from_path(absolute_path: &Path, project_root: &Path) -> DbResult<Self> {
        // Get file metadata
        let metadata = fs::metadata(absolute_path).map_err(|e| {
            DbError::IoError(format!(
                "Failed to read metadata for {}: {}",
                absolute_path.display(),
                e
            ))
        })?;

        // Compute relative path
        let relative_path = absolute_path
            .strip_prefix(project_root)
            .map_err(|e| {
                DbError::IoError(format!(
                    "Failed to compute relative path for {}: {}",
                    absolute_path.display(),
                    e
                ))
            })?
            .to_path_buf();

        // Get file size
        let size = metadata.len();

        // Get modification time
        let mtime_secs = metadata
            .modified()
            .map_err(|e| {
                DbError::IoError(format!(
                    "Failed to get modification time for {}: {}",
                    absolute_path.display(),
                    e
                ))
            })?
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| {
                DbError::IoError(format!(
                    "Invalid modification time for {}: {}",
                    absolute_path.display(),
                    e
                ))
            })?
            .as_secs();

        // Convert to i64, clamping to max i64 if it would overflow
        #[allow(clippy::cast_possible_wrap)]
        let mtime = mtime_secs.min(i64::MAX as u64) as i64;

        // Compute content hash
        let content_hash = Self::compute_hash(absolute_path)?;

        Ok(Self {
            absolute_path: absolute_path.to_path_buf(),
            relative_path,
            size,
            mtime,
            content_hash,
        })
    }

    /// Compute BLAKE3 hash of file contents
    ///
    /// Returns hex-encoded hash string.
    fn compute_hash(path: &Path) -> DbResult<String> {
        let contents = fs::read(path).map_err(|e| {
            DbError::IoError(format!("Failed to read file {}: {}", path.display(), e))
        })?;

        let hash = blake3::hash(&contents);
        Ok(hash.to_hex().to_string())
    }
}

/// Configuration for the file walker
///
/// Specifies which files to discover and which patterns to exclude.
#[derive(Debug, Clone)]
pub struct FileWalkerConfig {
    /// Project root directory to start walking from
    pub project_root: PathBuf,

    /// Specific paths to walk (if empty, walks `project_root`)
    /// Each path must be under `project_root`
    pub paths: Vec<PathBuf>,

    /// File extensions to include (e.g., "md")
    /// If empty, includes all files
    pub extensions: Vec<String>,

    /// Additional exclude patterns beyond `.gitignore`
    /// Supports glob patterns (e.g., "*.tmp", "`node_modules`/")
    pub exclude_patterns: Vec<String>,

    /// Whether to respect `.gitignore` files
    pub respect_gitignore: bool,

    /// Whether to follow symbolic links
    /// Default: false for safety (prevents cycles and unexpected traversal)
    pub follow_symlinks: bool,
}

impl FileWalkerConfig {
    /// Create a new configuration with default settings
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::walker::FileWalkerConfig;
    /// use std::path::PathBuf;
    ///
    /// let config = FileWalkerConfig::new(PathBuf::from("/project"));
    /// ```
    #[must_use]
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            paths: Vec::new(),
            extensions: vec!["md".to_string()],
            exclude_patterns: Self::default_exclude_patterns(),
            respect_gitignore: true,
            follow_symlinks: false,
        }
    }

    /// Default exclude patterns for Lash projects
    ///
    /// Excludes common development artifacts and the Lash database.
    fn default_exclude_patterns() -> Vec<String> {
        vec![
            ".git/".to_string(),
            "node_modules/".to_string(),
            "target/".to_string(),
            ".lash/db.sqlite".to_string(),
            ".lash/db.sqlite-wal".to_string(),
            ".lash/db.sqlite-shm".to_string(),
        ]
    }

    /// Set file extensions to include
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::walker::FileWalkerConfig;
    /// use std::path::PathBuf;
    ///
    /// let config = FileWalkerConfig::new(PathBuf::from("/project"))
    ///     .with_extensions(vec!["md".to_string(), "markdown".to_string()]);
    /// ```
    #[must_use]
    pub fn with_extensions(mut self, extensions: Vec<String>) -> Self {
        self.extensions = extensions;
        self
    }

    /// Set exclude patterns
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::walker::FileWalkerConfig;
    /// use std::path::PathBuf;
    ///
    /// let config = FileWalkerConfig::new(PathBuf::from("/project"))
    ///     .with_exclude_patterns(vec!["*.tmp".to_string()]);
    /// ```
    #[must_use]
    pub fn with_exclude_patterns(mut self, patterns: Vec<String>) -> Self {
        self.exclude_patterns = patterns;
        self
    }

    /// Set whether to respect `.gitignore`
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::walker::FileWalkerConfig;
    /// use std::path::PathBuf;
    ///
    /// let config = FileWalkerConfig::new(PathBuf::from("/project"))
    ///     .with_respect_gitignore(false);
    /// ```
    #[must_use]
    pub fn with_respect_gitignore(mut self, respect: bool) -> Self {
        self.respect_gitignore = respect;
        self
    }

    /// Set whether to follow symbolic links
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::walker::FileWalkerConfig;
    /// use std::path::PathBuf;
    ///
    /// let config = FileWalkerConfig::new(PathBuf::from("/project"))
    ///     .with_follow_symlinks(true);
    /// ```
    #[must_use]
    pub fn with_follow_symlinks(mut self, follow: bool) -> Self {
        self.follow_symlinks = follow;
        self
    }

    /// Set specific paths to walk (instead of entire project root)
    ///
    /// When paths are provided, only files under those paths will be discovered.
    /// Each path should be under the project root.
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::walker::FileWalkerConfig;
    /// use std::path::PathBuf;
    ///
    /// let config = FileWalkerConfig::new(PathBuf::from("/project"))
    ///     .with_paths(vec![PathBuf::from("/project/tasks")]);
    /// ```
    #[must_use]
    pub fn with_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.paths = paths;
        self
    }
}

/// File system walker for discovering Markdown files
///
/// Uses the `ignore` crate (same as ripgrep) for efficient directory traversal
/// with `.gitignore` support and custom exclusion patterns.
pub struct FileWalker {
    config: FileWalkerConfig,
}

impl FileWalker {
    /// Create a new file walker with the given configuration
    ///
    /// # Example
    ///
    /// ```
    /// use lash_db::walker::{FileWalker, FileWalkerConfig};
    /// use std::path::PathBuf;
    ///
    /// let config = FileWalkerConfig::new(PathBuf::from("/project"));
    /// let walker = FileWalker::new(config);
    /// ```
    #[must_use]
    pub fn new(config: FileWalkerConfig) -> Self {
        Self { config }
    }

    /// Discover all matching files in the project
    ///
    /// Returns a vector of file metadata for all discovered files.
    /// Files are returned in deterministic order (sorted by path).
    ///
    /// When `paths` is configured, only files under those paths are discovered.
    /// Otherwise, discovers all files under `project_root`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_db::walker::{FileWalker, FileWalkerConfig};
    /// use std::path::PathBuf;
    ///
    /// let config = FileWalkerConfig::new(PathBuf::from("/project"));
    /// let walker = FileWalker::new(config);
    ///
    /// let files = walker.discover_files()?;
    /// for file in &files {
    ///     println!("Found: {}", file.relative_path.display());
    /// }
    /// # Ok::<(), lash_db::DbError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Project root doesn't exist
    /// - Permission denied on directories (skips with warning)
    /// - I/O errors during traversal
    ///
    /// # Edge Cases
    ///
    /// - **Permission denied**: Skips directory and logs warning
    /// - **Broken symlinks**: Skips with warning
    /// - **Unicode filenames**: Fully supported
    /// - **Large directories**: Uses streaming iterator (efficient memory usage)
    pub fn discover_files(&self) -> DbResult<Vec<FileMetadata>> {
        // Determine which paths to walk
        let walk_paths = if self.config.paths.is_empty() {
            vec![self.config.project_root.clone()]
        } else {
            self.config.paths.clone()
        };

        let mut files = Vec::new();

        for walk_path in &walk_paths {
            // Check if path exists
            if !walk_path.exists() {
                return Err(DbError::IoError(format!(
                    "Path does not exist: {}",
                    walk_path.display()
                )));
            }

            // Handle single file case
            if walk_path.is_file() {
                if self.matches_extension(walk_path) && !self.is_excluded(walk_path) {
                    match FileMetadata::from_path(walk_path, &self.config.project_root) {
                        Ok(metadata) => files.push(metadata),
                        Err(e) => {
                            eprintln!("Warning: Skipping {}: {}", walk_path.display(), e);
                        }
                    }
                }
                continue;
            }

            // Build walker for this path
            let mut builder = WalkBuilder::new(walk_path);
            builder
                .follow_links(self.config.follow_symlinks)
                .git_ignore(self.config.respect_gitignore)
                .git_global(self.config.respect_gitignore)
                .git_exclude(self.config.respect_gitignore)
                .add_custom_ignore_filename(".lashignore");

            let walker = builder.build();

            // Walk the directory tree
            for entry in walker {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();

                        // Skip directories
                        if path.is_dir() {
                            continue;
                        }

                        // Skip symlinks if not following them
                        if !self.config.follow_symlinks && path.is_symlink() {
                            continue;
                        }

                        // Check if file matches extension filter
                        if !self.matches_extension(path) {
                            continue;
                        }

                        // Check if file matches exclude patterns
                        if self.is_excluded(path) {
                            continue;
                        }

                        // Collect metadata
                        match FileMetadata::from_path(path, &self.config.project_root) {
                            Ok(metadata) => files.push(metadata),
                            Err(e) => {
                                // Log warning but continue (handles permission denied, etc.)
                                eprintln!("Warning: Skipping {}: {}", path.display(), e);
                            }
                        }
                    }
                    Err(e) => {
                        // Log error but continue (handles broken symlinks, etc.)
                        eprintln!("Warning: Error during directory traversal: {e}");
                    }
                }
            }
        }

        // Sort by relative path for deterministic output
        files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

        // Deduplicate (in case multiple paths overlap)
        files.dedup_by(|a, b| a.relative_path == b.relative_path);

        Ok(files)
    }

    /// Check if a file matches the configured extensions
    fn matches_extension(&self, path: &Path) -> bool {
        // If no extensions specified, match all files
        if self.config.extensions.is_empty() {
            return true;
        }

        // Check if file has one of the specified extensions
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            self.config
                .extensions
                .iter()
                .any(|e| e.to_lowercase() == ext_str)
        } else {
            false
        }
    }

    /// Check if a file should be excluded based on exclude patterns
    fn is_excluded(&self, path: &Path) -> bool {
        // Get path relative to project root
        let Ok(relative_path) = path.strip_prefix(&self.config.project_root) else {
            return false; // If we can't get relative path, don't exclude
        };

        let path_str = relative_path.to_string_lossy();

        // Check each exclude pattern
        for pattern in &self.config.exclude_patterns {
            if pattern.ends_with('/') {
                // Directory pattern - check if path starts with it
                let dir_pattern = pattern.trim_end_matches('/');
                if path_str.starts_with(dir_pattern) {
                    return true;
                }
            } else if path_str == pattern.as_str() || path_str.ends_with(pattern.as_str()) {
                // Exact match or filename match
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper to create a test project structure
    fn create_test_project() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create directory structure
        fs::create_dir(root.join("docs")).unwrap();
        fs::create_dir(root.join("src")).unwrap();
        fs::create_dir(root.join(".git")).unwrap();
        fs::create_dir(root.join("node_modules")).unwrap();

        // Create markdown files
        fs::write(root.join("README.md"), "# README").unwrap();
        fs::write(root.join("docs/guide.md"), "# Guide").unwrap();
        fs::write(root.join("docs/api.md"), "# API").unwrap();
        fs::write(root.join("src/notes.md"), "# Notes").unwrap();

        // Create non-markdown files
        fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(root.join("Cargo.toml"), "[package]").unwrap();

        // Create files in excluded directories
        fs::write(root.join(".git/config"), "[core]").unwrap();
        fs::write(root.join("node_modules/package.md"), "# Package").unwrap();

        temp_dir
    }

    #[test]
    fn test_discover_markdown_files() {
        let temp_dir = create_test_project();
        let config = FileWalkerConfig::new(temp_dir.path().to_path_buf());
        let walker = FileWalker::new(config);

        let files = walker.discover_files().unwrap();

        // Should find 4 markdown files (excluding .git and node_modules)
        assert_eq!(files.len(), 4);

        // Verify all files are markdown
        for file in &files {
            assert_eq!(
                file.absolute_path.extension().unwrap(),
                "md",
                "File should have .md extension: {}",
                file.absolute_path.display()
            );
        }

        // Files should be sorted by relative path
        let paths: Vec<_> = files.iter().map(|f| &f.relative_path).collect();
        assert_eq!(paths[0], Path::new("README.md"));
        assert_eq!(paths[1], Path::new("docs/api.md"));
        assert_eq!(paths[2], Path::new("docs/guide.md"));
        assert_eq!(paths[3], Path::new("src/notes.md"));
    }

    #[test]
    fn test_file_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        let content = "# Test Content\n\nSome text here.";
        fs::write(&file_path, content).unwrap();

        let metadata = FileMetadata::from_path(&file_path, temp_dir.path()).unwrap();

        // Verify metadata
        assert_eq!(metadata.absolute_path, file_path);
        assert_eq!(metadata.relative_path, Path::new("test.md"));
        assert_eq!(metadata.size, content.len() as u64);
        assert!(metadata.mtime > 0);
        assert_eq!(metadata.content_hash.len(), 64); // BLAKE3 hash is 32 bytes = 64 hex chars

        // Verify hash is deterministic
        let metadata2 = FileMetadata::from_path(&file_path, temp_dir.path()).unwrap();
        assert_eq!(metadata.content_hash, metadata2.content_hash);
    }

    #[test]
    fn test_hash_changes_with_content() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");

        // Write initial content
        fs::write(&file_path, "Initial content").unwrap();
        let hash1 = FileMetadata::from_path(&file_path, temp_dir.path())
            .unwrap()
            .content_hash;

        // Modify content
        fs::write(&file_path, "Modified content").unwrap();
        let hash2 = FileMetadata::from_path(&file_path, temp_dir.path())
            .unwrap()
            .content_hash;

        // Hashes should be different
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_exclude_patterns() {
        let temp_dir = create_test_project();

        let config = FileWalkerConfig::new(temp_dir.path().to_path_buf());
        let walker = FileWalker::new(config);

        let files = walker.discover_files().unwrap();

        // Verify excluded directories are skipped
        for file in &files {
            assert!(
                !file.relative_path.starts_with(".git"),
                "Should not include .git files"
            );
            assert!(
                !file.relative_path.starts_with("node_modules"),
                "Should not include node_modules files"
            );
        }
    }

    #[test]
    fn test_custom_extensions() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create files with different extensions
        fs::write(root.join("file.md"), "markdown").unwrap();
        fs::write(root.join("file.txt"), "text").unwrap();
        fs::write(root.join("file.markdown"), "markdown").unwrap();

        // Test with default extensions (md only)
        let config = FileWalkerConfig::new(root.to_path_buf());
        let walker = FileWalker::new(config);
        let files = walker.discover_files().unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, Path::new("file.md"));

        // Test with custom extensions
        let config = FileWalkerConfig::new(root.to_path_buf())
            .with_extensions(vec!["md".to_string(), "markdown".to_string()]);
        let walker = FileWalker::new(config);
        let files = walker.discover_files().unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_no_extension_filter() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create files with different extensions
        fs::write(root.join("file.md"), "markdown").unwrap();
        fs::write(root.join("file.txt"), "text").unwrap();
        fs::write(root.join("file.rs"), "rust").unwrap();

        // Test with empty extensions (match all)
        let config = FileWalkerConfig::new(root.to_path_buf()).with_extensions(vec![]);
        let walker = FileWalker::new(config);
        let files = walker.discover_files().unwrap();
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_unicode_filenames() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create files with unicode names
        fs::write(root.join("日本語.md"), "Japanese").unwrap();
        fs::write(root.join("émojis🎉.md"), "French and emoji").unwrap();
        fs::write(root.join("Ελληνικά.md"), "Greek").unwrap();

        let config = FileWalkerConfig::new(root.to_path_buf());
        let walker = FileWalker::new(config);
        let files = walker.discover_files().unwrap();

        assert_eq!(files.len(), 3);

        // Verify unicode filenames are preserved
        let filenames: Vec<_> = files
            .iter()
            .map(|f| f.relative_path.file_name().unwrap().to_string_lossy())
            .collect();

        assert!(filenames.contains(&"日本語.md".into()));
        assert!(filenames.contains(&"émojis🎉.md".into()));
        assert!(filenames.contains(&"Ελληνικά.md".into()));
    }

    #[test]
    fn test_gitignore_respect() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Initialize git repo (required for .gitignore to be respected by ignore crate)
        std::process::Command::new("git")
            .arg("init")
            .current_dir(root)
            .output()
            .expect("Failed to initialize git repo");

        // Create .gitignore
        fs::write(root.join(".gitignore"), "ignored/\n*.tmp.md\n").unwrap();

        // Create directory structure
        fs::create_dir(root.join("ignored")).unwrap();
        fs::create_dir(root.join("included")).unwrap();

        // Create files
        fs::write(root.join("included/file.md"), "content").unwrap();
        fs::write(root.join("ignored/file.md"), "content").unwrap();
        fs::write(root.join("temp.tmp.md"), "content").unwrap();
        fs::write(root.join("normal.md"), "content").unwrap();

        // Test with gitignore respected (default)
        let config = FileWalkerConfig::new(root.to_path_buf());
        let walker = FileWalker::new(config);
        let files = walker.discover_files().unwrap();

        // Should only find files not in .gitignore
        assert_eq!(files.len(), 2);
        let paths: Vec<_> = files
            .iter()
            .map(|f| {
                // Normalize path separators for cross-platform comparison
                f.relative_path.to_string_lossy().replace('\\', "/")
            })
            .collect();
        assert!(paths.contains(&"included/file.md".to_string()));
        assert!(paths.contains(&"normal.md".to_string()));
        assert!(!paths.contains(&"ignored/file.md".to_string()));
        assert!(!paths.contains(&"temp.tmp.md".to_string()));

        // Test with gitignore disabled
        let config = FileWalkerConfig::new(root.to_path_buf()).with_respect_gitignore(false);
        let walker = FileWalker::new(config);
        let files = walker.discover_files().unwrap();

        // Should find all markdown files
        assert_eq!(files.len(), 4);
    }

    #[test]
    fn test_lashignore_respect() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // .lashignore is honored independently of git, so no `git init` needed.
        fs::write(
            root.join(".lashignore"),
            "skipped/\nnoindex.md\nnested/skipped-too/\n",
        )
        .unwrap();

        fs::create_dir(root.join("skipped")).unwrap();
        fs::create_dir_all(root.join("nested/skipped-too")).unwrap();
        fs::create_dir(root.join("kept")).unwrap();

        fs::write(root.join("skipped/a.md"), "x").unwrap();
        fs::write(root.join("nested/skipped-too/b.md"), "x").unwrap();
        fs::write(root.join("kept/c.md"), "x").unwrap();
        fs::write(root.join("noindex.md"), "x").unwrap();
        fs::write(root.join("indexed.md"), "x").unwrap();

        let config = FileWalkerConfig::new(root.to_path_buf());
        let walker = FileWalker::new(config);
        let files = walker.discover_files().unwrap();

        let paths: Vec<String> = files
            .iter()
            .map(|f| f.relative_path.to_string_lossy().replace('\\', "/"))
            .collect();

        assert!(paths.contains(&"indexed.md".to_string()));
        assert!(paths.contains(&"kept/c.md".to_string()));
        assert!(!paths.contains(&"noindex.md".to_string()));
        assert!(!paths.contains(&"skipped/a.md".to_string()));
        assert!(!paths.contains(&"nested/skipped-too/b.md".to_string()));
    }

    #[test]
    fn test_symlink_handling() {
        // This test only runs on Unix-like systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            let temp_dir = TempDir::new().unwrap();
            let root = temp_dir.path();

            // Create a file and a symlink to it
            fs::write(root.join("original.md"), "content").unwrap();
            symlink(root.join("original.md"), root.join("link.md")).unwrap();

            // Test with symlinks not followed (default)
            let config = FileWalkerConfig::new(root.to_path_buf());
            let walker = FileWalker::new(config);
            let files = walker.discover_files().unwrap();

            // Should only find original file
            assert_eq!(files.len(), 1);
            assert_eq!(files[0].relative_path, Path::new("original.md"));

            // Test with symlinks followed
            let config = FileWalkerConfig::new(root.to_path_buf()).with_follow_symlinks(true);
            let walker = FileWalker::new(config);
            let files = walker.discover_files().unwrap();

            // Should find both original and link
            assert_eq!(files.len(), 2);
        }
    }

    #[test]
    fn test_empty_directory() {
        let temp_dir = TempDir::new().unwrap();

        let config = FileWalkerConfig::new(temp_dir.path().to_path_buf());
        let walker = FileWalker::new(config);
        let files = walker.discover_files().unwrap();

        assert_eq!(files.len(), 0);
    }

    #[test]
    fn test_deeply_nested_structure() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create deeply nested structure
        let deep_path = root.join("a/b/c/d/e/f/g/h");
        fs::create_dir_all(&deep_path).unwrap();
        fs::write(deep_path.join("deep.md"), "deep content").unwrap();

        let config = FileWalkerConfig::new(root.to_path_buf());
        let walker = FileWalker::new(config);
        let files = walker.discover_files().unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, Path::new("a/b/c/d/e/f/g/h/deep.md"));
    }
}
