//! Types for creating new tasks via CLI and TUI
//!
//! This module provides types that enable programmatic task creation,
//! supporting various positioning strategies and metadata options.

use std::path::PathBuf;

use crate::status::TaskStatus;

/// Where to create the task
///
/// Specifies the target file for task creation, supporting various
/// strategies from explicit paths to contextual placement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileTarget {
    /// Use the currently focused/active file
    Current,
    /// Explicit file path
    Path(PathBuf),
    /// File containing a specific task (by full reference)
    ContainingTask(String),
    /// Create a new file at path
    NewFile {
        /// Path where the new file should be created
        path: PathBuf,
        /// Optional title for the file header
        title: Option<String>,
        /// Optional description for the file
        description: Option<String>,
    },
}

/// How to identify the parent task
///
/// Specifies the parent-child relationship for the new task,
/// supporting both explicit references and implicit positioning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParentRef {
    /// Top-level task (no parent)
    None,
    /// Parent by task ID (within same file)
    Id(String),
    /// Full task reference (file path + task ID)
    FullRef(String),
    /// Auto-append at a specific depth level
    AppendAtDepth(u8),
}

/// Where to insert among siblings
///
/// Controls the ordering of the new task relative to its siblings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsertPosition {
    /// Append after last sibling
    Append,
    /// At specific index (0-based)
    AtIndex(usize),
    /// Before a specific task (by ID)
    Before(String),
    /// After a specific task (by ID)
    After(String),
}

/// Request to create a new task
///
/// Encapsulates all information needed to create a task, including
/// its content, location, and metadata.
///
/// # Examples
///
/// ```
/// use lash_types::creation::{TaskCreationRequest, FileTarget, ParentRef, InsertPosition};
/// use lash_types::TaskStatus;
/// use std::path::PathBuf;
///
/// let request = TaskCreationRequest {
///     title: "Implement feature X".to_string(),
///     file_target: FileTarget::Path(PathBuf::from("tasks.md")),
///     parent: ParentRef::None,
///     position: InsertPosition::Append,
///     status: Some(TaskStatus::Open),
///     id: Some("feature-x".to_string()),
///     labels: vec!["backend".to_string(), "api".to_string()],
///     owner: Some("alice".to_string()),
///     estimate: Some("2h".to_string()),
///     depends_on: vec![],
///     agent_note: None,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCreationRequest {
    /// Task title/description
    pub title: String,
    /// Where to create the task
    pub file_target: FileTarget,
    /// Parent task reference
    pub parent: ParentRef,
    /// Position among siblings
    pub position: InsertPosition,
    /// Optional explicit status (defaults to Open)
    pub status: Option<TaskStatus>,
    /// Optional explicit ID (will be synthesized if not provided)
    pub id: Option<String>,
    /// Labels to apply to the task
    pub labels: Vec<String>,
    /// Optional owner/assignee
    pub owner: Option<String>,
    /// Optional time estimate
    pub estimate: Option<String>,
    /// Explicit dependencies
    pub depends_on: Vec<String>,
    /// Optional note for AI agents
    pub agent_note: Option<String>,
}

/// Builder for `TaskCreationRequest` with fluent API
///
/// Provides a convenient way to construct task creation requests
/// with method chaining and sensible defaults.
///
/// # Examples
///
/// ```
/// use lash_types::creation::{TaskCreationRequestBuilder, FileTarget, ParentRef};
/// use lash_types::TaskStatus;
/// use std::path::PathBuf;
///
/// let request = TaskCreationRequestBuilder::new("Implement feature")
///     .file_path(PathBuf::from("tasks.md"))
///     .parent_id("parent-task")
///     .status(TaskStatus::Open)
///     .label("backend")
///     .label("api")
///     .owner("alice")
///     .estimate("2h")
///     .build();
///
/// assert_eq!(request.title, "Implement feature");
/// assert_eq!(request.labels, vec!["backend", "api"]);
/// ```
#[derive(Debug, Clone)]
pub struct TaskCreationRequestBuilder {
    title: String,
    file_target: FileTarget,
    parent: ParentRef,
    position: InsertPosition,
    status: Option<TaskStatus>,
    id: Option<String>,
    labels: Vec<String>,
    owner: Option<String>,
    estimate: Option<String>,
    depends_on: Vec<String>,
    agent_note: Option<String>,
}

