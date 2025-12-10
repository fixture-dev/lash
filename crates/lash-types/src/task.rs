//! Task data model and hierarchical task tree representation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::dependency::{Dependency, DependencyRef};
use crate::error::{codes, LashError, Result};
use crate::status::TaskStatus;

/// A single task in the system
///
/// Tasks are the fundamental unit of work tracking in Lash. Each task has:
/// - A unique ID within its file
/// - A title/description
/// - A status (open, done, waived, blocked)
/// - Optional metadata (labels, owner, estimates, etc.)
/// - Position information (depth, parent, order)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Task {
    /// Unique ID within the file (synthesized if not provided)
    pub id: String,

    /// Task title/description
    pub title: String,

    /// Current status
    pub status: TaskStatus,

    /// Nesting level (0 = top-level)
    pub depth: u8,

    /// Parent task ID (if nested)
    pub parent_id: Option<String>,

    /// Position among siblings (0-indexed)
    pub order_index: usize,

    /// Line number in the source file (1-indexed, 0 if unknown)
    #[serde(default)]
    pub line_number: usize,

    /// Optional metadata
    pub metadata: TaskMetadata,

    /// Extended description (optional)
    pub body: Option<String>,
}

impl Task {
    /// Validate the task according to the given constraints
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Depth exceeds `max_depth`
    /// - Title is empty
    /// - ID contains invalid characters
    pub fn validate(&self, max_depth: u8) -> Result<()> {
        // Check depth limit
        if self.depth > max_depth {
            return Err(LashError::Lint {
                code: codes::E_LINT_DEPTH_EXCEEDED,
                message: format!(
                    "Task depth {} exceeds maximum depth {}",
                    self.depth, max_depth
                ),
                location: None,
                snippet: None,
                help: Some(format!(
                    "flatten the hierarchy to {max_depth} levels or fewer"
                )),
            });
        }

        // Check title not empty
        if self.title.trim().is_empty() {
            return Err(LashError::Lint {
                code: codes::E_LINT_STATUS_INCONSISTENCY,
                message: "Task title cannot be empty".to_string(),
                location: None,
                snippet: None,
                help: Some("provide a non-empty title for the task".to_string()),
            });
        }

        // Check ID validity (alphanumeric, dash, underscore, colon)
        if !is_valid_id(&self.id) {
            return Err(LashError::Lint {
                code: codes::E_LINT_MISSING_ANNOTATION,
                message: format!("Invalid task ID: '{}'", self.id),
                location: None,
                snippet: None,
                help: Some("task IDs must contain only alphanumeric characters, dashes, underscores, and colons".to_string()),
            });
        }

        Ok(())
    }

    /// Check if this task is complete (delegates to status)
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.status.is_complete()
    }

    /// Check if this task is blocked by any dependencies
    ///
    /// A task is blocked if it has open dependencies (any dependency that is not complete).
    #[must_use]
    pub fn is_blocked(&self, deps: &[Dependency]) -> bool {
        // Check if any dependency pointing to this task has incomplete source
        deps.iter().any(|dep| {
            dep.from_task_id.ends_with(&format!("#{}", self.id)) && !self.status.is_complete()
        })
    }

    /// Check if this task is a child of the given parent ID
    #[must_use]
    pub fn is_child_of(&self, parent_id: &str) -> bool {
        self.parent_id.as_deref() == Some(parent_id)
    }

    /// Get depth relative to parent (how many levels deep from parent)
    #[must_use]
    pub fn depth_from_parent(&self) -> u8 {
        self.depth
    }
}

/// Validate task ID format
fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ':')
}

/// Metadata associated with a task
///
/// Task metadata includes optional annotations like labels, owner, estimates,
/// and dependencies. All fields are optional to support minimal task definitions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskMetadata {
    /// Labels (inline and explicit)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,

    /// Assignee/owner
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,

    /// Time estimate (e.g., "2h", "3d")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimate: Option<String>,

    /// Explicit dependencies
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<DependencyRef>,

    /// Documentation references
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub docs: Vec<crate::dependency::DocRef>,

    /// Note for AI agents
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_note: Option<String>,

    /// Custom fields for extensibility
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub custom: HashMap<String, String>,
}

