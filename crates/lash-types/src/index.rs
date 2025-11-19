//! Root index model and utilities

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::error::{codes, LashError, Result};
use crate::status::TaskStatus;

/// Root index file for a Lash project
///
/// The root index serves as the entry point for the project, listing all
/// task files and their organization.
#[derive(Debug, Clone)]
pub struct RootIndex {
    /// Path to the index file
    pub path: PathBuf,

    /// Project title (from H1)
    pub title: String,

    /// Project-level metadata
    pub metadata: IndexMetadata,

    /// File references
    pub entries: Vec<IndexEntry>,
}

impl RootIndex {
    /// Create a new root index
    #[must_use]
    pub fn new(path: PathBuf, title: String) -> Self {
        Self {
            path,
            title,
            metadata: IndexMetadata::default(),
            entries: Vec::new(),
        }
    }

    /// Validate the root index
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Referenced paths don't exist
    /// - There are duplicate paths
    /// - Paths are outside project root
    pub fn validate(&self, project_root: &Path) -> Result<()> {
        let mut seen_paths = HashSet::new();
        let mut errors = Vec::new();

        for entry in &self.entries {
            // Check for duplicates
            if !seen_paths.insert(&entry.path) {
                errors.push(format!("Duplicate path in index: {}", entry.path.display()));
            }

            // Check path is within project root
            let full_path = project_root.join(&entry.path);
            if let Ok(canonical_full) = full_path.canonicalize() {
                if let Ok(canonical_root) = project_root.canonicalize() {
                    if !canonical_full.starts_with(&canonical_root) {
                        errors.push(format!(
                            "Path outside project root: {}",
                            entry.path.display()
                        ));
                    }
                }
            }

            // Check path exists (only warn if it doesn't, as files may not be created yet)
            if !full_path.exists() {
                errors.push(format!(
                    "Referenced path does not exist: {}",
                    entry.path.display()
                ));
            }
        }

        if !errors.is_empty() {
            return Err(LashError::Lint {
                code: codes::E_LINT_STATUS_INCONSISTENCY,
                message: format!("Index validation failed:\n  - {}", errors.join("\n  - ")),
                location: None,
                snippet: None,
                help: Some("fix the validation errors listed above".to_string()),
            });
        }

        Ok(())
    }

    /// Iterate over all entries
    pub fn iter_entries(&self) -> impl Iterator<Item = &IndexEntry> {
        self.entries.iter()
    }

    /// Get an entry by path
    #[must_use]
    pub fn get_entry(&self, path: &Path) -> Option<&IndexEntry> {
        self.entries.iter().find(|e| e.path == path)
    }

    /// Get all entries in a specific category
    #[must_use]
    pub fn get_category_entries(&self, category: &str) -> Vec<&IndexEntry> {
        self.entries
            .iter()
            .filter(|e| e.category.as_deref() == Some(category))
            .collect()
    }

    /// Add an entry to the index
    pub fn add_entry(&mut self, entry: IndexEntry) {
        self.entries.push(entry);
    }
}

/// Metadata for the root index
///
/// Contains project-level configuration and metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexMetadata {
    /// Project name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,

    /// Version string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Global labels
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,

    /// Custom metadata fields
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub custom: HashMap<String, String>,
}

/// Entry in the root index
///
/// Represents a reference to a task file from the root index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    /// Relative path to task file
    pub path: PathBuf,

    /// Entry status (from checkbox)
    pub status: TaskStatus,

    /// Optional title override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Optional category grouping (from H2 section)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

impl IndexEntry {
    /// Create a new index entry
    #[must_use]
    pub fn new(path: PathBuf, status: TaskStatus) -> Self {
        Self {
            path,
            status,
            title: None,
            category: None,
        }
    }

    /// Set the title
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the category
    #[must_use]
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }
}

