//! Task data model and hierarchical task tree representation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::dependency::{Dependency, DependencyRef};
use crate::error::{codes, LashError, Result};
use crate::status::TaskStatus;

/// Warning threshold for contextual note length (in characters).
/// Notes exceeding this length will trigger a warning during linting.
pub const NOTE_LENGTH_WARNING_THRESHOLD: usize = 200;

/// Error threshold for contextual note length (in characters).
/// Notes exceeding this length will trigger an error during linting.
pub const NOTE_LENGTH_ERROR_THRESHOLD: usize = 500;

/// A contextual note attached to a task.
///
/// Contextual notes are plain bullet points (without checkboxes) that provide
/// additional context, requirements, or acceptance criteria for a task. Unlike
/// tasks, they are informational only and don't track completion status.
///
/// # Examples
///
/// ```
/// use lash_types::task::ContextualNote;
///
/// let note = ContextualNote::new("Use library X for parsing", 42);
/// assert_eq!(note.text(), "Use library X for parsing");
/// assert_eq!(note.line_number(), 42);
/// assert!(note.validate().is_ok());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextualNote {
    /// The text content of the note
    text: String,
    /// Line number in the source file (1-indexed)
    line_number: usize,
}

impl ContextualNote {
    /// Create a new contextual note.
    ///
    /// # Arguments
    ///
    /// * `text` - The text content of the note
    /// * `line_number` - The line number in the source file (1-indexed)
    #[must_use]
    pub fn new(text: impl Into<String>, line_number: usize) -> Self {
        Self {
            text: text.into(),
            line_number,
        }
    }

    /// Get the text content of the note.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get the line number where this note appears in the source file.
    #[must_use]
    pub fn line_number(&self) -> usize {
        self.line_number
    }

    /// Get the length of the note text in characters.
    #[must_use]
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// Check if the note text is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Validate the contextual note.
    ///
    /// Returns `Ok(())` if the note is valid. Returns an error if the note
    /// exceeds the maximum length threshold (500 characters).
    ///
    /// # Errors
    ///
    /// Returns `LashError::Lint` if the note text exceeds the error threshold.
    pub fn validate(&self) -> Result<()> {
        if self.text.len() > NOTE_LENGTH_ERROR_THRESHOLD {
            return Err(LashError::Lint {
                code: codes::E_LINT_BAD_INDENTATION, // Reusing existing code for format issues
                message: format!(
                    "Contextual note exceeds maximum length: {} characters (max {}) at line {}",
                    self.text.len(),
                    NOTE_LENGTH_ERROR_THRESHOLD,
                    self.line_number
                ),
                location: None, // File path not known at this level
                snippet: Some(self.truncated_text(50)),
                help: Some(format!(
                    "shorten the note to {} characters or fewer",
                    NOTE_LENGTH_ERROR_THRESHOLD
                )),
            });
        }
        Ok(())
    }

    /// Check if the note exceeds the warning threshold.
    ///
    /// Returns `true` if the note length is above 200 characters but at or below
    /// the error threshold. Use this in linting rules to emit warnings.
    #[must_use]
    pub fn exceeds_warning_threshold(&self) -> bool {
        self.text.len() > NOTE_LENGTH_WARNING_THRESHOLD
            && self.text.len() <= NOTE_LENGTH_ERROR_THRESHOLD
    }

    /// Check if the note exceeds the error threshold.
    ///
    /// Returns `true` if the note length is above 500 characters.
    #[must_use]
    pub fn exceeds_error_threshold(&self) -> bool {
        self.text.len() > NOTE_LENGTH_ERROR_THRESHOLD
    }

    /// Get a truncated version of the text for display purposes.
    ///
    /// # Arguments
    ///
    /// * `max_len` - Maximum length before truncation (including ellipsis)
    #[must_use]
    pub fn truncated_text(&self, max_len: usize) -> String {
        if self.text.len() <= max_len {
            self.text.clone()
        } else if max_len <= 3 {
            "...".to_string()
        } else {
            format!("{}...", &self.text[..max_len - 3])
        }
    }
}

impl From<&str> for ContextualNote {
    fn from(text: &str) -> Self {
        Self::new(text, 0)
    }
}