/// Builder for constructing tasks with validation
///
/// # Examples
///
/// ```
/// use lash_types::task::{TaskBuilder, TaskMetadata};
/// use lash_types::TaskStatus;
///
/// let task = TaskBuilder::new("Implement feature")
///     .status(TaskStatus::Open)
///     .depth(1)
///     .parent("parent-task")
///     .label("backend")
///     .build()
///     .unwrap();
///
/// assert_eq!(task.title, "Implement feature");
/// assert_eq!(task.depth, 1);
/// assert_eq!(task.metadata.labels, vec!["backend"]);
/// ```
pub struct TaskBuilder {
    title: String,
    status: TaskStatus,
    depth: u8,
    parent_id: Option<String>,
    order_index: usize,
    line_number: usize,
    id: Option<String>,
    metadata: TaskMetadata,
    body: Option<String>,
}

impl TaskBuilder {
    /// Create a new task builder with the given title
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            status: TaskStatus::Open,
            depth: 0,
            parent_id: None,
            order_index: 0,
            line_number: 0,
            id: None,
            metadata: TaskMetadata::default(),
            body: None,
        }
    }

    /// Set the task status
    #[must_use]
    pub fn status(mut self, status: TaskStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the nesting depth
    #[must_use]
    pub fn depth(mut self, depth: u8) -> Self {
        self.depth = depth;
        self
    }

    /// Set the parent task ID
    #[must_use]
    pub fn parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    /// Set the order index
    #[must_use]
    pub fn order_index(mut self, index: usize) -> Self {
        self.order_index = index;
        self
    }

    /// Set the line number in the source file
    #[must_use]
    pub fn line_number(mut self, line_number: usize) -> Self {
        self.line_number = line_number;
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
        self.metadata.labels.push(label.into());
        self
    }

    /// Set the owner
    #[must_use]
    pub fn owner(mut self, owner: impl Into<String>) -> Self {
        self.metadata.owner = Some(owner.into());
        self
    }

    /// Set the estimate
    #[must_use]
    pub fn estimate(mut self, estimate: impl Into<String>) -> Self {
        self.metadata.estimate = Some(estimate.into());
        self
    }

    /// Set the body text
    #[must_use]
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Build the task, validating and synthesizing ID if needed
    ///
    /// # Errors
    ///
    /// Returns error if validation fails
    pub fn build(self) -> Result<Task> {
        // Synthesize ID if not provided
        let id = self
            .id
            .unwrap_or_else(|| format!("task-{}", self.order_index));

        let task = Task {
            id,
            title: self.title,
            status: self.status,
            depth: self.depth,
            parent_id: self.parent_id,
            order_index: self.order_index,
            line_number: self.line_number,
            metadata: self.metadata,
            body: self.body,
        };

        // Validate with reasonable max depth
        task.validate(10)?;

        Ok(task)
    }
}

/// Hierarchical task structure
///
/// `TaskTree` maintains a flat, indexed representation of tasks with parent-child
/// relationships tracked via IDs. This design supports efficient storage and querying
/// while preserving the hierarchical structure.
#[derive(Debug, Clone, Default)]
pub struct TaskTree {
    /// Flat list of all tasks
    tasks: Vec<Task>,
    /// Map from task ID to index in tasks vec
    id_to_index: HashMap<String, usize>,
}

impl TaskTree {
    /// Create a new empty task tree
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a task to the tree
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Task ID already exists in the tree
    /// - Task validation fails
    pub fn add_task(&mut self, task: Task) -> Result<()> {
        // Check for duplicate ID
        if self.id_to_index.contains_key(&task.id) {
            return Err(LashError::Lint {
                code: codes::E_LINT_DUPLICATE_ID,
                message: format!("Duplicate task ID: '{}'", task.id),
                location: None,
                snippet: None,
                help: Some("task IDs must be unique within a file".to_string()),
            });
        }

        // Validate task
        task.validate(10)?;

        // Add to tree
        let index = self.tasks.len();
        self.id_to_index.insert(task.id.clone(), index);
        self.tasks.push(task);

        Ok(())
    }

    /// Get a task by ID
    #[must_use]
    pub fn get_task(&self, id: &str) -> Option<&Task> {
        self.id_to_index.get(id).map(|&idx| &self.tasks[idx])
    }

    /// Get a mutable task by ID
    #[must_use]
    pub fn get_task_mut(&mut self, id: &str) -> Option<&mut Task> {
        self.id_to_index.get(id).map(|&idx| &mut self.tasks[idx])
    }