impl TaskCreationRequestBuilder {
    /// Create a new builder with the given title
    ///
    /// The title is the only required field. All other fields have sensible defaults:
    /// - `file_target`: Current file
    /// - `parent`: None (top-level task)
    /// - `position`: Append
    /// - `status`: None (defaults to Open when created)
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            file_target: FileTarget::Current,
            parent: ParentRef::None,
            position: InsertPosition::Append,
            status: None,
            id: None,
            labels: Vec::new(),
            owner: None,
            estimate: None,
            depends_on: Vec::new(),
            agent_note: None,
        }
    }

    /// Set the file target to an explicit path
    #[must_use]
    pub fn file_path(mut self, path: PathBuf) -> Self {
        self.file_target = FileTarget::Path(path);
        self
    }

    /// Set the file target to the current file
    #[must_use]
    pub fn current_file(mut self) -> Self {
        self.file_target = FileTarget::Current;
        self
    }

    /// Set the file target to the file containing a specific task
    #[must_use]
    pub fn file_containing_task(mut self, task_ref: impl Into<String>) -> Self {
        self.file_target = FileTarget::ContainingTask(task_ref.into());
        self
    }

    /// Set the file target to a new file
    #[must_use]
    pub fn new_file(
        mut self,
        path: PathBuf,
        title: Option<String>,
        description: Option<String>,
    ) -> Self {
        self.file_target = FileTarget::NewFile {
            path,
            title,
            description,
        };
        self
    }

    /// Set parent by ID (within same file)
    #[must_use]
    pub fn parent_id(mut self, id: impl Into<String>) -> Self {
        self.parent = ParentRef::Id(id.into());
        self
    }

    /// Set parent by full reference (file path + task ID)
    #[must_use]
    pub fn parent_full_ref(mut self, full_ref: impl Into<String>) -> Self {
        self.parent = ParentRef::FullRef(full_ref.into());
        self
    }

    /// Set parent to append at a specific depth level
    #[must_use]
    pub fn parent_at_depth(mut self, depth: u8) -> Self {
        self.parent = ParentRef::AppendAtDepth(depth);
        self
    }

    /// Set parent to none (top-level task)
    #[must_use]
    pub fn no_parent(mut self) -> Self {
        self.parent = ParentRef::None;
        self
    }

    /// Set position to append
    #[must_use]
    pub fn append(mut self) -> Self {
        self.position = InsertPosition::Append;
        self
    }

    /// Set position to a specific index
    #[must_use]
    pub fn at_index(mut self, index: usize) -> Self {
        self.position = InsertPosition::AtIndex(index);
        self
    }

    /// Set position before a specific task
    #[must_use]
    pub fn before(mut self, task_id: impl Into<String>) -> Self {
        self.position = InsertPosition::Before(task_id.into());
        self
    }

    /// Set position after a specific task
    #[must_use]
    pub fn after(mut self, task_id: impl Into<String>) -> Self {
        self.position = InsertPosition::After(task_id.into());
        self
    }

    /// Set the task status
    #[must_use]
    pub fn status(mut self, status: TaskStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Set an explicit task ID
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Add a label to the task
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self
    }

    /// Set all labels at once
    #[must_use]
    pub fn labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }

    /// Set the owner/assignee
    #[must_use]
    pub fn owner(mut self, owner: impl Into<String>) -> Self {
        self.owner = Some(owner.into());
        self
    }

    /// Set the time estimate
    #[must_use]
    pub fn estimate(mut self, estimate: impl Into<String>) -> Self {
        self.estimate = Some(estimate.into());
        self
    }

    /// Add a dependency
    #[must_use]
    pub fn depends_on(mut self, dep: impl Into<String>) -> Self {
        self.depends_on.push(dep.into());
        self
    }

    /// Set all dependencies at once
    #[must_use]
    pub fn dependencies(mut self, deps: Vec<String>) -> Self {
        self.depends_on = deps;
        self
    }

    /// Set an agent note
    #[must_use]
    pub fn agent_note(mut self, note: impl Into<String>) -> Self {
        self.agent_note = Some(note.into());
        self
    }

    /// Build the `TaskCreationRequest`
    #[must_use]
    pub fn build(self) -> TaskCreationRequest {
        TaskCreationRequest {
            title: self.title,
            file_target: self.file_target,
            parent: self.parent,
            position: self.position,
            status: self.status,
            id: self.id,
            labels: self.labels,
            owner: self.owner,
            estimate: self.estimate,
            depends_on: self.depends_on,
            agent_note: self.agent_note,
        }
    }
}

