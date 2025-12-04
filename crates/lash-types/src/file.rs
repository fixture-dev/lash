//! Task file metadata and content hashing

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::config::LashConfig;
use crate::dependency::DependencyRef;
use crate::error::Result;
use crate::task::TaskTree;

/// A task file in the system
///
/// Represents a single Markdown file containing tasks. Each file has:
/// - A path relative to the project root
/// - A title (from the first H1 heading)
/// - An ID (explicit via @id or derived from path)
/// - File-level metadata
/// - Optional description section
/// - A hierarchical task tree
/// - Content hash for change detection
#[derive(Debug, Clone)]
pub struct TaskFile {
    /// Relative path from project root
    pub path: PathBuf,

    /// H1 title from file
    pub title: String,

    /// File identifier (from @id or derived from path)
    pub id: String,

    /// File-level metadata
    pub metadata: FileMetadata,

    /// Optional description section content (## Description)
    pub description: Option<String>,

    /// Agent notes extracted from description section
    pub description_agent_notes: Vec<String>,

    /// Hierarchical task structure
    pub tasks: TaskTree,

    /// Content hash (blake3)
    pub hash: String,

    /// Last modified time
    pub mtime: SystemTime,
}

impl TaskFile {
    /// Validate the task file
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Any task validation fails
    /// - Task IDs are not unique within the file
    /// - Dependency references have invalid syntax
    pub fn validate(&self, config: &LashConfig) -> Result<()> {
        // Validate all tasks in tree
        self.tasks.validate(config.max_depth)?;

        // Check for task ID uniqueness (already enforced by TaskTree)
        // Dependency reference syntax validation will be done in parsing phase

        Ok(())
    }

    /// Check if content hash matches the given content
    #[must_use]
    pub fn hash_matches(&self, content: &str) -> bool {
        self.hash == compute_hash(content)
    }

    /// Compute the overall status of this file
    ///
    /// Status is determined by examining top-level tasks:
    /// - `Complete` if all top-level tasks are complete
    /// - `InProgress` if any top-level task is open
    /// - `Blocked` if any top-level task is blocked
    /// - `Empty` if there are no tasks
    #[must_use]
    pub fn compute_status(&self) -> FileStatus {
        let top_level_tasks: Vec<_> = self
            .tasks
            .tasks()
            .iter()
            .filter(|t| t.parent_id.is_none())
            .collect();

        if top_level_tasks.is_empty() {
            return FileStatus::Empty;
        }

        let has_blocked = top_level_tasks
            .iter()
            .any(|t| t.status == crate::status::TaskStatus::Blocked);
        let has_open = top_level_tasks
            .iter()
            .any(|t| t.status == crate::status::TaskStatus::Open);
        let all_complete = top_level_tasks.iter().all(|t| t.is_complete());

        if has_blocked {
            FileStatus::Blocked
        } else if has_open {
            FileStatus::InProgress
        } else if all_complete {
            FileStatus::Complete
        } else {
            FileStatus::InProgress
        }
    }
}

/// File-level metadata
///
/// Metadata that applies to the entire task file, distinct from task-level metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileMetadata {
    /// File-level labels
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,

    /// Overall status annotation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// File owner
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,

    /// Creation date (YYYY-MM-DD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,

    /// File-level dependencies
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<DependencyRef>,

    /// Documentation references
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub docs: Vec<crate::dependency::DocRef>,

    /// Other annotations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub custom: HashMap<String, String>,
}

/// Overall status of a task file
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileStatus {
    /// All top-level tasks are complete
    Complete,
    /// At least one top-level task is in progress
    InProgress,
    /// At least one top-level task is blocked
    Blocked,
    /// No tasks in file
    Empty,
}