impl From<String> for ContextualNote {
    fn from(text: String) -> Self {
        Self {
            text,
            line_number: 0,
        }
    }
}

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

    /// Whether the task has an explicit @id annotation (vs synthesized ID)
    #[serde(default)]
    pub has_explicit_id: bool,

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

    /// Contextual notes (plain bullet points nested under this task)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contextual_notes: Vec<ContextualNote>,
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
    has_explicit_id: bool,
    metadata: TaskMetadata,
    body: Option<String>,
    contextual_notes: Vec<ContextualNote>,
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
            has_explicit_id: false,
            metadata: TaskMetadata::default(),
            body: None,
            contextual_notes: Vec::new(),
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
    ///
    /// This marks the task as having an explicit ID (vs synthesized),
    /// which will be preserved during formatting.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self.has_explicit_id = true;
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

    /// Add a contextual note to the task.
    ///
    /// This method accepts any type that can be converted into a `ContextualNote`,
    /// including `&str` and `String` (with line number defaulting to 0).
    #[must_use]
    pub fn contextual_note(mut self, note: impl Into<ContextualNote>) -> Self {
        self.contextual_notes.push(note.into());
        self
    }

    /// Add a contextual note with a specific line number.
    ///
    /// Use this method when you know the source line number of the note.
    #[must_use]
    pub fn contextual_note_with_line(
        mut self,
        text: impl Into<String>,
        line_number: usize,
    ) -> Self {
        self.contextual_notes
            .push(ContextualNote::new(text, line_number));
        self
    }

    /// Set all contextual notes at once.
    #[must_use]
    pub fn contextual_notes(mut self, notes: Vec<ContextualNote>) -> Self {
        self.contextual_notes = notes;
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
            has_explicit_id: self.has_explicit_id,
            title: self.title,
            status: self.status,
            depth: self.depth,
            parent_id: self.parent_id,
            order_index: self.order_index,
            line_number: self.line_number,
            metadata: self.metadata,
            body: self.body,
            contextual_notes: self.contextual_notes,
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
    fn test_task_builder_contextual_notes() {
        let task = TaskBuilder::new("Implement feature")
            .contextual_note("Use library X for parsing")
            .contextual_note("Target < 100ms latency")
            .build()
            .unwrap();

        assert_eq!(task.contextual_notes.len(), 2);
        assert_eq!(task.contextual_notes[0].text(), "Use library X for parsing");
        assert_eq!(task.contextual_notes[1].text(), "Target < 100ms latency");
    }

    #[test]
    fn test_task_builder_contextual_notes_with_line_number() {
        let task = TaskBuilder::new("Task")
            .contextual_note_with_line("Note with line", 42)
            .build()
            .unwrap();

        assert_eq!(task.contextual_notes.len(), 1);
        assert_eq!(task.contextual_notes[0].text(), "Note with line");
        assert_eq!(task.contextual_notes[0].line_number(), 42);
    }

    #[test]
    fn test_task_builder_contextual_notes_bulk() {
        let notes = vec![
            ContextualNote::new("Note 1", 10),
            ContextualNote::new("Note 2", 11),
            ContextualNote::new("Note 3", 12),
        ];
        let task = TaskBuilder::new("Task")
            .contextual_notes(notes.clone())
            .build()
            .unwrap();

        assert_eq!(task.contextual_notes, notes);
    }

    #[test]
    fn test_task_contextual_notes_serialization() {
        let task = TaskBuilder::new("Task")
            .id("test-task")
            .contextual_note("First note")
            .contextual_note("Second note")
            .build()
            .unwrap();

        // Serialize to JSON
        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains("contextual_notes"));
        assert!(json.contains("First note"));

        // Deserialize back
        let deserialized: Task = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.contextual_notes, task.contextual_notes);
    }

    #[test]
    fn test_task_empty_contextual_notes_not_serialized() {
        let task = TaskBuilder::new("Task").id("test-task").build().unwrap();

        // Empty notes should not appear in JSON (skip_serializing_if)
        let json = serde_json::to_string(&task).unwrap();
        assert!(!json.contains("contextual_notes"));
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

    // ===== ContextualNote Tests =====

    #[test]
    fn test_contextual_note_new() {
        let note = ContextualNote::new("Test note", 42);
        assert_eq!(note.text(), "Test note");
        assert_eq!(note.line_number(), 42);
    }

    #[test]
    fn test_contextual_note_len() {
        let note = ContextualNote::new("Hello", 1);
        assert_eq!(note.len(), 5);
        assert!(!note.is_empty());

        let empty = ContextualNote::new("", 1);
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
    }

    #[test]
    fn test_contextual_note_from_str() {
        let note: ContextualNote = "From string".into();
        assert_eq!(note.text(), "From string");
        assert_eq!(note.line_number(), 0);
    }

    #[test]
    fn test_contextual_note_from_string() {
        let note: ContextualNote = String::from("From owned string").into();
        assert_eq!(note.text(), "From owned string");
        assert_eq!(note.line_number(), 0);
    }

    #[test]
    fn test_contextual_note_default() {
        let note = ContextualNote::default();
        assert_eq!(note.text(), "");
        assert_eq!(note.line_number(), 0);
    }

    #[test]
    fn test_contextual_note_validate_ok() {
        let note = ContextualNote::new("Short note", 1);
        assert!(note.validate().is_ok());
    }

    #[test]
    fn test_contextual_note_validate_at_warning_threshold() {
        // Exactly at warning threshold (200) - should validate OK
        let text = "a".repeat(NOTE_LENGTH_WARNING_THRESHOLD);
        let note = ContextualNote::new(text, 1);
        assert!(note.validate().is_ok());
        assert!(!note.exceeds_warning_threshold());
    }

    #[test]
    fn test_contextual_note_validate_above_warning_below_error() {
        // Above warning (200) but at or below error (500) - validates OK, but exceeds warning
        let text = "a".repeat(NOTE_LENGTH_WARNING_THRESHOLD + 1);
        let note = ContextualNote::new(text, 1);
        assert!(note.validate().is_ok());
        assert!(note.exceeds_warning_threshold());
        assert!(!note.exceeds_error_threshold());
    }

    #[test]
    fn test_contextual_note_validate_at_error_threshold() {
        // Exactly at error threshold (500) - should validate OK
        let text = "a".repeat(NOTE_LENGTH_ERROR_THRESHOLD);
        let note = ContextualNote::new(text, 1);
        assert!(note.validate().is_ok());
        assert!(!note.exceeds_error_threshold());
    }

    #[test]
    fn test_contextual_note_validate_above_error_threshold() {
        // Above error threshold (501+) - should fail validation
        let text = "a".repeat(NOTE_LENGTH_ERROR_THRESHOLD + 1);
        let note = ContextualNote::new(text, 42);
        let err = note.validate().unwrap_err();
        assert!(matches!(err, LashError::Lint { .. }));
        assert!(note.exceeds_error_threshold());
    }

    #[test]
    fn test_contextual_note_truncated_text() {
        let note = ContextualNote::new("Hello, World!", 1);

        // No truncation needed
        assert_eq!(note.truncated_text(20), "Hello, World!");

        // Truncation needed
        assert_eq!(note.truncated_text(10), "Hello, ...");

        // Very short max
        assert_eq!(note.truncated_text(3), "...");
        assert_eq!(note.truncated_text(2), "...");
    }

    #[test]
    fn test_contextual_note_serialization_roundtrip() {
        let note = ContextualNote::new("Test note", 42);
        let json = serde_json::to_string(&note).unwrap();
        let deserialized: ContextualNote = serde_json::from_str(&json).unwrap();

        assert_eq!(note, deserialized);
    }

    #[test]
    fn test_contextual_note_serialization_format() {
        let note = ContextualNote::new("Test", 10);
        let json = serde_json::to_string(&note).unwrap();

        // Verify JSON structure
        assert!(json.contains("\"text\""));
        assert!(json.contains("\"line_number\""));
        assert!(json.contains("\"Test\""));
        assert!(json.contains("10"));
    }

    #[test]
    fn test_contextual_note_equality() {
        let note1 = ContextualNote::new("Same text", 1);
        let note2 = ContextualNote::new("Same text", 1);
        let note3 = ContextualNote::new("Same text", 2);
        let note4 = ContextualNote::new("Different text", 1);

        assert_eq!(note1, note2);
        assert_ne!(note1, note3); // Different line number
        assert_ne!(note1, note4); // Different text
    }

    #[test]
    fn test_contextual_note_clone() {
        let note = ContextualNote::new("Original", 42);
        let cloned = note.clone();

        assert_eq!(note, cloned);
    }

    #[test]
    fn test_contextual_note_thresholds() {
        // Verify threshold constants are correct
        assert_eq!(NOTE_LENGTH_WARNING_THRESHOLD, 200);
        assert_eq!(NOTE_LENGTH_ERROR_THRESHOLD, 500);
    }
}