/// Result of successful task creation
///
/// Contains information about the newly created task,
/// including its location and generated ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCreationResult {
    /// The task ID (synthesized if not explicitly provided)
    pub task_id: String,
    /// Path to the file containing the task
    pub file_path: PathBuf,
    /// Line number where the task was inserted
    pub line_number: usize,
    /// Whether a new file was created
    pub is_new_file: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let request = TaskCreationRequestBuilder::new("Test task").build();

        assert_eq!(request.title, "Test task");
        assert_eq!(request.file_target, FileTarget::Current);
        assert_eq!(request.parent, ParentRef::None);
        assert_eq!(request.position, InsertPosition::Append);
        assert_eq!(request.status, None);
        assert_eq!(request.id, None);
        assert!(request.labels.is_empty());
        assert_eq!(request.owner, None);
    }

    #[test]
    fn test_builder_with_metadata() {
        let request = TaskCreationRequestBuilder::new("Feature task")
            .file_path(PathBuf::from("tasks.md"))
            .parent_id("parent-1")
            .label("backend")
            .label("api")
            .owner("alice")
            .estimate("2h")
            .status(TaskStatus::Open)
            .build();

        assert_eq!(request.title, "Feature task");
        assert_eq!(
            request.file_target,
            FileTarget::Path(PathBuf::from("tasks.md"))
        );
        assert_eq!(request.parent, ParentRef::Id("parent-1".to_string()));
        assert_eq!(request.labels, vec!["backend", "api"]);
        assert_eq!(request.owner, Some("alice".to_string()));
        assert_eq!(request.estimate, Some("2h".to_string()));
        assert_eq!(request.status, Some(TaskStatus::Open));
    }

    #[test]
    fn test_builder_dependencies() {
        let request = TaskCreationRequestBuilder::new("Task")
            .depends_on("task-1")
            .depends_on("task-2")
            .build();

        assert_eq!(request.depends_on, vec!["task-1", "task-2"]);
    }

    #[test]
    fn test_builder_dependencies_bulk() {
        let request = TaskCreationRequestBuilder::new("Task")
            .dependencies(vec!["task-1".to_string(), "task-2".to_string()])
            .build();

        assert_eq!(request.depends_on, vec!["task-1", "task-2"]);
    }

    #[test]
    fn test_builder_labels_bulk() {
        let request = TaskCreationRequestBuilder::new("Task")
            .labels(vec!["label1".to_string(), "label2".to_string()])
            .build();

        assert_eq!(request.labels, vec!["label1", "label2"]);
    }

    #[test]
    fn test_builder_agent_note() {
        let request = TaskCreationRequestBuilder::new("Task")
            .agent_note("This is a note for the agent")
            .build();

        assert_eq!(
            request.agent_note,
            Some("This is a note for the agent".to_string())
        );
    }

    #[test]
    fn test_builder_new_file() {
        let request = TaskCreationRequestBuilder::new("Task")
            .new_file(
                PathBuf::from("new.md"),
                Some("New File".to_string()),
                Some("Description".to_string()),
            )
            .build();

        assert!(matches!(request.file_target, FileTarget::NewFile { .. }));
        if let FileTarget::NewFile {
            path,
            title,
            description,
        } = request.file_target
        {
            assert_eq!(path, PathBuf::from("new.md"));
            assert_eq!(title, Some("New File".to_string()));
            assert_eq!(description, Some("Description".to_string()));
        }
    }

    #[test]
    fn test_builder_current_file() {
        let request = TaskCreationRequestBuilder::new("Task")
            .file_path(PathBuf::from("other.md"))
            .current_file()
            .build();

        assert_eq!(request.file_target, FileTarget::Current);
    }

    #[test]
    fn test_builder_file_containing_task() {
        let request = TaskCreationRequestBuilder::new("Task")
            .file_containing_task("path/to/file.md#task:id")
            .build();

        assert_eq!(
            request.file_target,
            FileTarget::ContainingTask("path/to/file.md#task:id".to_string())
        );
    }

    #[test]
    fn test_builder_parent_full_ref() {
        let request = TaskCreationRequestBuilder::new("Task")
            .parent_full_ref("path/to/file.md#task:parent-id")
            .build();

        assert_eq!(
            request.parent,
            ParentRef::FullRef("path/to/file.md#task:parent-id".to_string())
        );
    }

    #[test]
    fn test_builder_parent_at_depth() {
        let request = TaskCreationRequestBuilder::new("Task")
            .parent_at_depth(2)
            .build();

        assert_eq!(request.parent, ParentRef::AppendAtDepth(2));
    }

    #[test]
    fn test_builder_no_parent() {
        let request = TaskCreationRequestBuilder::new("Task")
            .parent_id("some-parent")
            .no_parent()
            .build();

        assert_eq!(request.parent, ParentRef::None);
    }

    #[test]
    fn test_builder_position_at_index() {
        let request = TaskCreationRequestBuilder::new("Task").at_index(5).build();

        assert_eq!(request.position, InsertPosition::AtIndex(5));
    }

    #[test]
    fn test_builder_position_before() {
        let request = TaskCreationRequestBuilder::new("Task")
            .before("task-id")
            .build();

        assert_eq!(
            request.position,
            InsertPosition::Before("task-id".to_string())
        );
    }

    #[test]
    fn test_builder_position_after() {
        let request = TaskCreationRequestBuilder::new("Task")
            .after("task-id")
            .build();

        assert_eq!(
            request.position,
            InsertPosition::After("task-id".to_string())
        );
    }

    #[test]
    fn test_builder_explicit_id() {
        let request = TaskCreationRequestBuilder::new("Task")
            .id("custom-id")
            .build();

        assert_eq!(request.id, Some("custom-id".to_string()));
    }

    #[test]
    fn test_task_creation_result() {
        let result = TaskCreationResult {
            task_id: "task-1".to_string(),
            file_path: PathBuf::from("tasks.md"),
            line_number: 42,
            is_new_file: false,
        };

        assert_eq!(result.task_id, "task-1");
        assert_eq!(result.file_path, PathBuf::from("tasks.md"));
        assert_eq!(result.line_number, 42);
        assert!(!result.is_new_file);
    }

    #[test]
    fn test_file_target_equality() {
        assert_eq!(FileTarget::Current, FileTarget::Current);
        assert_eq!(
            FileTarget::Path(PathBuf::from("test.md")),
            FileTarget::Path(PathBuf::from("test.md"))
        );
        assert_ne!(
            FileTarget::Path(PathBuf::from("test.md")),
            FileTarget::Path(PathBuf::from("other.md"))
        );
    }

    #[test]
    fn test_parent_ref_equality() {
        assert_eq!(ParentRef::None, ParentRef::None);
        assert_eq!(
            ParentRef::Id("task-1".to_string()),
            ParentRef::Id("task-1".to_string())
        );
        assert_ne!(
            ParentRef::Id("task-1".to_string()),
            ParentRef::Id("task-2".to_string())
        );
    }

    #[test]
    fn test_insert_position_equality() {
        assert_eq!(InsertPosition::Append, InsertPosition::Append);
        assert_eq!(InsertPosition::AtIndex(5), InsertPosition::AtIndex(5));
        assert_ne!(InsertPosition::AtIndex(5), InsertPosition::AtIndex(6));
    }
}