impl FileStatus {
    /// Convert status to string representation
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            FileStatus::Complete => "complete",
            FileStatus::InProgress => "in_progress",
            FileStatus::Blocked => "blocked",
            FileStatus::Empty => "empty",
        }
    }

    /// Parse status from string (defensive, returns default for unknown values)
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "complete" => FileStatus::Complete,
            "blocked" => FileStatus::Blocked,
            "empty" => FileStatus::Empty,
            _ => FileStatus::InProgress, // Default fallback for unknown or "in_progress"
        }
    }
}

/// Compute blake3 hash of content
///
/// Returns hex-encoded hash string for the given content.
///
/// # Examples
///
/// ```
/// use lash_types::file::compute_hash;
///
/// let hash1 = compute_hash("Hello, world!");
/// let hash2 = compute_hash("Hello, world!");
/// let hash3 = compute_hash("Different content");
///
/// assert_eq!(hash1, hash2);
/// assert_ne!(hash1, hash3);
/// ```
#[must_use]
pub fn compute_hash(content: &str) -> String {
    blake3::hash(content.as_bytes()).to_hex().to_string()
}

/// Synthesize a file ID from its path
///
/// Converts a file path to a dot-delimited identifier:
/// - `core/api/auth.md` → `core.api.auth`
/// - Strips `.md` extension
/// - Converts `/` to `.`
/// - Normalizes to lowercase
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use lash_types::file::synthesize_file_id;
///
/// assert_eq!(
///     synthesize_file_id(Path::new("core/api/auth.md")),
///     "core.api.auth"
/// );
/// assert_eq!(
///     synthesize_file_id(Path::new("tasks.md")),
///     "tasks"
/// );
/// assert_eq!(
///     synthesize_file_id(Path::new("Deep/Nested/Path.md")),
///     "deep.nested.path"
/// );
/// ```
#[must_use]
pub fn synthesize_file_id(path: &Path) -> String {
    let path_str = path.to_string_lossy();

    // Strip .md extension
    let without_ext = path_str.strip_suffix(".md").unwrap_or(&path_str);

    // Convert to lowercase and replace path separators with dots
    without_ext.to_lowercase().replace(['/', '\\'], ".")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigBuilder;
    use crate::status::TaskStatus;
    use crate::task::TaskBuilder;

    #[test]
    fn test_compute_hash() {
        let content1 = "Hello, world!";
        let content2 = "Hello, world!";
        let content3 = "Different content";

        let hash1 = compute_hash(content1);
        let hash2 = compute_hash(content2);
        let hash3 = compute_hash(content3);

        // Same content produces same hash
        assert_eq!(hash1, hash2);

        // Different content produces different hash
        assert_ne!(hash1, hash3);

        // Hash is hex string
        assert!(hash1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_hash_matches() {
        let content = "Test content";
        let hash = compute_hash(content);

        let file = TaskFile {
            path: PathBuf::from("test.md"),
            title: "Test".to_string(),
            id: "test".to_string(),
            metadata: FileMetadata::default(),
            tasks: TaskTree::new(),
            hash: hash.clone(),
            mtime: SystemTime::now(),
            description: None,
            description_agent_notes: Vec::new(),
        };

        assert!(file.hash_matches(content));
        assert!(!file.hash_matches("Different content"));
    }

    #[test]
    fn test_synthesize_file_id() {
        assert_eq!(
            synthesize_file_id(Path::new("core/api/auth.md")),
            "core.api.auth"
        );
        assert_eq!(synthesize_file_id(Path::new("tasks.md")), "tasks");
        assert_eq!(
            synthesize_file_id(Path::new("Deep/Nested/Path.md")),
            "deep.nested.path"
        );
        assert_eq!(synthesize_file_id(Path::new("simple.md")), "simple");
    }

    #[test]
    fn test_synthesize_file_id_no_extension() {
        assert_eq!(
            synthesize_file_id(Path::new("no-extension")),
            "no-extension"
        );
    }

    #[test]
    fn test_file_status_empty() {
        let file = TaskFile {
            path: PathBuf::from("test.md"),
            title: "Test".to_string(),
            id: "test".to_string(),
            metadata: FileMetadata::default(),
            tasks: TaskTree::new(),
            hash: compute_hash(""),
            mtime: SystemTime::now(),
            description: None,
            description_agent_notes: Vec::new(),
        };

        assert_eq!(file.compute_status(), FileStatus::Empty);
    }

    #[test]
    fn test_file_status_complete() {
        let mut tasks = TaskTree::new();
        tasks
            .add_task(
                TaskBuilder::new("Task 1")
                    .id("task-1")
                    .status(TaskStatus::Done)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        tasks
            .add_task(
                TaskBuilder::new("Task 2")
                    .id("task-2")
                    .status(TaskStatus::Waived)
                    .build()
                    .unwrap(),
            )
            .unwrap();

        let file = TaskFile {
            path: PathBuf::from("test.md"),
            title: "Test".to_string(),
            id: "test".to_string(),
            metadata: FileMetadata::default(),
            tasks,
            hash: compute_hash(""),
            mtime: SystemTime::now(),
            description: None,
            description_agent_notes: Vec::new(),
        };

        assert_eq!(file.compute_status(), FileStatus::Complete);
    }

    #[test]
    fn test_file_status_in_progress() {
        let mut tasks = TaskTree::new();
        tasks
            .add_task(
                TaskBuilder::new("Task 1")
                    .id("task-1")
                    .status(TaskStatus::Done)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        tasks
            .add_task(
                TaskBuilder::new("Task 2")
                    .id("task-2")
                    .status(TaskStatus::Open)
                    .build()
                    .unwrap(),
            )
            .unwrap();

        let file = TaskFile {
            path: PathBuf::from("test.md"),
            title: "Test".to_string(),
            id: "test".to_string(),
            metadata: FileMetadata::default(),
            tasks,
            hash: compute_hash(""),
            mtime: SystemTime::now(),
            description: None,
            description_agent_notes: Vec::new(),
        };

        assert_eq!(file.compute_status(), FileStatus::InProgress);
    }

    #[test]
    fn test_file_status_blocked() {
        let mut tasks = TaskTree::new();
        tasks
            .add_task(
                TaskBuilder::new("Task 1")
                    .id("task-1")
                    .status(TaskStatus::Blocked)
                    .build()
                    .unwrap(),
            )
            .unwrap();

        let file = TaskFile {
            path: PathBuf::from("test.md"),
            title: "Test".to_string(),
            id: "test".to_string(),
            metadata: FileMetadata::default(),
            tasks,
            hash: compute_hash(""),
            mtime: SystemTime::now(),
            description: None,
            description_agent_notes: Vec::new(),
        };

        assert_eq!(file.compute_status(), FileStatus::Blocked);
    }

    #[test]
    fn test_file_validate() {
        let config = ConfigBuilder::new().build().unwrap();

        let mut tasks = TaskTree::new();
        tasks
            .add_task(TaskBuilder::new("Task").id("task-1").build().unwrap())
            .unwrap();

        let file = TaskFile {
            path: PathBuf::from("test.md"),
            title: "Test".to_string(),
            id: "test".to_string(),
            metadata: FileMetadata::default(),
            tasks,
            hash: compute_hash(""),
            mtime: SystemTime::now(),
            description: None,
            description_agent_notes: Vec::new(),
        };

        assert!(file.validate(&config).is_ok());
    }

    #[test]
    fn test_file_metadata_serialization() {
        let metadata = FileMetadata {
            labels: vec!["label1".to_string(), "label2".to_string()],
            status: Some("in-progress".to_string()),
            owner: Some("alice".to_string()),
            created: Some("2024-01-01".to_string()),
            depends_on: vec![],
            docs: vec![],
            custom: HashMap::new(),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: FileMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.labels, metadata.labels);
        assert_eq!(deserialized.status, metadata.status);
        assert_eq!(deserialized.owner, metadata.owner);
        assert_eq!(deserialized.created, metadata.created);
    }
}