    /// Get all children of a parent task
    #[must_use]
    pub fn get_children(&self, parent_id: &str) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|t| t.is_child_of(parent_id))
            .collect()
    }

    /// Get all descendants of a task (recursive)
    #[must_use]
    pub fn get_descendants(&self, id: &str) -> Vec<&Task> {
        let mut descendants = Vec::new();
        let mut queue: Vec<&str> = vec![id];

        while let Some(current_id) = queue.pop() {
            for child in self.get_children(current_id) {
                descendants.push(child);
                queue.push(&child.id);
            }
        }

        descendants
    }

    /// Validate the entire task tree
    ///
    /// # Errors
    ///
    /// Returns error if any task validation fails or if there are structural issues
    pub fn validate(&self, max_depth: u8) -> Result<()> {
        for task in &self.tasks {
            task.validate(max_depth)?;

            // Check parent exists if specified
            if let Some(ref parent_id) = task.parent_id {
                if !self.id_to_index.contains_key(parent_id) {
                    return Err(LashError::Lint {
                        code: codes::E_LINT_MISSING_ANNOTATION,
                        message: format!(
                            "Task '{}' references non-existent parent '{}'",
                            task.id, parent_id
                        ),
                        location: None,
                        snippet: None,
                        help: Some(format!("ensure parent task '{parent_id}' exists")),
                    });
                }
            }
        }

        Ok(())
    }

    /// Get all tasks in the tree
    #[must_use]
    pub fn tasks(&self) -> &[Task] {
        &self.tasks
    }

    /// Get the number of tasks in the tree
    #[must_use]
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Check if the tree is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_builder_basic() {
        let task = TaskBuilder::new("Test task")
            .status(TaskStatus::Done)
            .build()
            .unwrap();

        assert_eq!(task.title, "Test task");
        assert_eq!(task.status, TaskStatus::Done);
        assert_eq!(task.depth, 0);
        assert!(task.parent_id.is_none());
    }

    #[test]
    fn test_task_builder_with_metadata() {
        let task = TaskBuilder::new("Feature task")
            .depth(1)
            .parent("parent-1")
            .label("backend")
            .label("api")
            .owner("alice")
            .estimate("2h")
            .build()
            .unwrap();

        assert_eq!(task.depth, 1);
        assert_eq!(task.parent_id.as_deref(), Some("parent-1"));
        assert_eq!(task.metadata.labels, vec!["backend", "api"]);
        assert_eq!(task.metadata.owner.as_deref(), Some("alice"));
        assert_eq!(task.metadata.estimate.as_deref(), Some("2h"));
    }

    #[test]
    fn test_task_builder_id_synthesis() {
        let task = TaskBuilder::new("Task").order_index(5).build().unwrap();

        assert_eq!(task.id, "task-5");
    }

    #[test]
    fn test_task_builder_explicit_id() {
        let task = TaskBuilder::new("Task").id("custom-id").build().unwrap();

        assert_eq!(task.id, "custom-id");
    }

    #[test]
    fn test_task_validation_depth() {
        let task = TaskBuilder::new("Deep task").depth(15).build().unwrap_err();

        assert!(matches!(task, LashError::Lint { .. }));
    }

    #[test]
    fn test_task_validation_empty_title() {
        let mut task = TaskBuilder::new("Valid").build().unwrap();
        task.title = "   ".to_string();

        assert!(task.validate(10).is_err());
    }

    #[test]
    fn test_task_is_complete() {
        let done = TaskBuilder::new("Done")
            .status(TaskStatus::Done)
            .build()
            .unwrap();
        assert!(done.is_complete());

        let waived = TaskBuilder::new("Waived")
            .status(TaskStatus::Waived)
            .build()
            .unwrap();
        assert!(waived.is_complete());

        let open = TaskBuilder::new("Open").build().unwrap();
        assert!(!open.is_complete());
    }

    #[test]
    fn test_task_is_child_of() {
        let child = TaskBuilder::new("Child")
            .parent("parent-1")
            .build()
            .unwrap();

        assert!(child.is_child_of("parent-1"));
        assert!(!child.is_child_of("other-parent"));
    }

    #[test]
    fn test_task_tree_add() {
        let mut tree = TaskTree::new();

        let task1 = TaskBuilder::new("Task 1").id("task-1").build().unwrap();
        let task2 = TaskBuilder::new("Task 2").id("task-2").build().unwrap();

        tree.add_task(task1).unwrap();
        tree.add_task(task2).unwrap();

        assert_eq!(tree.len(), 2);
    }

    #[test]
    fn test_task_tree_duplicate_id() {
        let mut tree = TaskTree::new();

        let task1 = TaskBuilder::new("Task 1").id("same-id").build().unwrap();
        let task2 = TaskBuilder::new("Task 2").id("same-id").build().unwrap();

        tree.add_task(task1).unwrap();
        let err = tree.add_task(task2).unwrap_err();

        assert!(matches!(err, LashError::Lint { .. }));
    }

    #[test]
    fn test_task_tree_get_task() {
        let mut tree = TaskTree::new();

        let task = TaskBuilder::new("Task").id("find-me").build().unwrap();
        tree.add_task(task).unwrap();

        let found = tree.get_task("find-me");
        assert!(found.is_some());
        assert_eq!(found.unwrap().title, "Task");

        let not_found = tree.get_task("missing");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_task_tree_get_children() {
        let mut tree = TaskTree::new();

        let parent = TaskBuilder::new("Parent").id("parent").build().unwrap();
        let child1 = TaskBuilder::new("Child 1")
            .id("child-1")
            .parent("parent")
            .build()
            .unwrap();
        let child2 = TaskBuilder::new("Child 2")
            .id("child-2")
            .parent("parent")
            .build()
            .unwrap();
        let other = TaskBuilder::new("Other").id("other").build().unwrap();

        tree.add_task(parent).unwrap();
        tree.add_task(child1).unwrap();
        tree.add_task(child2).unwrap();
        tree.add_task(other).unwrap();

        let children = tree.get_children("parent");
        assert_eq!(children.len(), 2);
        assert!(children.iter().any(|t| t.id == "child-1"));
        assert!(children.iter().any(|t| t.id == "child-2"));
    }

    #[test]
    fn test_task_tree_get_descendants() {
        let mut tree = TaskTree::new();

        // Build a tree:
        //   parent
        //   ├── child1
        //   │   └── grandchild1
        //   └── child2

        tree.add_task(TaskBuilder::new("Parent").id("parent").build().unwrap())
            .unwrap();
        tree.add_task(
            TaskBuilder::new("Child 1")
                .id("child1")
                .parent("parent")
                .build()
                .unwrap(),
        )
        .unwrap();
        tree.add_task(
            TaskBuilder::new("Child 2")
                .id("child2")
                .parent("parent")
                .build()
                .unwrap(),
        )
        .unwrap();
        tree.add_task(
            TaskBuilder::new("Grandchild 1")
                .id("grandchild1")
                .parent("child1")
                .build()
                .unwrap(),
        )
        .unwrap();

        let descendants = tree.get_descendants("parent");
        assert_eq!(descendants.len(), 3);
        assert!(descendants.iter().any(|t| t.id == "child1"));
        assert!(descendants.iter().any(|t| t.id == "child2"));
        assert!(descendants.iter().any(|t| t.id == "grandchild1"));
    }

    #[test]
    fn test_task_tree_validate_missing_parent() {
        let mut tree = TaskTree::new();

        let task = TaskBuilder::new("Child")
            .id("child")
            .parent("missing-parent")
            .build()
            .unwrap();

        tree.add_task(task).unwrap();

        let err = tree.validate(10).unwrap_err();
        assert!(matches!(err, LashError::Lint { .. }));
    }

    #[test]
    fn test_task_tree_validate_success() {
        let mut tree = TaskTree::new();

        tree.add_task(TaskBuilder::new("Parent").id("parent").build().unwrap())
            .unwrap();
        tree.add_task(
            TaskBuilder::new("Child")
                .id("child")
                .parent("parent")
                .build()
                .unwrap(),
        )
        .unwrap();

        assert!(tree.validate(10).is_ok());
    }

    #[test]
    fn test_is_valid_id() {
        assert!(is_valid_id("task-1"));
        assert!(is_valid_id("feature:auth"));
        assert!(is_valid_id("test_case_123"));
        assert!(is_valid_id("simple"));

        assert!(!is_valid_id(""));
        assert!(!is_valid_id("has space"));
        assert!(!is_valid_id("has@symbol"));
    }
}