/// Find the index file in a directory
///
/// Looks for `lash.index.md` first, then falls back to `index.lash.md`.
///
/// # Examples
///
/// ```no_run
/// use lash_types::index::find_index_file;
/// use std::path::Path;
///
/// let index = find_index_file(Path::new(".")).unwrap();
/// println!("Found index at: {}", index.display());
/// ```
///
/// # Errors
///
/// Returns error if neither index file is found
pub fn find_index_file(dir: &Path) -> Result<PathBuf> {
    // Try lash.index.md first
    let primary = dir.join("lash.index.md");
    if primary.exists() {
        return Ok(primary);
    }

    // Fall back to index.lash.md
    let fallback = dir.join("index.lash.md");
    if fallback.exists() {
        return Ok(fallback);
    }

    Err(LashError::Config {
        code: codes::E_CONFIG_MISSING_INDEX,
        message: format!(
            "No index file found in {} (looked for lash.index.md and index.lash.md)",
            dir.display()
        ),
        path: Some(dir.to_path_buf()),
        help: Some("create an index file at the root of your project".to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_root_index_new() {
        let index = RootIndex::new(PathBuf::from("lash.index.md"), "Test Project".to_string());

        assert_eq!(index.path, PathBuf::from("lash.index.md"));
        assert_eq!(index.title, "Test Project");
        assert_eq!(index.entries.len(), 0);
    }

    #[test]
    fn test_index_entry_new() {
        let entry = IndexEntry::new(PathBuf::from("tasks.md"), TaskStatus::Open);

        assert_eq!(entry.path, PathBuf::from("tasks.md"));
        assert_eq!(entry.status, TaskStatus::Open);
        assert!(entry.title.is_none());
        assert!(entry.category.is_none());
    }

    #[test]
    fn test_index_entry_builder() {
        let entry = IndexEntry::new(PathBuf::from("tasks.md"), TaskStatus::Done)
            .with_title("My Tasks")
            .with_category("Development");

        assert_eq!(entry.title.as_deref(), Some("My Tasks"));
        assert_eq!(entry.category.as_deref(), Some("Development"));
    }

    #[test]
    fn test_add_entry() {
        let mut index = RootIndex::new(PathBuf::from("index.md"), "Project".to_string());

        let entry = IndexEntry::new(PathBuf::from("tasks.md"), TaskStatus::Open);
        index.add_entry(entry);

        assert_eq!(index.entries.len(), 1);
    }

    #[test]
    fn test_iter_entries() {
        let mut index = RootIndex::new(PathBuf::from("index.md"), "Project".to_string());

        index.add_entry(IndexEntry::new(PathBuf::from("a.md"), TaskStatus::Open));
        index.add_entry(IndexEntry::new(PathBuf::from("b.md"), TaskStatus::Done));

        let count = index.iter_entries().count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_get_entry() {
        let mut index = RootIndex::new(PathBuf::from("index.md"), "Project".to_string());

        index.add_entry(IndexEntry::new(PathBuf::from("tasks.md"), TaskStatus::Open));

        let found = index.get_entry(Path::new("tasks.md"));
        assert!(found.is_some());

        let not_found = index.get_entry(Path::new("missing.md"));
        assert!(not_found.is_none());
    }

    #[test]
    fn test_get_category_entries() {
        let mut index = RootIndex::new(PathBuf::from("index.md"), "Project".to_string());

        index.add_entry(
            IndexEntry::new(PathBuf::from("a.md"), TaskStatus::Open).with_category("Dev"),
        );
        index.add_entry(
            IndexEntry::new(PathBuf::from("b.md"), TaskStatus::Done).with_category("Dev"),
        );
        index.add_entry(
            IndexEntry::new(PathBuf::from("c.md"), TaskStatus::Open).with_category("Docs"),
        );

        let dev_entries = index.get_category_entries("Dev");
        assert_eq!(dev_entries.len(), 2);

        let docs_entries = index.get_category_entries("Docs");
        assert_eq!(docs_entries.len(), 1);
    }

    #[test]
    fn test_validate_duplicate_paths() {
        let temp_dir = TempDir::new().unwrap();
        let mut index = RootIndex::new(temp_dir.path().join("index.md"), "Project".to_string());

        // Create a file
        let file_path = temp_dir.path().join("tasks.md");
        fs::write(&file_path, "# Tasks\n").unwrap();

        // Add duplicate entries
        index.add_entry(IndexEntry::new(PathBuf::from("tasks.md"), TaskStatus::Open));
        index.add_entry(IndexEntry::new(PathBuf::from("tasks.md"), TaskStatus::Done));

        let result = index.validate(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let mut index = RootIndex::new(temp_dir.path().join("index.md"), "Project".to_string());

        index.add_entry(IndexEntry::new(
            PathBuf::from("missing.md"),
            TaskStatus::Open,
        ));

        let result = index.validate(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_success() {
        let temp_dir = TempDir::new().unwrap();
        let mut index = RootIndex::new(temp_dir.path().join("index.md"), "Project".to_string());

        // Create files
        let file1 = temp_dir.path().join("tasks.md");
        let file2 = temp_dir.path().join("notes.md");
        fs::write(&file1, "# Tasks\n").unwrap();
        fs::write(&file2, "# Notes\n").unwrap();

        index.add_entry(IndexEntry::new(PathBuf::from("tasks.md"), TaskStatus::Open));
        index.add_entry(IndexEntry::new(PathBuf::from("notes.md"), TaskStatus::Done));

        let result = index.validate(temp_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_find_index_file_primary() {
        let temp_dir = TempDir::new().unwrap();

        // Create primary index file
        let primary = temp_dir.path().join("lash.index.md");
        fs::write(&primary, "# Index\n").unwrap();

        let found = find_index_file(temp_dir.path()).unwrap();
        assert_eq!(found, primary);
    }

    #[test]
    fn test_find_index_file_fallback() {
        let temp_dir = TempDir::new().unwrap();

        // Create fallback index file
        let fallback = temp_dir.path().join("index.lash.md");
        fs::write(&fallback, "# Index\n").unwrap();

        let found = find_index_file(temp_dir.path()).unwrap();
        assert_eq!(found, fallback);
    }

    #[test]
    fn test_find_index_file_not_found() {
        let temp_dir = TempDir::new().unwrap();

        let result = find_index_file(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_index_metadata_serialization() {
        let metadata = IndexMetadata {
            project: Some("Lash".to_string()),
            version: Some("1.0.0".to_string()),
            labels: vec!["project".to_string(), "task-tracker".to_string()],
            custom: HashMap::new(),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: IndexMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.project, metadata.project);
        assert_eq!(deserialized.version, metadata.version);
        assert_eq!(deserialized.labels, metadata.labels);
    }

    #[test]
    fn test_index_entry_serialization() {
        let entry = IndexEntry::new(PathBuf::from("tasks.md"), TaskStatus::Open)
            .with_title("My Tasks")
            .with_category("Development");

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: IndexEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.path, entry.path);
        assert_eq!(deserialized.status, entry.status);
        assert_eq!(deserialized.title, entry.title);
        assert_eq!(deserialized.category, entry.category);
    }
}
